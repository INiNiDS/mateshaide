use super::PluginManager;
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::hash::Hasher;
use web_time::{Duration, Instant};

pub const PERMISSION: &str = "document.autosave";
const TICK: Duration = Duration::from_secs(1);
const MAX_STATUS_BYTES: usize = 48;
const MAX_DOCUMENT_OPS: usize = 4;
#[cfg(target_arch = "wasm32")]
const STORAGE_KEY: &str = "matesha.autosave.v1";

#[derive(Default)]
pub struct HostState {
    pub statuses: BTreeMap<String, String>,
    started: Option<Instant>,
    next_tick: Option<Instant>,
    running_ops: bool,
}

impl PluginManager {
    fn autosave_ids(&self) -> Vec<String> {
        self.entries
            .iter()
            .filter(|(_, e)| e.runtime.is_some() && e.package.manifest.permits(PERMISSION))
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn tick(&mut self, canvas: &mut crate::Canvas) -> (Option<Instant>, bool) {
        let ids = self.autosave_ids();
        if ids.is_empty() {
            self.host.next_tick = None;
            return (None, false);
        }
        let now = Instant::now();
        if let Some(next) = self.host.next_tick
            && now < next
        {
            return (Some(next), false);
        }
        let started = *self.host.started.get_or_insert(now);
        let (statuses, notice) = (self.host.statuses.clone(), self.notice.clone());
        let event = json!({
            "name": "autosave.tick",
            "now": (now - started).as_millis() as f64,
            "stamp": canvas.content_stamp(),
            "empty": canvas.is_document_empty(),
        });
        for id in ids {
            self.dispatch(&id, &event, canvas);
        }
        let next = now + TICK;
        self.host.next_tick = Some(next);
        (
            Some(next),
            statuses != self.host.statuses || notice != self.notice,
        )
    }

    pub fn before_exit(&mut self, canvas: &mut crate::Canvas) {
        let event = json!({
            "name": "app.exit",
            "stamp": canvas.content_stamp(),
            "empty": canvas.is_document_empty(),
        });
        for id in self.autosave_ids() {
            self.dispatch(&id, &event, canvas);
        }
    }

    pub(super) fn set_status(&mut self, id: &str, cmd: &Value) -> Result<()> {
        let text = cmd["text"].as_str().unwrap_or_default();
        ensure!(
            text.len() <= MAX_STATUS_BYTES && !text.chars().any(char::is_control),
            "status text limit"
        );
        if text.is_empty() {
            self.host.statuses.remove(id);
        } else {
            self.host.statuses.insert(id.to_owned(), text.to_owned());
        }
        Ok(())
    }

    pub(super) fn queue_document_op(
        ops: &mut Vec<(String, String)>,
        id: &str,
        kind: &str,
    ) -> Result<()> {
        ensure!(ops.len() < MAX_DOCUMENT_OPS, "too many document operations");
        ops.push((id.to_owned(), kind.to_owned()));
        Ok(())
    }

    pub(super) fn run_document_ops(
        &mut self,
        ops: Vec<(String, String)>,
        canvas: &mut crate::Canvas,
    ) {
        if ops.is_empty() {
            return;
        }
        if self.host.running_ops {
            self.notice = "Auto-save: nested document operation ignored".into();
            return;
        }
        self.host.running_ops = true;
        for (plugin, op) in ops {
            let event = match op.as_str() {
                "document.save" => self.save_slot(canvas),
                "document.restore" => self.restore_slot(canvas),
                _ => continue,
            };
            self.dispatch(&plugin, &event, canvas);
        }
        self.host.running_ops = false;
    }

    fn save_slot(&mut self, canvas: &crate::Canvas) -> Value {
        let stamp = canvas.content_stamp();
        let result = canvas
            .document_bytes()
            .and_then(|bytes| write_slot(&bytes).map(|()| bytes.len()));
        match result {
            Ok(bytes) => {
                json!({"name": "document.saved", "ok": true, "stamp": stamp, "bytes": bytes})
            }
            Err(e) => {
                self.notice = format!("Auto-save failed: {e:#}");
                json!({"name": "document.saved", "ok": false, "stamp": stamp, "error": format!("{e:#}")})
            }
        }
    }

    fn restore_slot(&mut self, canvas: &mut crate::Canvas) -> Value {
        match read_slot().and_then(|bytes| canvas.load_document_bytes(&bytes)) {
            Ok(()) => {
                self.document_changed(canvas);
                json!({"name": "document.restored", "ok": true, "stamp": canvas.content_stamp()})
            }
            Err(e) => json!({"name": "document.restored", "ok": false, "error": format!("{e:#}")}),
        }
    }
}

impl crate::Canvas {
    #[must_use]
    pub fn content_stamp(&self) -> String {
        let mut h = rustc_hash::FxHasher::default();
        h.write(bytemuck::cast_slice(&self.instances));
        h.write_usize(self.texts.len());
        for t in &self.texts {
            h.write(t.text.as_bytes());
            for v in [t.position.x, t.position.y, t.size] {
                h.write_u32(v.to_bits());
            }
            for v in t.color.to_array() {
                h.write_u32(v.to_bits());
            }
            h.write(t.font.as_deref().unwrap_or_default().as_bytes());
            h.write_u8(0xff);
        }
        let _ = serde_json::to_writer(HashWriter(&mut h), &self.plugin_nodes);
        format!("{:016x}", h.finish())
    }

