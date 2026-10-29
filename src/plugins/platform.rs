use super::{manager::Settings, package::Package};
use anyhow::{Context, Result, ensure};
use std::{cell::RefCell, collections::BTreeMap, sync::Arc};
use winit::window::Window;

pub enum Request {
    Document(Vec<u8>),
    Package(Vec<u8>),
    Event(serde_json::Value),
    Error(String),
}

const MAX_DOCUMENT_SIZE: usize = 32 * 1024 * 1024;
const MAX_PACKAGE_ARCHIVE_SIZE: usize = 8 * 1024 * 1024;
const MAX_PACKAGE_FILE_SIZE: usize = 2 * 1024 * 1024;
const MAX_ARCHIVE_ENTRIES: usize = 512;
const MAX_BROWSER_PACKAGES: usize = 256;
const SETTINGS_STORAGE_KEY: &str = "matesha.plugins.v1";

thread_local! {
    static WINDOW: RefCell<Option<Arc<Window>>> = const { RefCell::new(None) };
    static LAST_IME: RefCell<Option<bool>> = const { RefCell::new(None) };
    static REQUESTS: RefCell<Vec<Request>> = const { RefCell::new(Vec::new()) };

    #[cfg(not(target_arch = "wasm32"))]
    static CLIPBOARD: RefCell<Option<arboard::Clipboard>> = const { RefCell::new(None) };
}

pub fn bind_window(window: Arc<Window>) {
    #[cfg(target_arch = "wasm32")]
    let wake_window = Arc::clone(&window);

    WINDOW.with(|slot| *slot.borrow_mut() = Some(window));

    #[cfg(target_arch = "wasm32")]
    install_wasm_wake_handler(wake_window);
}

#[cfg(target_arch = "wasm32")]
fn install_wasm_wake_handler(window: Arc<Window>) {
    use wasm_bindgen::JsCast as _;

    let wake = wasm_bindgen::closure::Closure::wrap(
        Box::new(move || window.request_redraw()) as Box<dyn Fn()>
    );

    if let Some(browser_window) = web_sys::window() {
        let _ = js_sys::Reflect::set(
            &browser_window,
            &"__mateshaWake".into(),
            wake.as_ref().unchecked_ref(),
        );
    }

    wake.forget();
    setup_browser();
}

pub fn editor_focus(enabled: bool, x: f32, y: f32) {
    update_native_ime(enabled, x, y);

    #[cfg(target_arch = "wasm32")]
    notify_browser_editor_focus(enabled, x, y);
}

fn update_native_ime(enabled: bool, x: f32, y: f32) {
    WINDOW.with(|slot| {
        let Some(window) = slot.borrow().as_ref() else {
            return;
        };

        LAST_IME.with(|last| {
            let mut last = last.borrow_mut();

            if *last != Some(enabled) {
                window.set_ime_allowed(enabled);
                *last = Some(enabled);
            }
        });

        window.set_ime_cursor_area(
            winit::dpi::PhysicalPosition::new(x as f64, y as f64),
            winit::dpi::PhysicalSize::new(2u32, 24u32),
        );
    });
}

#[cfg(target_arch = "wasm32")]
fn notify_browser_editor_focus(enabled: bool, x: f32, y: f32) {
    let script =
        format!("window.__mateshaEditorFocus && window.__mateshaEditorFocus({enabled},{x},{y})");

    let _ = js_sys::eval(&script);
}

fn push(request: Request) {
    REQUESTS.with(|queue| queue.borrow_mut().push(request));
}

pub fn drain() -> Vec<Request> {
    #[cfg(target_arch = "wasm32")]
    drain_browser_requests();

    REQUESTS.with(|queue| std::mem::take(&mut *queue.borrow_mut()))
}

#[cfg(target_arch = "wasm32")]
fn drain_browser_requests() {
    let Ok(value) =
        js_sys::eval("JSON.stringify((window.__mateshaPluginQueue || []).splice(0, 64))")
    else {
        return;
    };

    let Some(text) = value.as_string() else {
        return;
    };

    let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(&text) else {
        return;
    };

    for item in items {
        push_browser_request(item);
    }
}

#[cfg(target_arch = "wasm32")]
fn push_browser_request(value: serde_json::Value) {
    match value["kind"].as_str() {
        Some("document") => push_bytes_request(value, Request::Document),
        Some("package") => push_bytes_request(value, Request::Package),
        Some("event") => push(Request::Event(value["event"].clone())),
        Some("error") => push(Request::Error(
            value["message"]
                .as_str()
                .unwrap_or("Browser error")
                .to_owned(),
        )),
        _ => {}
    }
}

