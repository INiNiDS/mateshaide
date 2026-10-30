use super::{
    helpers::Helpers,
    runtime::{BOOTSTRAP_SOURCE, JSON_SOURCE},
};
use anyhow::Result;
use mlua::debug::HookTriggers;
use mlua::{Function, Lua, LuaOptions, StdLib, Table, VmState};
use std::{cell::RefCell, rc::Rc};

pub struct Backend {
    lua: Lua,
    invoke: Function,
}
impl Backend {
    pub fn new() -> Result<Self> {
        let make = || -> mlua::Result<Self> {
            let lua = Lua::new_with(
                StdLib::TABLE | StdLib::STRING | StdLib::MATH | StdLib::UTF8,
                LuaOptions::default(),
            )?;
            lua.set_memory_limit(64 * 1024 * 1024)?;
            let helpers = Rc::new(RefCell::new(Helpers::new()));
            let host = lua.create_function(move |_, (op, input): (String, String)| {
                Ok(helpers.borrow_mut().bridge(&op, &input))
            })?;
            let epoch = web_time::Instant::now();
            let clock =
                lua.create_function(move |_, ()| Ok(epoch.elapsed().as_secs_f64() * 1000.0))?;
            let hook =
                lua.create_function(|lua, (enabled, callback): (bool, Option<Function>)| {
                    if enabled {
                        let callback = callback.ok_or_else(|| {
                            mlua::Error::RuntimeError("missing budget callback".into())
                        })?;
                        lua.set_hook(
                            HookTriggers::new().every_nth_instruction(1000),
                            move |_, _| {
                                callback.call::<()>(())?;
                                Ok(VmState::Continue)
                            },
                        )?;
                    } else {
                        lua.remove_hook();
                    }
                    Ok(())
                })?;
            let json: Table = lua.load(JSON_SOURCE).set_name("@matesha/json.lua").eval()?;
            let factory: Function = lua
                .load(BOOTSTRAP_SOURCE)
                .set_name("@matesha/bootstrap.lua")
                .eval()?;
            let invoke: Function = factory.call((json, host, clock, hook))?;
            Ok(Self { lua, invoke })
        };
        make().map_err(|e| anyhow::anyhow!("Lua init: {e}"))
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
