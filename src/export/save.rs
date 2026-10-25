pub fn save_bytes(data: &[u8], mime: &str, filename: &str) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = mime;
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
    wasm::copy_image(rgba, width, height);
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use arboard::{Clipboard, ImageData};
    use std::borrow::Cow;
    use std::cell::RefCell;

    thread_local! {
        static CLIPBOARD: RefCell<Option<Clipboard>> = const { RefCell::new(None) };
    }

    pub fn copy_image(rgba: &[u8], width: u32, height: u32) {
        CLIPBOARD.with(|slot| {
            let mut slot = slot.borrow_mut();

            if slot.is_none() {
                match Clipboard::new() {
                    Ok(cb) => *slot = Some(cb),
                    Err(e) => {
                        tracing::error!("clipboard unavailable: {e}");
                        return;
                    }
                }
            }

            let Some(clipboard) = slot.as_mut() else { return };

            let img = ImageData {
                width: width as usize,
                height: height as usize,
                bytes: Cow::Borrowed(rgba),
            };

            match clipboard.set_image(img) {
                Ok(()) => tracing::info!("copied to clipboard"),
                Err(e) => {
                    tracing::error!("clipboard error: {e}");
                    *slot = None;
                }
            }
        });
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

    pub fn copy_image(rgba: &[u8], width: u32, height: u32) {
        let png = match crate::export::png::encode(rgba, width, height) {
            Ok(png) => png,
            Err(e) => {
                tracing::error!("failed to encode clipboard image: {e}");
                return;
            }
        };
        let window = web_sys::window().expect("no global window");

        let arr = js_sys::Uint8Array::new_with_length(png.len() as u32);
        arr.copy_from(&png);
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
