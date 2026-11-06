use super::{
    helpers::Helpers,
    runtime::{BOOTSTRAP_SOURCE, JSON_SOURCE},
};
use anyhow::Result;
use mlua::debug::HookTriggers;
use mlua::{Function, Lua, LuaOptions, StdLib, Table, VmState};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

const MEMORY_LIMIT_BYTES: usize = 64 * 1024 * 1024;
const BUDGET_INSTRUCTION_INTERVAL: u32 = 1_000;

pub struct Backend {
    lua: Lua,
    invoke: Function,
}

impl Backend {
    pub fn new(assets: BTreeMap<String, Vec<u8>>) -> Result<Self> {
        Self::try_new(assets).map_err(|e| anyhow::anyhow!("Lua init: {e}"))
    }

    fn try_new(assets: BTreeMap<String, Vec<u8>>) -> mlua::Result<Self> {
        let lua = open_sandbox()?;
        let host = make_host_bridge(&lua, assets)?;
        let clock = make_clock(&lua)?;
        let hook = make_budget_hook(&lua)?;

        let json: Table = lua.load(JSON_SOURCE).set_name("@matesha/json.lua").eval()?;
        let factory: Function = lua
            .load(BOOTSTRAP_SOURCE)
            .set_name("@matesha/bootstrap.lua")
            .eval()?;
        let invoke: Function = factory.call((json, host, clock, hook))?;

        Ok(Self { lua, invoke })
    }

    pub fn invoke(
        &mut self,
        files: &str,
        entry: &str,
        state: &str,
        name: &str,
        args: &str,
    ) -> Result<String> {
        let result = self
            .invoke
            .call::<String>((files, entry, state, name, args));
        self.lua.remove_hook();
        let _ = self.lua.gc_step();
        result.map_err(|e| anyhow::anyhow!("Lua: {e}"))
    }
}

fn open_sandbox() -> mlua::Result<Lua> {
    let lua = Lua::new_with(
        StdLib::TABLE | StdLib::STRING | StdLib::MATH | StdLib::UTF8,
        LuaOptions::default(),
    )?;
    lua.set_memory_limit(MEMORY_LIMIT_BYTES)?;
    Ok(lua)
}

fn make_host_bridge(lua: &Lua, assets: BTreeMap<String, Vec<u8>>) -> mlua::Result<Function> {
    let helpers = Rc::new(RefCell::new(Helpers::new(assets)));
    lua.create_function(move |_, (op, input): (String, String)| {
        Ok(helpers.borrow_mut().bridge(&op, &input))
    })
}

fn make_clock(lua: &Lua) -> mlua::Result<Function> {
    let epoch = web_time::Instant::now();
    lua.create_function(move |_, ()| Ok(epoch.elapsed().as_secs_f64() * 1000.0))
}

fn make_budget_hook(lua: &Lua) -> mlua::Result<Function> {
    lua.create_function(|lua, (enabled, callback): (bool, Option<Function>)| {
        if !enabled {
            lua.remove_hook();
            return Ok(());
        }
        let callback =
            callback.ok_or_else(|| mlua::Error::RuntimeError("missing budget callback".into()))?;
        lua.set_hook(
            HookTriggers::new().every_nth_instruction(BUDGET_INSTRUCTION_INTERVAL),
            move |_, _| {
                callback.call::<()>(())?;
                Ok(VmState::Continue)
            },
        )?;
        Ok(())
    })
}
