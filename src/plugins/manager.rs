use super::{
    document::PluginNode,
    manifest::HOST_API,
    package::{self, Package},
    render::Scene,
    runtime::{Contributions, Runtime, json_value},
};
use anyhow::{Context, Result, bail, ensure};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

const EVENT_COMMAND_BUDGET: usize = 64;
const EVENT_TIME_BUDGET_MS: u128 = 350;
const FRAME_TIME_BUDGET_MS: u128 = 80;
const MAX_COMMANDS_PER_CALLBACK: usize = 128;
const MAX_SERVICE_DEPTH: usize = 8;
const MAX_DOCUMENT_NODES: usize = 5_000;
const MAX_DOCUMENT_BYTES: usize = 16 * 1024 * 1024;
const MAX_CLIPBOARD_BYTES: usize = 262_144;
const MAX_SCENE_PRIMITIVES: usize = 65_536;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Settings {
    pub approved: BTreeMap<String, String>,
    pub enabled: BTreeSet<String>,

    #[serde(default)]
    pub appearance: crate::theme::Appearance,
}

pub struct Entry {
    pub package: Package,
    pub runtime: Option<Runtime>,
    pub contributions: Contributions,
    pub error: Option<String>,
}

impl Entry {
    pub(crate) fn new(package: Package) -> Self {
        Self {
            package,
            runtime: None,
            contributions: Contributions::default(),
            error: None,
        }
    }
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum WriteKind {
    Create,
    Update,
}

struct Transaction {
    nodes: Vec<PluginNode>,
    chain: Vec<String>,
    budget: usize,
    effects: Vec<String>,
    /// Отложенный `file.pick`: (accept, token). Диалог открывается только если транзакция прошла.
    pick: Option<(String, Value)>,
    started: web_time::Instant,
}

impl Transaction {
    fn new(id: &str, nodes: Vec<PluginNode>) -> Self {
        Self {
            nodes,
            chain: vec![id.to_owned()],
            budget: EVENT_COMMAND_BUDGET,
            effects: Vec::new(),
            pick: None,
            started: web_time::Instant::now(),
        }
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::load()
    }
}

impl PluginManager {
    pub(crate) fn with_settings(settings: Settings) -> Self {
        Self {
            entries: BTreeMap::new(),
            selected: None,
            focus: None,
            notice: String::new(),
            confirmation: None,
            settings,
            serial: 1,
            history_group: None,
            group_recorded: false,
        }
    }

    #[must_use]
    pub fn load() -> Self {
        let mut manager = Self::with_settings(super::platform::load_settings());
        for bytes in super::platform::load_user_fonts() {
            let _ = crate::graphics::fonts::register(bytes);
        }
        manager.scan();
        manager.restore_enabled();
        manager
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

    #[must_use]
    pub fn active(&self, id: &str) -> bool {
        self.entries.get(id).is_some_and(|e| e.runtime.is_some())
    }

    #[must_use]
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
        let mut out = Vec::new();
        self.visit_plan(id, &mut BTreeSet::new(), &mut BTreeSet::new(), &mut out)?;
        Ok(out)
    }

