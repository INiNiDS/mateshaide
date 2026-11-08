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
mod ui;
#[cfg(test)]
pub mod tests;
mod panel;
pub mod autosave;

pub use document::PluginNode;

pub const TEXT_PLUGIN: &str = "org.matesha.text";
pub const TEXT_TOOL: &str = "text";
pub const HELP_PLUGIN: &str = "org.matesha.help";

pub use manager::PluginManager;
pub use render::{Scene, UiFrame};
pub(crate) use render::{UiImage, UiImageKind};
pub use ui::PluginPage;
pub(crate) use ui::UiAction;

#[cfg(target_arch = "wasm32")]
pub async fn prepare_browser() -> anyhow::Result<()> {
    lua_web::prepare_browser().await
}
