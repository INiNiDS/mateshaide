use super::package::Package;
use anyhow::{Context, Result, ensure};
use rhai::{
    AST, Array, CallFnOptions, Dynamic, Engine, EvalAltResult, Map, Module, ModuleResolver,
    Position, Scope, Shared,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
use unicode_segmentation::UnicodeSegmentation;

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

struct Resolver {
    sources: BTreeMap<String, String>,
    stack: RefCell<Vec<String>>,
}
impl ModuleResolver for Resolver {
    fn resolve(
        &self,
        engine: &Engine,
        _source: Option<&str>,
        path: &str,
        pos: Position,
    ) -> std::result::Result<Shared<Module>, Box<EvalAltResult>> {
        let name = if path.ends_with(".rhai") {
            path.to_owned()
        } else {
            format!("{path}.rhai")
        };
        if super::manifest::relative_name(&name).is_err()
            || self.stack.borrow().len() >= 8
            || self.stack.borrow().contains(&name)
        {
            return Err(Box::new(EvalAltResult::ErrorModuleNotFound(
                path.into(),
                pos,
            )));
        }
        let source = self
            .sources
            .get(&name)
            .ok_or_else(|| Box::new(EvalAltResult::ErrorModuleNotFound(path.into(), pos)))?;
        self.stack.borrow_mut().push(name);
        let result = (|| {
            let ast = engine
                .compile(source)
                .map_err(|e| Box::<EvalAltResult>::from(e))?;
            Module::eval_ast_as_new(Scope::new(), &ast, engine).map(Shared::new)
        })();
        self.stack.borrow_mut().pop();
        result
    }
}

pub struct Runtime {
    engine: Engine,
    ast: AST,
    pub state: Dynamic,
    started: Rc<RefCell<web_time::Instant>>,
    fuel: Rc<std::cell::Cell<usize>>,
}
impl Runtime {
    // SUPER HIGH COMPLEXITY TODO!!! TODO!!! TOO MANY TODOS!!! TODO!!!
    pub fn new(package: &Package) -> Result<Self> {
        let scripts = package.scripts()?;
        let source = scripts
            .get(&package.manifest.entry)
            .context("entry missing")?
            .clone();
        let mut engine = Engine::new();
        engine.set_max_operations(150_000);
        engine.set_max_call_levels(32);
        engine.set_max_expr_depths(64, 32);
        engine.set_max_string_size(262_144);
        engine.set_max_array_size(65_536);
        engine.set_max_map_size(256);
        engine.set_max_modules(16);
        engine.set_module_resolver(Resolver {
            sources: scripts,
            stack: RefCell::new(Vec::new()),
        });
        engine.on_print(|_| {});
        engine.on_debug(|_, _, _| {});
        let started = Rc::new(RefCell::new(web_time::Instant::now()));
        let clock = Rc::clone(&started);
        engine.on_progress(move |_| {
            if clock.borrow().elapsed().as_millis() > 50 {
                Some(Dynamic::from("plugin time budget exceeded"))
            } else {
                None
            }
        });
        let fuel = Rc::new(std::cell::Cell::new(262_144usize));
        let gfuel = Rc::clone(&fuel);
        engine.register_fn(
            "graphemes",
            move |s: &str| -> std::result::Result<Array, Box<EvalAltResult>> {
                spend(&gfuel, s.len())?;
                Ok(s.graphemes(true)
                    .map(|g| Dynamic::from(g.to_owned()))
                    .collect())
            },
        );
        let jfuel = Rc::clone(&fuel);
        engine.register_fn(
            "join_text",
            move |a: Array| -> std::result::Result<String, Box<EvalAltResult>> {
                let out: String = a
                    .iter()
                    .filter_map(|v| v.clone().try_cast::<String>())
                    .collect();
                spend(&jfuel, out.len())?;
                Ok(out)
            },
        );
        let mut fonts = glyphon::FontSystem::new();
        fonts
            .db_mut()
            .load_font_data(include_bytes!("../../assets/Roboto-Black.ttf").to_vec());
        let fonts = Rc::new(RefCell::new(fonts));
        let tfuel = Rc::clone(&fuel);
        engine.register_fn(
            "text_layout",
            move |text: &str,
                  size: rhai::FLOAT|
                  -> std::result::Result<Dynamic, Box<EvalAltResult>> {
                if text.len() > 32768 || !size.is_finite() || !(4.0..=512.0).contains(&size) {
                    return Err("text_layout input limit".into());
                }
                spend(&tfuel, text.len().max(64))?;
                let result = layout(&mut fonts.borrow_mut(), text, size as f32);
                rhai::serde::to_dynamic(result)
            },
        );
        let ast = engine.compile(source).map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(Self {
            engine,
            ast,
            state: Dynamic::from_map(Map::new()),
            started,
            fuel,
        })
    }
    pub fn has(&self, name: &str, arity: usize) -> bool {
        self.ast
            .iter_functions()
            .any(|f| f.name == name && f.params.len() == arity)
    }
    pub fn call(&mut self, name: &str, args: Vec<Dynamic>) -> Result<Value> {
        ensure!(
            self.has(name, args.len()),
            "missing callback {name}/{}",
            args.len()
        );
        *self.started.borrow_mut() = web_time::Instant::now();
        self.fuel.set(262_144);
        let options = CallFnOptions::new().bind_this_ptr(&mut self.state);
        let mut scope = Scope::new();
        let result = self
            .engine
            .call_fn_with_options::<Dynamic>(options, &mut scope, &self.ast, name, args)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let value =
            rhai::serde::from_dynamic::<Value>(&result).map_err(|e| anyhow::anyhow!("{e}"))?;
        ensure!(
            serde_json::to_vec(&value)?.len() <= 1_048_576,
            "callback output too large"
        );
        Ok(value)
    }
    pub fn pure_call(&mut self, name: &str, args: Vec<Dynamic>) -> Result<Value> {
        let state = self.state.clone();
        let result = self.call(name, args);
        self.state = state;
        result
    }
    pub fn activate(&mut self) -> Result<Contributions> {
        let value = self.call("activate", vec![])?;
        let c: Contributions = serde_json::from_value(value)?;
        ensure!(
            c.tools.len() <= 24
                && c.services.len() <= 32
                && c.object_types.len() <= 32
                && c.commands.len() <= 24,
            "too many contributions"
        );
        for names in [
            c.tools.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(),
            c.commands.iter().map(|t| t.id.as_str()).collect(),
            c.services.iter().map(String::as_str).collect(),
            c.object_types.iter().map(String::as_str).collect(),
        ] {
            let mut seen = std::collections::BTreeSet::new();
            for id in names {
                ensure!(
                    super::manifest::valid_id(id) && seen.insert(id),
                    "invalid or duplicate contribution: {id}"
                );
            }
        }
        for label in c
            .tools
            .iter()
            .map(|t| &t.label)
            .chain(c.commands.iter().map(|t| &t.label))
        {
            ensure!(label.len() <= 80, "label too long");
        }
        ensure!(self.has("handle", 2), "missing handle(event, context)");
        if !c.object_types.is_empty() {
            ensure!(self.has("render", 2), "missing render(node, context)");
        }
        if !c.services.is_empty() {
            ensure!(
                self.has("service", 3),
                "missing service(name, args, context)"
            );
        }
        Ok(c)
    }
}

fn spend(
    fuel: &std::cell::Cell<usize>,
    amount: usize,
) -> std::result::Result<(), Box<EvalAltResult>> {
    if amount > fuel.get() {
        return Err("native helper budget exceeded".into());
    }
    fuel.set(fuel.get() - amount);
    Ok(())
}

pub fn dynamic(value: &impl Serialize) -> Result<Dynamic> {
    rhai::serde::to_dynamic(value).map_err(|e| anyhow::anyhow!("{e}"))
}

// SUPER HIGH COMPLEXITY TODO!!! TODO!!! TOO MANY TODOS!!! TODO!!!
fn layout(fonts: &mut glyphon::FontSystem, text: &str, size: f32) -> Value {
    use glyphon::{Attrs, Buffer, Family, Metrics, Shaping, Wrap};
    let mut buffer = Buffer::new(fonts, Metrics::new(size, size * 1.25));
    buffer.set_wrap(fonts, Wrap::None);
    buffer.set_text(
        text,
        &Attrs::new().family(Family::Name("Roboto")),
        Shaping::Advanced,
        None,
    );
    buffer.shape_until_scroll(fonts, false);
    let gs: Vec<_> = text.grapheme_indices(true).collect();
    let mut carets = vec![json!({"x":0.0,"y":0.0,"line":0}); gs.len() + 1];
    let mut lines = Vec::new();
    let mut byte_base = 0usize;
    let mut line_i = 0usize;
    let mut width = 0.0f32;
    for run in buffer.layout_runs() {
        while line_i < run.line_i {
            let rem = &text[byte_base..];
            byte_base += rem.find('\n').map_or(rem.len(), |n| n + 1);
            line_i += 1;
        }
        width = width.max(run.line_w);
        let start = gs.partition_point(|(b, _)| *b < byte_base);
        let end = gs.partition_point(|(b, _)| *b < byte_base + run.text.len());
        for idx in start..=end.min(gs.len()) {
            let byte = gs
                .get(idx)
                .map_or(text.len(), |(b, _)| *b)
                .saturating_sub(byte_base)
                .min(run.text.len());
            let x = if let Some(g) = run.glyphs.iter().find(|g| g.start <= byte && byte < g.end) {
                let count = run.text[g.start..g.end].graphemes(true).count().max(1) as f32;
                let before = run.text[g.start..byte].graphemes(true).count() as f32 / count;
                if g.level.is_rtl() {
                    g.x + g.w * (1.0 - before)
                } else {
                    g.x + g.w * before
                }
            } else if let Some(g) = run.glyphs.last() {
                if g.level.is_rtl() { g.x } else { g.x + g.w }
            } else {
                0.0
            };
            carets[idx] = json!({"x":x,"y":run.line_top,"line":run.line_i});
        }
        lines.push(json!({"start":start,"end":end,"y":run.line_top,"width":run.line_w}));
    }
    if lines.is_empty() {
        lines.push(json!({"start":0,"end":0,"y":0.0,"width":0.0}));
    }
    json!({"width":width,"height":lines.len() as f32*size*1.25,"line_height":size*1.25,"carets":carets,"lines":lines})
}
