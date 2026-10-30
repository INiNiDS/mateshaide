#[cfg(not(target_arch = "wasm32"))]
use super::lua_native::Backend;
#[cfg(target_arch = "wasm32")]
use super::lua_web::Backend;
use super::{manifest::valid_id, package::Package};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeSet;

pub(super) const JSON_SOURCE: &str = include_str!("json.lua");
pub(super) const BOOTSTRAP_SOURCE: &str = include_str!("bootstrap.lua");

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tool {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub text_input: bool,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Command {
    pub id: String,
    pub label: String,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contributions {
    #[serde(default)]
    pub tools: Vec<Tool>,
    #[serde(default)]
    pub services: Vec<String>,
    #[serde(default)]
    pub object_types: Vec<String>,
    #[serde(default)]
    pub commands: Vec<Command>,
}

pub struct Runtime {
    backend: Backend,
    sources: String,
    entry: String,
    callbacks: BTreeSet<String>,
    pub state: Value,
}
impl Runtime {
    pub fn new(package: &Package) -> Result<Self> {
        Ok(Self {
            backend: Backend::new()?,
            sources: serde_json::to_string(&package.scripts()?)?,
            entry: package.manifest.entry.clone(),
            callbacks: BTreeSet::new(),
            state: json!({}),
        })
    }
    pub fn has(&self, name: &str) -> bool {
        self.callbacks.contains(name)
    }
    pub fn call(&mut self, name: &str, args: Vec<Value>) -> Result<Value> {
        let state = serde_json::to_string(&self.state)?;
        ensure!(state.len() <= 1048576, "plugin state limit");
        let args = serde_json::to_string(&args)?;
        ensure!(args.len() <= 16777216, "callback input limit");
        let out = self
            .backend
            .invoke(&self.sources, &self.entry, &state, name, &args)?;
        ensure!(out.len() <= 1048576, "callback output limit");
        let out: Value = serde_json::from_str(&out)?;
        let next = out.get("state").context("missing plugin state")?.clone();
        ensure!(next.is_object(), "plugin state must remain a JSON object");
        let names = out["callbacks"]
            .as_array()
            .context("missing callback metadata")?;
        self.callbacks = names
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect();
        self.state = next;
        Ok(out.get("result").cloned().unwrap_or(Value::Null))
    }
    pub fn pure_call(&mut self, name: &str, args: Vec<Value>) -> Result<Value> {
        let previous = self.state.clone();
        let out = self.call(name, args);
        self.state = previous;
        out
    }
    pub fn activate(&mut self) -> Result<Contributions> {
        let c: Contributions = serde_json::from_value(self.call("activate", vec![])?)?;
        ensure!(
            c.tools.len() <= 24
                && c.services.len() <= 32
                && c.object_types.len() <= 32
                && c.commands.len() <= 24,
            "contribution limit"
        );
        for names in [
            c.tools.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(),
            c.commands.iter().map(|t| t.id.as_str()).collect(),
            c.services.iter().map(String::as_str).collect(),
            c.object_types.iter().map(String::as_str).collect(),
        ] {
            let mut seen = BTreeSet::new();
            for id in names {
                ensure!(
                    valid_id(id) && seen.insert(id),
                    "invalid or duplicate contribution"
                );
            }
        }
        for label in c
            .tools
            .iter()
            .map(|t| &t.label)
            .chain(c.commands.iter().map(|t| &t.label))
        {
            ensure!(label.len() <= 80, "label limit");
        }
        ensure!(self.has("handle"), "missing handle(state,event,context)");
        if !c.object_types.is_empty() {
            ensure!(self.has("render"), "missing render(state,node,context)");
        }
        if !c.services.is_empty() {
            ensure!(
                self.has("service"),
                "missing service(state,name,args,context)"
            );
        }
        Ok(c)
    }
}
pub fn json_value(value: &impl Serialize) -> Result<Value> {
    Ok(serde_json::to_value(value)?)
}