#[cfg(target_arch = "wasm32")]
fn push_bytes_request(value: serde_json::Value, constructor: fn(Vec<u8>) -> Request) {
    match serde_json::from_value(value["bytes"].clone()) {
        Ok(bytes) => push(constructor(bytes)),
        Err(error) => push(Request::Error(error.to_string())),
    }
}

pub fn load_settings() -> Settings {
    let data = read_settings_data();

    data.and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn read_settings_data() -> Option<String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::fs::read_to_string(settings_path()).ok()
    }

    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|window| window.local_storage().ok().flatten())
            .and_then(|storage| storage.get_item(SETTINGS_STORAGE_KEY).ok().flatten())
    }
}

pub fn save_settings(settings: &Settings) -> Result<()> {
    let text = serde_json::to_string_pretty(settings)?;

    write_settings_data(&text)
}

fn write_settings_data(text: &str) -> Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        write_settings_file(text)
    }

    #[cfg(target_arch = "wasm32")]
    {
        write_settings_storage(text)
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn write_settings_file(text: &str) -> Result<()> {
    let path = settings_path();

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let temporary = path.with_extension("tmp");
    std::fs::write(&temporary, text)?;
    std::fs::rename(temporary, path)?;

    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn write_settings_storage(text: &str) -> Result<()> {
    let storage = browser_storage()?;
    storage
        .set_item(SETTINGS_STORAGE_KEY, text)
        .map_err(|_| anyhow::anyhow!("localStorage write failed"))?;

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn settings_path() -> std::path::PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("APPDATA").map(std::path::PathBuf::from))
        .or_else(|| {
            std::env::var_os("HOME").map(|home| std::path::PathBuf::from(home).join(".config"))
        })
        .unwrap_or_else(|| std::path::PathBuf::from(".matesha"));

    base.join("mateshaide").join("plugins.json")
}

pub fn clipboard_read() -> Result<String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        read_native_clipboard()
    }

    #[cfg(target_arch = "wasm32")]
    {
        anyhow::bail!("In the browser, paste using Ctrl+V into the focused editor")
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn read_native_clipboard() -> Result<String> {
    CLIPBOARD.with(|slot| {
        let mut clipboard = slot.borrow_mut();

        if clipboard.is_none() {
            *clipboard = Some(arboard::Clipboard::new()?);
        }

        let text = clipboard
            .as_mut()
            .context("clipboard unavailable")?
            .get_text()?;

        ensure!(text.len() <= 262144, "clipboard too large");

        Ok(text)
    })
}

pub fn clipboard_write(text: &str) -> Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        write_native_clipboard(text)
    }

    #[cfg(target_arch = "wasm32")]
    {
        write_browser_clipboard(text)
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn write_native_clipboard(text: &str) -> Result<()> {
    CLIPBOARD.with(|slot| {
        let mut clipboard = slot.borrow_mut();

        if clipboard.is_none() {
            *clipboard = Some(arboard::Clipboard::new()?);
        }

        clipboard
            .as_mut()
            .context("clipboard unavailable")?
            .set_text(text)?;

        Ok(())
    })
}

#[cfg(target_arch = "wasm32")]
fn write_browser_clipboard(text: &str) -> Result<()> {
    let quoted = serde_json::to_string(text)?;

    js_sys::eval(&format!(
        "navigator.clipboard.writeText({quoted}).catch(e => \
         (window.__mateshaPluginQueue.push({{kind:'error',message:String(e)}}), \
         window.__mateshaWake && window.__mateshaWake()))"
    ))
    .map_err(|_| anyhow::anyhow!("clipboard write failed"))?;

    Ok(())
}

pub fn open_repository(url: &str) {
    if !url.starts_with("https://") {
        return;
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Err(error) = webbrowser::open(url) {
            push(Request::Error(error.to_string()));
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        open_browser_repository(url);
    }
}

#[cfg(target_arch = "wasm32")]
fn open_browser_repository(url: &str) {
    let Ok(quoted) = serde_json::to_string(url) else {
        return;
    };

    let _ = js_sys::eval(&format!(
        "window.open({quoted}, '_blank', 'noopener,noreferrer')"
    ));
}

pub fn save_document(bytes: &[u8]) {
    crate::export::save::save_with_dialog(bytes, "application/json", "document.matesha.json");
}

pub fn request_open() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        request_document_file();
    }

    #[cfg(target_arch = "wasm32")]
    {
        let _ = js_sys::eval("window.__mateshaChooseFile('document', '.json', 33554432)");
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn request_document_file() {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("Matesha document", &["json"])
        .pick_file()
    else {
        return;
    };

    match read_bounded(&path, MAX_DOCUMENT_SIZE) {
        Ok(bytes) => push(Request::Document(bytes)),
        Err(error) => push(Request::Error(error.to_string())),
    }
}

pub fn request_import() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        request_package_file();
    }

    #[cfg(target_arch = "wasm32")]
    {
        let _ = js_sys::eval("window.__mateshaChooseFile('package', '.zip', 8388608)");
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn request_package_file() {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("Plugin ZIP", &["zip"])
        .pick_file()
    else {
        return;
    };

    match read_bounded(&path, MAX_PACKAGE_ARCHIVE_SIZE) {
        Ok(bytes) => push(Request::Package(bytes)),
        Err(error) => push(Request::Error(error.to_string())),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn read_bounded(path: &std::path::Path, max: usize) -> Result<Vec<u8>> {
    use std::io::Read as _;

    let mut bytes = Vec::new();

    std::fs::File::open(path)?
        .take(max as u64 + 1)
        .read_to_end(&mut bytes)?;

    ensure!(bytes.len() <= max, "file too large");

    Ok(bytes)
}

pub fn install_zip(bytes: &[u8]) -> Result<String> {
    let files = extract_package_files(bytes)?;
    let package = Package::from_files(files, "ZIP import".into())?;
    let plugin_id = package.manifest.id.clone();

    store_package(&plugin_id, &package)?;

    Ok(plugin_id)
}

fn extract_package_files(bytes: &[u8]) -> Result<BTreeMap<String, Vec<u8>>> {
    ensure!(bytes.len() <= MAX_PACKAGE_ARCHIVE_SIZE, "archive too large");

    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))?;

    ensure!(
        archive.len() <= MAX_ARCHIVE_ENTRIES,
        "too many archive entries"
    );

    let files = read_archive_files(&mut archive)?;
    let files = normalize_archive_root(files);
    let files = retain_package_files(files);

    Ok(files)
}

fn read_archive_files(
    archive: &mut zip::ZipArchive<std::io::Cursor<&[u8]>>,
) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut files = BTreeMap::new();
    let mut total_size = 0usize;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;

        if entry.is_dir() {
            continue;
        }

        ensure!(
            entry
                .unix_mode()
                .is_none_or(|mode| mode & 0o170000 != 0o120000),
            "archive symlink forbidden"
        );

        let name = entry.name().to_owned();
        super::manifest::relative_name(&name)?;

        let data = read_archive_entry(&mut entry)?;
        total_size += data.len();

        ensure!(
            total_size <= MAX_PACKAGE_ARCHIVE_SIZE,
            "decompression limit"
        );

        ensure!(
            files.insert(name, data).is_none(),
            "duplicate archive filename"
        );
    }

    Ok(files)
}

