use super::{
    document::PluginNode,
    manifest::HOST_API,
    package::{self, Package},
    render::Scene,
    runtime::{Contributions, Runtime, dynamic},
};
use anyhow::{Context, Result, bail, ensure};
use rhai::Dynamic;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Settings {
    pub approved: BTreeMap<String, String>,
    pub enabled: BTreeSet<String>,
}
pub struct Entry {
    pub package: Package,
    pub runtime: Option<Runtime>,
    pub contributions: Contributions,
    pub error: Option<String>,
}
#[derive(Clone)]
pub enum Confirmation {
    Enable(Vec<String>),
    Disable(Vec<String>),
}
pub struct PluginManager {
    pub entries: BTreeMap<String, Entry>,
    pub selected: Option<(String, String)>,
    pub focus: Option<String>,
    pub notice: String,
    pub confirmation: Option<Confirmation>,
    pub settings: Settings,
    serial: u64,
    history_group: Option<String>,
    group_recorded: bool,
}
impl Default for PluginManager {
    fn default() -> Self {
        Self::load()
    }
}
impl PluginManager {
    #[cfg(test)]
    pub(super) fn for_tests(packages: Vec<Package>, activate: bool) -> Result<Self> {
        let mut m = Self {
            entries: BTreeMap::new(),
            selected: None,
            focus: None,
            notice: String::new(),
            confirmation: None,
            settings: Settings::default(),
            serial: 1,
            history_group: None,
            group_recorded: false,
        };
        for package in packages {
            let id = package.manifest.id.clone();
            m.entries.insert(
                id,
                Entry {
                    package,
                    runtime: None,
                    contributions: Contributions::default(),
                    error: None,
                },
            );
        }
        if activate {
            let ids: Vec<_> = m.entries.keys().cloned().collect();
            for id in ids {
                let plan = m.plan(&id)?;
                m.enable(&plan)?;
            }
        }
        Ok(m)
    }
    #[cfg(test)]
    pub(super) fn disable_for_test(&mut self, id: &str) {
        self.request_disable(id);
        if let Some(Confirmation::Disable(plan)) = self.confirmation.take() {
            self.disable(&plan);
        }
    }
    // TO DO TODO
    pub fn load() -> Self {
        let settings = super::platform::load_settings();
        let mut out = Self {
            entries: BTreeMap::new(),
            selected: None,
            focus: None,
            notice: String::new(),
            confirmation: None,
            settings,
            serial: 1,
            history_group: None,
            group_recorded: false,
        };
        out.scan();
        let enabled = out.settings.enabled.clone();
        for id in enabled {
            if let Ok(plan) = out.plan(&id) {
                if plan.iter().all(|p| {
                    out.settings.approved.get(p)
                        == out.entries.get(p).map(|e| &e.package.fingerprint)
                }) {
                    if let Err(e) = out.enable(&plan) {
                        out.notice = format!("Auto-activation failed: {e:#}");
                    }
                } else {
                    out.notice = "A plugin changed. Review and activate it again.".into();
                }
            }
        }
        out
    }
    fn scan(&mut self) {
        let (packages, mut errors) = package::discover();
        let mut duplicates = BTreeSet::new();
        for package in packages {
            let id = package.manifest.id.clone();
            if self.entries.contains_key(&id) {
                duplicates.insert(id);
                continue;
            }
            self.entries.insert(
                id,
                Entry {
                    package,
                    runtime: None,
                    contributions: Contributions::default(),
                    error: None,
                },
            );
        }
        for id in duplicates {
            self.entries.remove(&id);
            errors.push(format!("Duplicate plugin ID blocked: {id}"));
        }
        if !errors.is_empty() {
            self.notice = errors.join("\n");
        }
    }
    pub fn rescan(&mut self) {
        let old: Vec<_> = self.entries.keys().cloned().collect();
        self.disable(&old);
        self.entries.clear();
        self.confirmation = None;
        self.scan();
        self.notice = format!(
            "{} plugins found. Activate plugins after reviewing them. {}",
            self.entries.len(),
            self.notice
        );
    }
    pub fn active(&self, id: &str) -> bool {
        self.entries.get(id).is_some_and(|e| e.runtime.is_some())
    }
    pub fn wants_text_input(&self) -> bool {
        self.selected.as_ref().is_some_and(|(p, t)| {
            self.focus.as_ref() == Some(p)
                && self.entries.get(p).is_some_and(|e| {
                    e.contributions
                        .tools
                        .iter()
                        .any(|tool| tool.id == *t && tool.text_input)
                })
        })
    }
    pub fn plan(&self, id: &str) -> Result<Vec<String>> {
        fn visit(
            m: &PluginManager,
            id: &str,
            visiting: &mut BTreeSet<String>,
            done: &mut BTreeSet<String>,
            out: &mut Vec<String>,
        ) -> Result<()> {
            if done.contains(id) {
                return Ok(());
            }
            ensure!(visiting.insert(id.to_owned()), "dependency cycle at {id}");
            let e = m
                .entries
                .get(id)
                .with_context(|| format!("Plugin not installed: {id}"))?;
            ensure!(
                VersionReq::parse(&e.package.manifest.api)?.matches(&Version::parse(HOST_API)?),
                "incompatible host API: {id}"
            );
            for dep in &e.package.manifest.dependencies {
                let other = m.entries.get(&dep.id).with_context(|| {
                    format!(
                        "Missing dependency {}. Download: {}",
                        dep.id, dep.repository
                    )
                })?;
                ensure!(
                    VersionReq::parse(&dep.version)?
                        .matches(&Version::parse(&other.package.manifest.version)?),
                    "{} requires {} {}",
                    id,
                    dep.id,
                    dep.version
                );
                visit(m, &dep.id, visiting, done, out)?;
            }
            visiting.remove(id);
            done.insert(id.to_owned());
            if e.runtime.is_none() {
                out.push(id.to_owned());
            }
            Ok(())
        }
        let mut out = Vec::new();
        visit(
            self,
            id,
            &mut BTreeSet::new(),
            &mut BTreeSet::new(),
            &mut out,
        )?;
        Ok(out)
    }
    pub fn request_enable(&mut self, id: &str) {
        match self.plan(id) {
            Ok(plan) => self.confirmation = Some(Confirmation::Enable(plan)),
            Err(e) => self.notice = format!("Cannot activate: {e:#}"),
        }
    }
    pub fn request_disable(&mut self, id: &str) {
        let mut list = vec![id.to_owned()];
        loop {
            let more: Vec<_> = self
                .entries
                .iter()
                .filter(|(p, e)| {
                    e.runtime.is_some()
                        && !list.contains(p)
                        && e.package
                            .manifest
                            .dependencies
                            .iter()
                            .any(|d| list.contains(&d.id))
                })
                .map(|(p, _)| p.clone())
                .collect();
            if more.is_empty() {
                break;
            }
            list.extend(more);
        }
        self.confirmation = Some(Confirmation::Disable(list));
    }
    pub fn confirm(&mut self) {
        if let Some(c) = self.confirmation.take() {
            match c {
                Confirmation::Enable(plan) => match self.enable(&plan) {
                    Ok(()) => {
                        for id in plan {
                            self.settings
                                .approved
                                .insert(id.clone(), self.entries[&id].package.fingerprint.clone());
                            self.settings.enabled.insert(id);
                        }
                        self.notice = "Plugins activated".into();
                    }
                    Err(e) => self.notice = format!("Activation rolled back: {e:#}"),
                },
                Confirmation::Disable(plan) => {
                    self.disable(&plan);
                    for id in plan {
                        self.settings.enabled.remove(&id);
                    }
                    self.notice = "Plugins disabled. Document objects were preserved.".into();
                }
            }
            self.persist();
        }
    }
    fn enable(&mut self, plan: &[String]) -> Result<()> {
        let mut added = Vec::new();
        for id in plan {
            if self.active(id) {
                continue;
            }
            let result: Result<()> = (|| {
                let e = self.entries.get_mut(id).context("plugin disappeared")?;
                let mut runtime = Runtime::new(&e.package)?;
                let c = runtime.activate()?;
                e.runtime = Some(runtime);
                e.contributions = c;
                e.error = None;
                Ok(())
            })();
            if let Err(error) = result {
                self.disable(&added);
                return Err(error);
            }
            added.push(id.clone());
        }
        Ok(())
    }
    fn disable(&mut self, plan: &[String]) {
        self.history_group = None;
        self.group_recorded = false;
        for id in plan.iter().rev() {
            if let Some(e) = self.entries.get_mut(id) {
                if let Some(mut r) = e.runtime.take() {
                    if r.has("deactivate", 0) {
                        let _ = r.call("deactivate", vec![]);
                    }
                }
                e.contributions = Contributions::default();
            }
            if self.selected.as_ref().is_some_and(|(p, _)| p == id) {
                self.selected = None;
            }
            if self.focus.as_ref() == Some(id) {
                self.focus = None;
            }
        }
    }
    pub fn persist(&mut self) {
        if let Err(e) = super::platform::save_settings(&self.settings) {
            self.notice = format!("Could not save plugin settings: {e:#}");
        }
    }
    pub fn select_tool(&mut self, plugin: &str, tool: &str, canvas: &mut crate::Canvas) {
        if !self.entries.get(plugin).is_some_and(|e| {
            e.runtime.is_some() && e.contributions.tools.iter().any(|t| t.id == tool)
        }) {
            return;
        }
        self.blur(canvas);
        self.selected = Some((plugin.into(), tool.into()));
        self.dispatch(plugin, json!({"name":"tool.selected","tool":tool}), canvas);
    }
    pub fn blur(&mut self, canvas: &mut crate::Canvas) {
        if let Some((id, _)) = self.selected.clone() {
            self.dispatch(&id, json!({"name":"blur"}), canvas);
        }
        self.focus = None;
    }
    fn context(&mut self, id: &str, nodes: &[PluginNode]) -> Result<Dynamic> {
        let entry = self.entries.get(id).context("unknown plugin")?;
        let visible: Vec<_> = nodes
            .iter()
            .filter(|n| n.owner == id || entry.package.manifest.permits("document.read"))
            .collect();
        self.serial = self.serial.wrapping_add(1);
        let mut new_id = format!("{id}:{}", self.serial);
        while nodes.iter().any(|n| n.id == new_id) {
            self.serial = self.serial.wrapping_add(1);
            new_id = format!("{id}:{}", self.serial);
        }
        dynamic(
            &json!({"nodes":visible,"new_id":new_id,"tool":self.selected.as_ref().filter(|(p,_)|p==id).map_or("",|(_,t)|t.as_str()),"focused":self.focus.as_deref()==Some(id),"api":HOST_API}),
        )
    }
    // TODO
    pub fn dispatch(&mut self, id: &str, event: Value, canvas: &mut crate::Canvas) {
        if !self.active(id) {
            return;
        }
        let ending =
            (event["name"] == "pointer" && event["phase"] == "up") || event["name"] == "blur";
        if event["name"] == "pointer" && event["phase"] == "down" {
            self.history_group = Some(id.to_owned());
            self.group_recorded = false;
        }
        let states: Vec<_> = self
            .entries
            .iter()
            .filter_map(|(id, e)| e.runtime.as_ref().map(|r| (id.clone(), r.state.clone())))
            .collect();
        let focus = self.focus.clone();
        let mut nodes = canvas.plugin_nodes.clone();
        let mut budget = 64usize;
        let mut effects = Vec::new();
        let result: Result<()> = (|| {
            let ctx = self.context(id, &nodes)?;
            let cmds = self.invoke(id, "handle", vec![dynamic(&event)?, ctx])?;
            self.apply(
                id,
                &cmds,
                &mut nodes,
                &mut vec![id.to_owned()],
                &mut budget,
                &mut effects,
            )?;
            ensure!(
                nodes.len() <= 5000 && serde_json::to_vec(&nodes)?.len() <= 16 * 1024 * 1024,
                "document plugin data limit"
            );
            for text in &effects {
                super::platform::clipboard_write(text)?;
            }
            Ok(())
        })();
        if let Err(e) = result {
            for (id, state) in states {
                if let Some(r) = self.entries.get_mut(&id).and_then(|e| e.runtime.as_mut()) {
                    r.state = state;
                }
            }
            self.focus = focus;
            self.notice = format!("Plugin {id}: transaction rejected: {e:#}");
            if ending {
                self.history_group = None;
                self.group_recorded = false;
            }
            return;
        }
        if nodes != canvas.plugin_nodes {
            if self.history_group.as_deref() != Some(id) || !self.group_recorded {
                canvas.record_history();
            }
            if self.history_group.as_deref() == Some(id) {
                self.group_recorded = true;
            }
            canvas.plugin_nodes = nodes;
        }
        if ending {
            self.history_group = None;
            self.group_recorded = false;
        }
    }
    fn invoke(&mut self, id: &str, name: &str, args: Vec<Dynamic>) -> Result<Value> {
        self.entries
            .get_mut(id)
            .and_then(|e| e.runtime.as_mut())
            .context("plugin is not active")?
            .call(name, args)
            .with_context(|| format!("{id}::{name}"))
    }
    fn permission(&self, id: &str, p: &str) -> Result<()> {
        ensure!(
            self.entries
                .get(id)
                .is_some_and(|e| e.package.manifest.permits(p)),
            "{id} lacks permission {p}"
        );
        Ok(())
    }
    // TODO
    fn apply(
        &mut self,
        id: &str,
        value: &Value,
        nodes: &mut Vec<PluginNode>,
        chain: &mut Vec<String>,
        budget: &mut usize,
        effects: &mut Vec<String>,
    ) -> Result<()> {
        let cmds = value
            .as_array()
            .context("callback must return an array of commands")?;
        ensure!(cmds.len() <= 128, "too many commands");
        for cmd in cmds {
            ensure!(*budget > 0, "cross-plugin/command budget exceeded");
            *budget -= 1;
            let kind = cmd["kind"].as_str().context("missing command kind")?;
            match kind {
                "node.create" | "node.update" => {
                    self.permission(id, "document.write")?;
                    let mut node: PluginNode = serde_json::from_value(cmd["node"].clone())?;
                    if kind == "node.create" {
                        node.owner = id.into();
                        ensure!(
                            node.id.starts_with(&format!("{id}:")),
                            "node ID must be namespaced"
                        );
                    }
                    ensure!(node.owner == id, "cannot modify another plugin's object");
                    ensure!(
                        self.entries[id]
                            .contributions
                            .object_types
                            .contains(&node.object_type),
                        "object type not registered"
                    );
                    node.validate()?;
                    if kind == "node.create" {
                        ensure!(
                            !nodes.iter().any(|n| n.id == node.id),
                            "duplicate object ID"
                        );
                        nodes.push(node);
                    } else {
                        let idx = nodes
                            .iter()
                            .position(|n| n.id == node.id && n.owner == id)
                            .context("object not found or not owned")?;
                        nodes[idx] = node;
                    }
                }
                "node.delete" => {
                    self.permission(id, "document.write")?;
                    let key = cmd["id"].as_str().context("missing node ID")?;
                    let idx = nodes
                        .iter()
                        .position(|n| n.id == key && n.owner == id)
                        .context("object not found or not owned")?;
                    nodes.remove(idx);
                }
                "input.focus" => {
                    if self.selected.as_ref().is_some_and(|(p, _)| p == id) {
                        self.focus = Some(id.into());
                    }
                }
                "input.blur" => {
                    if self.focus.as_deref() == Some(id) {
                        self.focus = None;
                    }
                }
                "call" => {
                    self.permission(id, "plugins.call")?;
                    let target = cmd["plugin"].as_str().context("missing target")?;
                    let service = cmd["service"].as_str().context("missing service")?;
                    ensure!(
                        self.entries[id]
                            .package
                            .manifest
                            .dependencies
                            .iter()
                            .any(|d| d.id == target),
                        "service target is not a declared dependency"
                    );
                    ensure!(
                        self.active(target)
                            && self.entries[target]
                                .contributions
                                .services
                                .iter()
                                .any(|s| s == service),
                        "service unavailable"
                    );
                    ensure!(
                        chain.len() < 8 && !chain.iter().any(|p| p == target),
                        "service call cycle/depth limit"
                    );
                    let ctx = self.context(target, nodes)?;
                    let reply = self.invoke(
                        target,
                        "service",
                        vec![dynamic(&service)?, dynamic(&cmd["args"])?, ctx],
                    )?;
                    chain.push(target.into());
                    self.apply(target, &reply["commands"], nodes, chain, budget, effects)?;
                    chain.pop();
                    if let Some(token) = cmd.get("token") {
                        let ctx = self.context(id, nodes)?;
                        let event = json!({"name":"service.result","token":token,"value":reply["value"],"plugin":target,"service":service});
                        let commands = self.invoke(id, "handle", vec![dynamic(&event)?, ctx])?;
                        self.apply(id, &commands, nodes, chain, budget, effects)?;
                    }
                }
                "clipboard.write" => {
                    self.permission(id, "clipboard.write")?;
                    let text = cmd["text"].as_str().context("missing clipboard text")?;
                    ensure!(text.len() <= 262144, "clipboard output too large");
                    effects.push(text.into());
                }
                "clipboard.read" => {
                    self.permission(id, "clipboard.read")?;
                    let read = super::platform::clipboard_read();
                    let ev = match read {
                        Ok(text) => json!({"name":"clipboard","ok":true,"text":text}),
                        Err(e) => {
                            json!({"name":"clipboard","ok":false,"text":"","error":e.to_string()})
                        }
                    };
                    let ctx = self.context(id, nodes)?;
                    let out = self.invoke(id, "handle", vec![dynamic(&ev)?, ctx])?;
                    self.apply(id, &out, nodes, chain, budget, effects)?;
                }
                _ => bail!("unknown host operation: {kind}"),
            }
        }
        Ok(())
    }
    pub fn document_changed(&mut self, canvas: &mut crate::Canvas) {
        self.history_group = None;
        self.group_recorded = false;
        let ids: Vec<_> = self
            .entries
            .iter()
            .filter(|(_, e)| e.runtime.is_some())
            .map(|(id, _)| id.clone())
            .collect();
        for id in ids {
            if let Ok(ctx) = self.context(&id, &canvas.plugin_nodes) {
                let _ = self.invoke(
                    &id,
                    "handle",
                    vec![
                        dynamic(&json!({"name":"document.changed"})).unwrap_or(Dynamic::UNIT),
                        ctx,
                    ],
                );
            }
        }
    }
    // Why i writing so big functions... Ok T O D O (for IDE - TODO)
    pub fn project(&mut self, nodes: &[PluginNode], overlay: bool, zoom: f32) -> Scene {
        let mut scene = Scene::default();
        let mut errors = Vec::new();
        let started = web_time::Instant::now();
        for node in nodes {
            let rendered: Result<()> = (|| {
                ensure!(
                    started.elapsed().as_millis() <= 80,
                    "frame script budget exceeded"
                );
                let ctx = dynamic(
                    &json!({"zoom":zoom,"api":HOST_API,"focused":self.focus.as_deref()==Some(node.owner.as_str())}),
                )?;
                let e = self
                    .entries
                    .get_mut(&node.owner)
                    .context("plugin missing")?;
                ensure!(
                    e.contributions.object_types.contains(&node.object_type),
                    "object type unavailable"
                );
                let r = e.runtime.as_mut().context("plugin disabled")?;
                let out = r.pure_call("render", vec![dynamic(node)?, ctx])?;
                scene.extend_primitives(&out)?;
                Ok(())
            })();
            if let Err(e) = rendered {
                scene.placeholder(node);
                if self.active(&node.owner) {
                    errors.push(format!("{}: {e:#}", node.owner));
                }
            }
            scene.shapes.push(crate::ShapeInstance::rect(
                node.x,
                node.y - node.height,
                node.width,
                node.height,
                crate::Color::TRANSPARENT,
            ));
            if scene.len() > 65536 {
                self.notice = "Scene primitive budget exceeded".into();
                break;
            }
        }
        if overlay {
            if let Some((id, _)) = self.selected.clone() {
                if self.active(&id) {
                    let context = json!({"nodes":nodes.iter().filter(|n|n.owner==id).collect::<Vec<_>>(),"focused":self.focus.as_deref()==Some(id.as_str()),"zoom":zoom});
                    if let Some(r) = self.entries.get_mut(&id).and_then(|e| e.runtime.as_mut()) {
                        if r.has("overlay", 1) {
                            match r.pure_call(
                                "overlay",
                                vec![dynamic(&context).unwrap_or(Dynamic::UNIT)],
                            ) {
                                Ok(v) => {
                                    if let Err(e) = scene.extend_primitives(&v) {
                                        errors.push(e.to_string());
                                    }
                                }
                                Err(e) => errors.push(e.to_string()),
                            }
                        }
                    }
                }
            }
        }
        if let Some(error) = errors.first() {
            self.notice = format!("Render: {error}");
        }
        scene
    }
}
