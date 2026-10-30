pub(super) const JSON_SOURCE: &str = include_str!("json.lua");
pub(super) const BOOTSTRAP_SOURCE: &str = include_str!("bootstrap.lua");

#[cfg(not(target_arch = "wasm32"))]
use super::lua_native::Backend;