    #[must_use]
    pub fn is_document_empty(&self) -> bool {
        self.instances.is_empty() && self.texts.is_empty() && self.plugin_nodes.is_empty()
    }
}

struct HashWriter<'a>(&'a mut rustc_hash::FxHasher);

impl std::io::Write for HashWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.write(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl crate::Engine {
    pub fn plugin_tick(&mut self) -> (Option<Instant>, bool) {
        self.plugins.tick(&mut self.canvas)
    }

    pub fn plugins_before_exit(&mut self) {
        self.plugins.before_exit(&mut self.canvas);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn slot_path() -> std::path::PathBuf {
    #[cfg(test)]
    {
        std::env::temp_dir()
            .join(format!("matesha-autosave-test-{}", std::process::id()))
            .join(format!("{:?}", std::thread::current().id()).replace(['(', ')'], ""))
            .join("document.matesha.json")
    }
    #[cfg(not(test))]
    {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(std::path::PathBuf::from)
            .or_else(|| std::env::var_os("APPDATA").map(std::path::PathBuf::from))
            .or_else(|| {
                std::env::var_os("HOME").map(|home| std::path::PathBuf::from(home).join(".config"))
            })
            .unwrap_or_else(|| std::path::PathBuf::from(".matesha"))
            .join("mateshaide")
            .join("autosave")
            .join("document.matesha.json")
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn write_slot(bytes: &[u8]) -> Result<()> {
    let path = slot_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("tmp");
    std::fs::write(&temporary, bytes)?;
    std::fs::rename(temporary, path)?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn read_slot() -> Result<Vec<u8>> {
    let path = slot_path();
    ensure!(path.is_file(), "no auto-saved document yet");
    Ok(std::fs::read(path)?)
}

#[cfg(target_arch = "wasm32")]
fn storage() -> Result<web_sys::Storage> {
    web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .ok_or_else(|| anyhow::anyhow!("localStorage unavailable"))
}

#[cfg(target_arch = "wasm32")]
pub(super) fn write_slot(bytes: &[u8]) -> Result<()> {
    let text = std::str::from_utf8(bytes)?;
    storage()?
        .set_item(STORAGE_KEY, text)
        .map_err(|_| anyhow::anyhow!("browser storage is full or blocked"))
}

#[cfg(target_arch = "wasm32")]
pub(super) fn read_slot() -> Result<Vec<u8>> {
    let text = storage()?
        .get_item(STORAGE_KEY)
        .map_err(|_| anyhow::anyhow!("browser storage is blocked"))?
        .ok_or_else(|| anyhow::anyhow!("no auto-saved document yet"))?;
    Ok(text.into_bytes())
}
