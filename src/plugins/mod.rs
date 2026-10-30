mod document;
mod helpers;
#[cfg(not(target_arch = "wasm32"))]
mod lua_native;
#[cfg(target_arch = "wasm32")]
mod lua_web;
mod manager;
mod manifest;
mod package;
pub mod platform;
mod render;
mod runtime;
#[cfg(test)]
mod tests;
mod ui;

pub use document::PluginNode;
pub use manager::PluginManager;
pub use render::{Scene, UiFrame};
pub use ui::PluginPage;
pub(crate) use ui::UiAction;

#[cfg(target_arch = "wasm32")]
pub async fn prepare_browser() -> anyhow::Result<()> {
    lua_web::prepare_browser().await
}