fn read_archive_entry<R: std::io::Read + std::io::Seek>(
    entry: &mut zip::read::ZipFile<'_, R>,
) -> Result<Vec<u8>> {
    use std::io::Read as _;

    ensure!(
        entry.size() <= MAX_PACKAGE_FILE_SIZE as u64,
        "archive file too large"
    );

    let mut data = Vec::new();

    entry
        .by_ref()
        .take(MAX_PACKAGE_FILE_SIZE as u64 + 1)
        .read_to_end(&mut data)?;

    ensure!(data.len() <= MAX_PACKAGE_FILE_SIZE, "decompression limit");

    Ok(data)
}

fn normalize_archive_root(mut files: BTreeMap<String, Vec<u8>>) -> BTreeMap<String, Vec<u8>> {
    if files.contains_key("plugin.aam") {
        return files;
    }

    let Some(root) = files
        .keys()
        .find(|name| name.ends_with("/plugin.aam"))
        .and_then(|name| name.strip_suffix("plugin.aam"))
        .map(str::to_owned)
    else {
        return files;
    };

    files
        .into_iter()
        .filter_map(|(name, bytes)| {
            name.strip_prefix(&root)
                .map(|name| (name.to_owned(), bytes))
        })
        .collect()
}

fn retain_package_files(files: BTreeMap<String, Vec<u8>>) -> BTreeMap<String, Vec<u8>> {
    files
        .into_iter()
        .filter(|(name, _)| {
            name == "plugin.aam" || name.ends_with(".rhai") || name.starts_with("assets/")
        })
        .collect()
}

