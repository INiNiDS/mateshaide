pub fn save_bytes(data: &[u8], mime: &str, filename: &str) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        match std::fs::write(filename, data) {
            Ok(()) => tracing::info!("Saved {} ({} bytes)", filename, data.len()),
            Err(e) => {
                tracing::error!("failed to save {}: {}", filename, e);
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    wasm::download(data, mime, filename);
}

pub fn save_with_dialog(data: &[u8], mime: &str, default_filename: &str) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let ext = mime.rsplit('/').next().unwrap();
        if let Some(path) = rfd::FileDialog::new()
            .add_filter(mime, &[ext])
            .set_file_name(default_filename)
            .save_file()
        {
            match std::fs::write(&path, data) {
                Ok(()) => {
                    tracing::info!("Saved {} ({} bytes)", path.display(), data.len());
                }
                Err(e) => {
                    tracing::error!("failed to save {}: {}", path.display(), e);
                }
            }
        }
    }
    #[cfg(target_arch = "wasm32")]
    wasm::download(data, mime, default_filename);
}

pub fn copy_image(rgba: &[u8], width: u32, height: u32) {
    #[cfg(not(target_arch = "wasm32"))]
    native::copy_image(rgba, width, height);

    #[cfg(target_arch = "wasm32")]
    {
        let _ = (width, height);
        wasm::copy_image(rgba);
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    pub fn copy_image(rgba: &[u8], width: u32, height: u32) {
        match arboard::Clipboard::new() {
            Ok(mut clipboard) => {
                let img = arboard::ImageData {
                    width: width as usize,
                    height: height as usize,
                    bytes: std::borrow::Cow::Borrowed(rgba),
                };
                match clipboard.set_image(img) {
                    Ok(()) => tracing::info!("copied to clipboard"),
                    Err(e) => tracing::error!("clipboard error: {e}"),
                }
            }
            Err(e) => tracing::error!("clipboard unavailable: {e}"),
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    pub fn download(data: &[u8], mime: &str, filename: &str) {
        let window = web_sys::window().expect("no global window");

        let arr = js_sys::Uint8Array::new_with_length(data.len() as u32);
        arr.copy_from(data);
        js_sys::Reflect::set(&window, &"__matesha_dl".into(), &arr).expect("set __matesha_dl");

        let script = format!(
            r#"(function() {{
                var d = window.__matesha_dl;
                var b = new Blob([d], {{ type: '{mime}' }});
                var u = URL.createObjectURL(b);
                var a = document.createElement('a');
                a.href = u;
                a.download = '{filename}';
                a.click();
                URL.revokeObjectURL(u);
                delete window.__matesha_dl;
            }})()"#
        );
        js_sys::eval(&script).ok();
    }

    pub fn copy_image(rgba: &[u8]) {
        let window = web_sys::window().expect("no global window");

        let arr = js_sys::Uint8Array::new_with_length(rgba.len() as u32);
        arr.copy_from(rgba);
        js_sys::Reflect::set(&window, &"__matesha_clip".into(), &arr).expect("set __matesha_clip");

        let _ = js_sys::eval(
            r#"(function() {
                var d = window.__matesha_clip;
                var b = new Blob([d], { type: 'image/png' });
                try {
                    var item = new ClipboardItem({ 'image/png': b });
                    navigator.clipboard.write([item]).catch(function(e) {
                        console.error('clipboard write:', e);
                    });
                } catch(e) {
                    console.error('ClipboardItem unsupported:', e);
                }
                delete window.__matesha_clip;
            })()"#,
        );
    }
}
