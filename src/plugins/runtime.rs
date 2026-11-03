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

const MAX_STATE_BYTES: usize = 1024 * 1024;
const MAX_CALLBACK_INPUT_BYTES: usize = 16 * 1024 * 1024;
const MAX_CALLBACK_OUTPUT_BYTES: usize = 1024 * 1024;
const MAX_TOOLS: usize = 24;
const MAX_COMMANDS: usize = 24;
const MAX_SERVICES: usize = 32;
const MAX_OBJECT_TYPES: usize = 32;
const MAX_LABEL_BYTES: usize = 80;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ButtonStyle {
    #[serde(default)]
    pub background: Option<[f32; 4]>,

    #[serde(default)]
    pub text: Option<[f32; 4]>,

    #[serde(default)]
    pub selected_background: Option<[f32; 4]>,
}

impl ButtonStyle {
    fn validate(&self) -> Result<()> {
        for color in [self.background, self.text, self.selected_background]
            .into_iter()
            .flatten()
        {
            ensure!(
                color
                    .iter()
                    .all(|value| value.is_finite() && (0.0..=1.0).contains(value)),
                "button color must contain RGBA values between 0 and 1"
            );
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tool {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub text_input: bool,
    #[serde(default)]
    pub style: ButtonStyle,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Command {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub style: ButtonStyle,
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
        ensure!(state.len() <= MAX_STATE_BYTES, "plugin state limit");

        let args = serde_json::to_string(&args)?;
        ensure!(
            args.len() <= MAX_CALLBACK_INPUT_BYTES,
            "callback input limit"
        );

        let out = self
            .backend
            .invoke(&self.sources, &self.entry, &state, name, &args)?;
        ensure!(
            out.len() <= MAX_CALLBACK_OUTPUT_BYTES,
            "callback output limit"
        );

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
        let contributions: Contributions = serde_json::from_value(self.call("activate", vec![])?)?;

        validate_styles(&contributions)?;
        validate_limits(&contributions)?;
        validate_ids(&contributions)?;
        validate_labels(&contributions)?;
        self.require_callbacks(&contributions)?;

        Ok(contributions)
    }

    fn require_callbacks(&self, contributions: &Contributions) -> Result<()> {
        ensure!(self.has("handle"), "missing handle(state,event,context)");

        if !contributions.object_types.is_empty() {
            ensure!(self.has("render"), "missing render(state,node,context)");
        }

        if !contributions.services.is_empty() {
            ensure!(
                self.has("service"),
                "missing service(state,name,args,context)"
            );
        }

        Ok(())
    }
}

fn validate_styles(contributions: &Contributions) -> Result<()> {
    for style in contributions
        .tools
        .iter()
        .map(|tool| &tool.style)
        .chain(contributions.commands.iter().map(|command| &command.style))
    {
        style.validate()?;
    }

    Ok(())
}

fn validate_limits(contributions: &Contributions) -> Result<()> {
    ensure!(
        contributions.tools.len() <= MAX_TOOLS
            && contributions.services.len() <= MAX_SERVICES
            && contributions.object_types.len() <= MAX_OBJECT_TYPES
            && contributions.commands.len() <= MAX_COMMANDS,
        "contribution limit"
    );

    Ok(())
}

fn validate_ids(contributions: &Contributions) -> Result<()> {
    let groups: [Vec<&str>; 4] = [
        contributions.tools.iter().map(|t| t.id.as_str()).collect(),
        contributions
            .commands
            .iter()
            .map(|command| command.id.as_str())
            .collect(),
        contributions.services.iter().map(String::as_str).collect(),
        contributions
            .object_types
            .iter()
            .map(String::as_str)
            .collect(),
    ];

    for ids in groups {
        let mut seen = BTreeSet::new();

        for id in ids {
            ensure!(
                valid_id(id) && seen.insert(id),
                "invalid or duplicate contribution"
            );
        }
    }

    Ok(())
}

fn validate_labels(contributions: &Contributions) -> Result<()> {
    for label in contributions
        .tools
        .iter()
        .map(|tool| &tool.label)
        .chain(contributions.commands.iter().map(|command| &command.label))
    {
        ensure!(label.len() <= MAX_LABEL_BYTES, "label limit");
    }

    Ok(())
}

pub fn json_value(value: &impl Serialize) -> Result<Value> {
    Ok(serde_json::to_value(value)?)
}