    fn visit_plan(
        &self,
        id: &str,
        visiting: &mut BTreeSet<String>,
        done: &mut BTreeSet<String>,
        out: &mut Vec<String>,
    ) -> Result<()> {
        if done.contains(id) {
            return Ok(());
        }
        ensure!(visiting.insert(id.to_owned()), "dependency cycle at {id}");

        let e = self
            .entries
            .get(id)
            .with_context(|| format!("Plugin not installed: {id}"))?;
        ensure!(
            VersionReq::parse(&e.package.manifest.api)?.matches(&Version::parse(HOST_API)?),
            "incompatible host API: {id}"
        );

        for dep in &e.package.manifest.dependencies {
            let other = self.entries.get(&dep.id).with_context(|| {
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
            self.visit_plan(&dep.id, visiting, done, out)?;
        }

        visiting.remove(id);
        done.insert(id.to_owned());
        if e.runtime.is_none() {
            out.push(id.to_owned());
        }
        Ok(())
    }

    pub fn request_enable(&mut self, id: &str) {
        match self.plan(id) {
            Ok(plan) => self.confirmation = Some(Confirmation::Enable(plan)),
            Err(e) => self.notice = format!("Cannot activate: {e:#}"),
        }
    }

    pub fn request_disable(&mut self, id: &str) {
        let list = self.dependents_closure(id);
        self.confirmation = Some(Confirmation::Disable(list));
    }

    fn dependents_closure(&self, id: &str) -> Vec<String> {
        let mut list = vec![id.to_owned()];
        loop {
            let more: Vec<String> = self
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
                return list;
            }
            list.extend(more);
        }
    }

    pub fn confirm(&mut self) {
        let Some(confirmation) = self.confirmation.take() else {
            return;
        };
        match confirmation {
            Confirmation::Enable(plan) => self.finish_enable(plan),
            Confirmation::Disable(plan) => self.finish_disable(&plan),
        }
        self.persist();
    }

    fn finish_enable(&mut self, plan: Vec<String>) {
        match self.enable(&plan) {
            Ok(()) => {
                for id in plan {
                    let fingerprint = self.entries[&id].package.fingerprint.clone();
                    self.settings.approved.insert(id.clone(), fingerprint);
                    self.settings.enabled.insert(id);
                }
                self.notice = "Plugins activated".into();
            }
            Err(e) => self.notice = format!("Activation rolled back: {e:#}"),
        }
    }

    fn finish_disable(&mut self, plan: &[String]) {
        self.disable(plan);
        for id in plan {
            self.settings.enabled.remove(id);
        }
        self.notice = "Plugins disabled. Document objects were preserved.".into();
    }

    pub fn persist(&mut self) {
        if let Err(e) = super::platform::save_settings(&self.settings) {
            self.notice = format!("Could not save plugin settings: {e:#}");
        }
    }

    pub fn select_tool(&mut self, plugin: &str, tool: &str, canvas: &mut crate::Canvas) {
        if !self.has_tool(plugin, tool) {
            return;
        }
        self.blur(canvas);
        self.selected = Some((plugin.into(), tool.into()));
        self.dispatch(
            plugin,
            &json!({"name": "tool.selected", "tool": tool}),
            canvas,
        );
    }

    fn has_tool(&self, plugin: &str, tool: &str) -> bool {
        self.entries.get(plugin).is_some_and(|e| {
            e.runtime.is_some() && e.contributions.tools.iter().any(|t| t.id == tool)
        })
    }

    pub fn blur(&mut self, canvas: &mut crate::Canvas) {
        if let Some((id, _)) = self.selected.clone() {
            self.dispatch(&id, &json!({"name": "blur"}), canvas);
        }
        self.focus = None;
    }

    pub fn dispatch(&mut self, id: &str, event: &Value, canvas: &mut crate::Canvas) {
        if !self.active(id) {
            return;
        }
        let ending = is_gesture_end(event);
        if is_pointer(event, "down") {
            self.begin_history_group(id);
        }

        let undo = self.snapshot();
        let mut tx = Transaction::new(id, canvas.plugin_nodes.clone());
        match self.run_handler(id, event, &mut tx) {
            Ok(()) => self.commit_nodes(id, canvas, tx.nodes),
            Err(e) => {
                undo(&mut *self);
                self.notice = format!("Plugin {id}: transaction rejected: {e:#}");
            }
        }

        if ending {
            self.end_history_group();
        }
    }

    fn snapshot(&self) -> impl FnOnce(&mut Self) + use<> {
        let states: Vec<_> = self
            .entries
            .iter()
            .filter_map(|(pid, e)| e.runtime.as_ref().map(|r| (pid.clone(), r.state.clone())))
            .collect();
        let focus = self.focus.clone();
        move |manager: &mut Self| {
            for (pid, state) in states {
                if let Some(r) = manager
                    .entries
                    .get_mut(&pid)
                    .and_then(|e| e.runtime.as_mut())
                {
                    r.state = state;
                }
            }
            manager.focus = focus;
        }
    }

    fn run_handler(&mut self, id: &str, event: &Value, tx: &mut Transaction) -> Result<()> {
        let ctx = self.context(id, tx.nodes.as_slice())?;
        let cmds = self.invoke(id, "handle", &[json_value(event)?, ctx])?;
        self.apply(id, &cmds, tx)?;

        ensure!(
            tx.started.elapsed().as_millis() <= EVENT_TIME_BUDGET_MS,
            "event time budget exceeded"
        );
        check_document_limits(tx.nodes.as_slice())?;
        for text in &tx.effects {
            super::platform::clipboard_write(text)?;
        }
        if let Some((accept, token)) = tx.pick.take() {
            super::platform::request_plugin_file(id, &accept, &token);
        }
        Ok(())
    }

    pub fn deliver_file(&mut self, file: super::platform::PluginFile, canvas: &mut crate::Canvas) {
        let super::platform::PluginFile {
            plugin,
            accept,
            token,
            name,
            bytes,
        } = file;
        let result = match accept.as_str() {
            "font" => crate::graphics::fonts::register(bytes.clone()).inspect(|_| {
                if let Err(e) = super::platform::store_user_font(&bytes) {
                    self.notice = format!("Font is available until restart: {e:#}");
                }
            }),
            other => Err(anyhow::anyhow!("unsupported file kind: {other}")),
        };
        let event = match result {
            Ok(family) => json!({
                "name": "file.picked", "ok": true, "kind": accept,
                "family": family, "file": name, "token": token,
            }),
            Err(e) => json!({
                "name": "file.picked", "ok": false, "kind": accept,
                "file": name, "token": token, "error": format!("{e:#}"),
            }),
        };
        self.dispatch(&plugin, &event, canvas);
    }

    fn commit_nodes(&mut self, id: &str, canvas: &mut crate::Canvas, nodes: Vec<PluginNode>) {
        if nodes == canvas.plugin_nodes {
            return;
        }
        let grouped = self.history_group.as_deref() == Some(id);
        if !grouped || !self.group_recorded {
            canvas.record_history();
        }
        if grouped {
            self.group_recorded = true;
        }
        canvas.plugin_nodes = nodes;
    }

    pub fn document_changed(&mut self, canvas: &mut crate::Canvas) {
        self.end_history_group();
        for id in self.active_ids() {
            if let Ok(ctx) = self.context(&id, &canvas.plugin_nodes) {
                let event = json_value(&json!({"name": "document.changed"})).unwrap_or(Value::Null);
                let _ = self.invoke(&id, "handle", &[event, ctx]);
            }
        }
    }

    pub fn project(&mut self, nodes: &[PluginNode], overlay: bool, zoom: f32) -> Scene {
        let mut scene = Scene::default();
        let mut errors = Vec::new();
        let started = web_time::Instant::now();

        for node in nodes {
            if let Err(e) = self.render_node(node, zoom, &started, &mut scene) {
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
            if scene.len() > MAX_SCENE_PRIMITIVES {
                self.notice = "Scene primitive budget exceeded".into();
                break;
            }
        }

        if overlay && let Err(e) = self.render_overlay(nodes, zoom, &mut scene) {
            errors.push(e.to_string());
        }

        if let Some(error) = errors.first() {
            self.notice = format!("Render: {error}");
        }
        scene
    }

    fn render_node(
        &mut self,
        node: &PluginNode,
        zoom: f32,
        started: &web_time::Instant,
        scene: &mut Scene,
    ) -> Result<()> {
        ensure!(
            started.elapsed().as_millis() <= FRAME_TIME_BUDGET_MS,
            "frame script budget exceeded"
        );
        let focused = self.focus.as_deref() == Some(node.owner.as_str());
        let ctx = json_value(&json!({"zoom": zoom, "api": HOST_API, "focused": focused}))?;

        let e = self
            .entries
            .get_mut(&node.owner)
            .context("plugin missing")?;
        ensure!(
            e.contributions.object_types.contains(&node.object_type),
            "object type unavailable"
        );
        let runtime = e.runtime.as_mut().context("plugin disabled")?;
        let out = runtime.pure_call("render", &[json_value(node)?, ctx])?;
        scene.extend_primitives(&out)?;
        Ok(())
    }

    fn render_overlay(&mut self, nodes: &[PluginNode], zoom: f32, scene: &mut Scene) -> Result<()> {
        let Some((id, _)) = self.selected.clone() else {
            return Ok(());
        };
        if !self.active(&id) {
            return Ok(());
        }
        let owned: Vec<_> = nodes.iter().filter(|n| n.owner == id).collect();
        let context = json!({
            "nodes": owned,
            "focused": self.focus.as_deref() == Some(id.as_str()),
            "zoom": zoom,
        });
        let Some(runtime) = self.entries.get_mut(&id).and_then(|e| e.runtime.as_mut()) else {
            return Ok(());
        };
        if !runtime.has("overlay") {
            return Ok(());
        }
        let out = runtime.pure_call("overlay", &[json_value(&context).unwrap_or(Value::Null)])?;
        scene.extend_primitives(&out)?;
        Ok(())
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
            self.entries.insert(id, Entry::new(package));
        }
        for id in duplicates {
            self.entries.remove(&id);
            errors.push(format!("Duplicate plugin ID blocked: {id}"));
        }
        if !errors.is_empty() {
            self.notice = errors.join("\n");
        }
    }

    fn restore_enabled(&mut self) {
        let enabled = self.settings.enabled.clone();
        for id in enabled {
            let Ok(plan) = self.plan(&id) else {
                continue;
            };
            if !self.fingerprints_approved(&plan) {
                self.notice = "A plugin changed. Review and activate it again.".into();
            } else if let Err(e) = self.enable(&plan) {
                self.notice = format!("Auto-activation failed: {e:#}");
            }
        }
    }

    fn fingerprints_approved(&self, plan: &[String]) -> bool {
        plan.iter().all(|p| {
            self.settings.approved.get(p) == self.entries.get(p).map(|e| &e.package.fingerprint)
        })
    }

    fn apply(&mut self, id: &str, value: &Value, tx: &mut Transaction) -> Result<()> {
        let cmds = value
            .as_array()
            .context("callback must return an array of commands")?;
        ensure!(cmds.len() <= MAX_COMMANDS_PER_CALLBACK, "too many commands");

        for cmd in cmds {
            ensure!(
                tx.started.elapsed().as_millis() <= EVENT_TIME_BUDGET_MS,
                "event time budget exceeded"
            );
            ensure!(tx.budget > 0, "cross-plugin/command budget exceeded");
            tx.budget -= 1;
            self.apply_command(id, cmd, tx)?;
        }
        Ok(())
    }

    fn apply_command(&mut self, id: &str, cmd: &Value, tx: &mut Transaction) -> Result<()> {
        let kind = cmd["kind"].as_str().context("missing command kind")?;
        match kind {
            "node.create" => self.write_node(id, WriteKind::Create, cmd, &mut tx.nodes),
            "node.update" => self.write_node(id, WriteKind::Update, cmd, &mut tx.nodes),
            "node.delete" => self.delete_node(id, cmd, &mut tx.nodes),
            "input.focus" => {
                if self.selected.as_ref().is_some_and(|(p, _)| p == id) {
                    self.focus = Some(id.into());
                }
                Ok(())
            }
            "input.blur" => {
                if self.focus.as_deref() == Some(id) {
                    self.focus = None;
                }
                Ok(())
            }
            "call" => self.call_service(id, cmd, tx),
            "clipboard.write" => self.queue_clipboard_write(id, cmd, &mut tx.effects),
            "clipboard.read" => self.read_clipboard(id, tx),
            "file.pick" => {
                self.permission(id, "files.pick")?;
                let accept = cmd["accept"].as_str().context("missing file kind")?;
                ensure!(
                    super::platform::PICKABLE
                        .iter()
                        .any(|(kind, _, _)| *kind == accept),
                    "cannot pick files of kind {accept}"
                );
                ensure!(tx.pick.is_none(), "only one file.pick per event");
                tx.pick = Some((accept.to_owned(), cmd["token"].clone()));
                Ok(())
            }
            _ => bail!("unknown host operation: {kind}"),
        }
    }

    fn write_node(
        &self,
        id: &str,
        kind: WriteKind,
        cmd: &Value,
        nodes: &mut Vec<PluginNode>,
    ) -> Result<()> {
        self.permission(id, "document.write")?;
        let mut node: PluginNode = serde_json::from_value(cmd["node"].clone())?;
        if kind == WriteKind::Create {
            node.owner = id.into();
            ensure!(
                node.id.starts_with(&format!("{id}:")),
                "node ID must be namespaced"
            );
        }
        ensure!(node.owner == id, "cannot modify another plugin's object");
        ensure!(
            self.entries
                .get(id)
                .is_some_and(|e| e.contributions.object_types.contains(&node.object_type)),
            "object type not registered"
        );
        node.validate()?;

        match kind {
            WriteKind::Create => {
                ensure!(
                    !nodes.iter().any(|n| n.id == node.id),
                    "duplicate object ID"
                );
                nodes.push(node);
            }
            WriteKind::Update => {
                let idx = nodes
                    .iter()
                    .position(|n| n.id == node.id && n.owner == id)
                    .context("object not found or not owned")?;
                nodes[idx] = node;
            }
        }
        Ok(())
    }

    fn delete_node(&self, id: &str, cmd: &Value, nodes: &mut Vec<PluginNode>) -> Result<()> {
        self.permission(id, "document.write")?;
        let key = cmd["id"].as_str().context("missing node ID")?;
        let idx = nodes
            .iter()
            .position(|n| n.id == key && n.owner == id)
            .context("object not found or not owned")?;
        nodes.remove(idx);
        Ok(())
    }

    fn call_service(&mut self, id: &str, cmd: &Value, tx: &mut Transaction) -> Result<()> {
        self.permission(id, "plugins.call")?;
        let target = cmd["plugin"].as_str().context("missing target")?;
        let service = cmd["service"].as_str().context("missing service")?;
        self.check_service_access(id, target, service)?;
        ensure!(
            tx.chain.len() < MAX_SERVICE_DEPTH && !tx.chain.iter().any(|p| p == target),
            "service call cycle/depth limit"
        );

        let ctx = self.context(target, tx.nodes.as_slice())?;
        let reply = self.invoke(
            target,
            "service",
            &[json_value(&service)?, json_value(&cmd["args"])?, ctx],
        )?;

        tx.chain.push(target.into());
        self.apply(target, &reply["commands"], tx)?;
        tx.chain.pop();

        if let Some(token) = cmd.get("token") {
            let ctx = self.context(id, tx.nodes.as_slice())?;
            let event = json!({
                "name": "service.result",
                "token": token,
                "value": reply["value"],
                "plugin": target,
                "service": service,
            });
            let commands = self.invoke(id, "handle", &[json_value(&event)?, ctx])?;
            self.apply(id, &commands, tx)?;
        }
        Ok(())
    }

    fn check_service_access(&self, id: &str, target: &str, service: &str) -> Result<()> {
        let declared = self.entries.get(id).is_some_and(|e| {
            e.package
                .manifest
                .dependencies
                .iter()
                .any(|d| d.id == target)
        });
        ensure!(declared, "service target is not a declared dependency");

        let available = self.active(target)
            && self
                .entries
                .get(target)
                .is_some_and(|e| e.contributions.services.iter().any(|s| s == service));
        ensure!(available, "service unavailable");
        Ok(())
    }

    fn queue_clipboard_write(
        &self,
        id: &str,
        cmd: &Value,
        effects: &mut Vec<String>,
    ) -> Result<()> {
        self.permission(id, "clipboard.write")?;
        let text = cmd["text"].as_str().context("missing clipboard text")?;
        ensure!(
            text.len() <= MAX_CLIPBOARD_BYTES,
            "clipboard output too large"
        );
        effects.push(text.into());
        Ok(())
    }

    fn read_clipboard(&mut self, id: &str, tx: &mut Transaction) -> Result<()> {
        self.permission(id, "clipboard.read")?;
        let event = match super::platform::clipboard_read() {
            Ok(text) => json!({"name": "clipboard", "ok": true, "text": text}),
            Err(e) => json!({
                "name": "clipboard",
                "ok": false,
                "text": "",
                "error": e.to_string(),
            }),
        };
        let ctx = self.context(id, tx.nodes.as_slice())?;
        let out = self.invoke(id, "handle", &[json_value(&event)?, ctx])?;
        self.apply(id, &out, tx)
    }

    fn enable(&mut self, plan: &[String]) -> Result<()> {
        let mut added = Vec::new();
        for id in plan {
            if self.active(id) {
                continue;
            }
            if let Err(error) = self.start_plugin(id) {
                self.disable(&added);
                return Err(error);
            }
            added.push(id.clone());
        }
        Ok(())
    }

    pub(crate) fn start_plugin(&mut self, id: &str) -> Result<()> {
        let e = self.entries.get_mut(id).context("plugin disappeared")?;
        let mut runtime = Runtime::new(&e.package)?;
        let contributions = runtime.activate()?;
        e.runtime = Some(runtime);
        e.contributions = contributions;
        e.error = None;
        Ok(())
    }

    fn disable(&mut self, plan: &[String]) {
        self.end_history_group();
        for id in plan.iter().rev() {
            self.shut_down(id);
            if self.selected.as_ref().is_some_and(|(p, _)| p == id) {
                self.selected = None;
            }
            if self.focus.as_ref() == Some(id) {
                self.focus = None;
            }
        }
    }

    fn shut_down(&mut self, id: &str) {
        if let Some(e) = self.entries.get_mut(id) {
            if let Some(mut runtime) = e.runtime.take()
                && runtime.has("deactivate")
            {
                let _ = runtime.call("deactivate", &[]);
            }
            e.contributions = Contributions::default();
        }
    }

    fn invoke(&mut self, id: &str, name: &str, args: &[Value]) -> Result<Value> {
        self.entries
            .get_mut(id)
            .and_then(|e| e.runtime.as_mut())
            .context("plugin is not active")?
            .call(name, args)
            .with_context(|| format!("{id}::{name}"))
    }

    fn permission(&self, id: &str, permission: &str) -> Result<()> {
        ensure!(
            self.entries
                .get(id)
                .is_some_and(|e| e.package.manifest.permits(permission)),
            "{id} lacks permission {permission}"
        );
        Ok(())
    }

    fn context(&mut self, id: &str, nodes: &[PluginNode]) -> Result<Value> {
        let entry = self.entries.get(id).context("unknown plugin")?;
        let can_read_all = entry.package.manifest.permits("document.read");
        let visible: Vec<_> = nodes
            .iter()
            .filter(|n| n.owner == id || can_read_all)
            .collect();

        let new_id = self.next_object_id(id, nodes);
        let tool = self
            .selected
            .as_ref()
            .filter(|(p, _)| p == id)
            .map_or("", |(_, t)| t.as_str());

        json_value(&json!({
            "nodes": visible,
            "new_id": new_id,
            "tool": tool,
            "focused": self.focus.as_deref() == Some(id),
            "api": HOST_API,
        }))
    }

    fn next_object_id(&mut self, id: &str, nodes: &[PluginNode]) -> String {
        loop {
            self.serial = self.serial.wrapping_add(1);
            let candidate = format!("{id}:{}", self.serial);
            if !nodes.iter().any(|n| n.id == candidate) {
                return candidate;
            }
        }
    }

    fn active_ids(&self) -> Vec<String> {
        self.entries
            .iter()
            .filter(|(_, e)| e.runtime.is_some())
            .map(|(id, _)| id.clone())
            .collect()
    }

    fn begin_history_group(&mut self, id: &str) {
        self.history_group = Some(id.to_owned());
        self.group_recorded = false;
    }

    fn end_history_group(&mut self) {
        self.history_group = None;
        self.group_recorded = false;
    }
}

fn check_document_limits(nodes: &[PluginNode]) -> Result<()> {
    ensure!(
        nodes.len() <= MAX_DOCUMENT_NODES && serde_json::to_vec(nodes)?.len() <= MAX_DOCUMENT_BYTES,
        "document plugin data limit"
    );
    Ok(())
}

fn is_pointer(event: &Value, phase: &str) -> bool {
    event["name"] == "pointer" && event["phase"] == phase
}

fn is_gesture_end(event: &Value) -> bool {
    is_pointer(event, "up") || event["name"] == "blur"
}