fn store_package(plugin_id: &str, package: &Package) -> Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        write_package_to_disk(plugin_id, package)
    }

    #[cfg(target_arch = "wasm32")]
    {
        write_package_to_storage(plugin_id, package)
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn write_package_to_disk(plugin_id: &str, package: &Package) -> Result<()> {
    let root = std::path::Path::new("plugins");
    std::fs::create_dir_all(root)?;

    let destination = root.canonicalize()?.join(plugin_id);

    ensure!(
        !destination.exists(),
        "plugin already exists; remove its directory before importing an update"
    );

    std::fs::create_dir(&destination)?;

    let result = write_package_files(&destination, package);

    if result.is_err() {
        let _ = std::fs::remove_dir_all(&destination);
    }

    result
}

#[cfg(not(target_arch = "wasm32"))]
fn write_package_files(destination: &std::path::Path, package: &Package) -> Result<()> {
    for (name, bytes) in &package.files {
        let path = destination.join(name);

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(path, bytes)?;
    }

    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn write_package_to_storage(plugin_id: &str, package: &Package) -> Result<()> {
    let storage = browser_storage()?;
    let key = format!("matesha.package.{plugin_id}");
    let files = serde_json::to_string(&package.files)?;

    storage
        .set_item(&key, &files)
        .map_err(|_| anyhow::anyhow!("browser storage full"))?;

    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn browser_storage() -> Result<web_sys::Storage> {
    web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .context("localStorage unavailable")
}

#[cfg(target_arch = "wasm32")]
pub fn browser_packages() -> Vec<BTreeMap<String, Vec<u8>>> {
    let Some(storage) = browser_storage().ok() else {
        return Vec::new();
    };

    let count = storage.length().unwrap_or(0).min(MAX_BROWSER_PACKAGES);

    (0..count)
        .filter_map(|index| storage.key(index).ok().flatten())
        .filter(|key| key.starts_with("matesha.package."))
        .filter_map(|key| storage.get_item(&key).ok().flatten())
        .filter(|data| data.len() <= MAX_DOCUMENT_SIZE)
        .filter_map(|data| serde_json::from_str(&data).ok())
        .collect()
}

#[cfg(target_arch = "wasm32")]
fn setup_browser() {
    let _ = js_sys::eval(
        r#"
(() => {
 if (window.__mateshaPluginQueue) return;
 const queue = window.__mateshaPluginQueue = [];
 const wake=()=>window.__mateshaWake && window.__mateshaWake();
 const emit = event => { if(queue.length < 256) queue.push({kind:'event', event}); wake(); };
 const t = document.createElement('textarea');
 Object.assign(t.style, {position:'fixed', left:'0px', top:'0px', width:'2px', height:'24px', opacity:'0.01', zIndex:'-1'});
 t.setAttribute('aria-label','Matesha text input'); t.autocapitalize='off'; t.spellcheck=false;
 document.body.appendChild(t); let composing=false; let focused=false;
 t.addEventListener('compositionstart', () => {composing=true;});
 t.addEventListener('compositionupdate', e => emit({name:'ime.preedit', text:e.data || ''}));
 t.addEventListener('compositionend', e => {composing=false; emit({name:'text', text:e.data || ''}); emit({name:'ime.preedit',text:''}); t.value='';});
 t.addEventListener('input', () => {if(!composing && t.value){emit({name:'text',text:t.value});t.value='';}});
 t.addEventListener('keydown', e => {
   e.stopPropagation();
   const special=['Backspace','Delete','Enter','Tab','Escape','ArrowLeft','ArrowRight','ArrowUp','ArrowDown','Home','End'];
   if(composing || e.key==='Process') return;
   if(special.includes(e.key) || e.ctrlKey || e.metaKey){
     if((e.ctrlKey || e.metaKey) && e.key.toLowerCase()==='v') return;
     e.preventDefault(); emit({name:'key',key:e.key.length===1?e.key.toLowerCase():e.key,ctrl:e.ctrlKey||e.metaKey,shift:e.shiftKey,alt:e.altKey});
   }
 });
 t.addEventListener('blur',()=>{focused=false;emit({name:'blur'});});
 window.__mateshaEditorFocus=(enabled,x,y)=>{
   t.style.left=Math.max(0,x/window.devicePixelRatio)+'px';t.style.top=Math.max(0,y/window.devicePixelRatio)+'px';
   if(enabled && document.activeElement!==t){focused=true;t.focus({preventScroll:true});}
   if(!enabled && document.activeElement===t){focused=false;t.blur();}
 };
 window.__mateshaChooseFile=(kind,accept,max)=>{
   const input=document.createElement('input'); input.type='file';input.accept=accept;
   input.onchange=async()=>{try{const f=input.files[0];if(!f)return;if(f.size>max)throw Error('File too large');const b=new Uint8Array(await f.arrayBuffer());queue.push({kind,bytes:Array.from(b)});wake();}catch(e){queue.push({kind:'error',message:String(e)});wake();}};input.click();
 };
})();
"#,
    );
}
