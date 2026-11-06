#[cfg(all(test, not(target_arch = "wasm32")))]
mod text_tool_tests {
    use crate::Canvas;
    use crate::plugins::PluginManager;
    use crate::plugins::manager::{Entry, Settings};
    use crate::plugins::package::Package;
    use serde_json::{Value, json};

    fn editor() -> (PluginManager, Canvas) {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/text-editor");
        let files = ["plugin.aam", "main.lua", "assets/icon.png"]
            .into_iter()
            .map(|f| (f.to_owned(), std::fs::read(root.join(f)).unwrap()))
            .collect();
        let package = Package::from_files(files, "test".into()).unwrap();
        let mut manager = PluginManager::with_settings(Settings::default());
        let id = package.manifest.id.clone();
        assert_eq!(id, super::super::TEXT_PLUGIN);
        manager.entries.insert(id.clone(), Entry::new(package));
        manager.start_plugin(&id).unwrap();
        let mut canvas = Canvas::new(800.0, 600.0);
        manager.select_tool(&id, super::super::TEXT_TOOL, &mut canvas);
        (manager, canvas)
    }

    fn send(m: &mut PluginManager, c: &mut Canvas, event: &Value) {
        m.dispatch(super::super::TEXT_PLUGIN, event, c);
        assert!(m.notice.is_empty(), "{}", m.notice);
    }

    fn click(m: &mut PluginManager, c: &mut Canvas, x: f32, y: f32) {
        for phase in ["down", "up"] {
            let event = json!({
                "name": "pointer", "phase": phase, "x": x, "y": y, "shift": false, "ctrl": false
            });
            send(m, c, &event);
        }
    }

    fn key(m: &mut PluginManager, c: &mut Canvas, key: &str) {
        let event = json!({"name": "key", "key": key, "ctrl": false, "shift": false, "alt": false});
        send(m, c, &event);
    }

    #[test]
    fn click_places_caret_and_first_char_creates_block_in_one_undo_step() {
        let (mut m, mut c) = editor();
        click(&mut m, &mut c, 100.0, 50.0);
        assert!(
            m.wants_text_input(),
            "caret is active right after the click"
        );
        assert_eq!(
            c.plugin_nodes.len(),
            0,
            "empty block is not in the document yet"
        );

        send(&mut m, &mut c, &json!({"name": "text", "text": "a"}));
        send(&mut m, &mut c, &json!({"name": "text", "text": "b"}));
        assert_eq!(c.plugin_nodes.len(), 1);
        assert_eq!(c.plugin_nodes[0].x, 100.0);
        assert_eq!(c.plugin_nodes[0].y, 50.0);
        assert_eq!(c.plugin_nodes[0].data["text"], "ab");

        key(&mut m, &mut c, "Escape");
        assert!(!m.wants_text_input());
        assert_eq!(c.plugin_nodes.len(), 1);

        assert!(c.undo());
        assert!(c.undo());
        assert_eq!(c.plugin_nodes.len(), 0, "undo removes the created block");
        assert!(!c.undo());
    }

    #[test]
    fn empty_block_is_dropped_on_escape_click_outside_and_blur() {
        let (mut m, mut c) = editor();

        click(&mut m, &mut c, 0.0, 0.0);
        key(&mut m, &mut c, "Escape");
        assert!(!m.wants_text_input());

        click(&mut m, &mut c, 0.0, 0.0);
        click(&mut m, &mut c, 400.0, 400.0);
        assert!(!m.wants_text_input(), "click outside only finishes editing");

        click(&mut m, &mut c, 400.0, 400.0);
        assert!(m.wants_text_input(), "next click starts a new block");
        m.blur(&mut c);

        assert_eq!(c.plugin_nodes.len(), 0);
        assert!(!c.undo(), "nothing was recorded in history");
    }

    #[test]
    fn block_emptied_by_backspace_is_removed_on_finish() {
        let (mut m, mut c) = editor();
        click(&mut m, &mut c, 0.0, 0.0);
        send(&mut m, &mut c, &json!({"name": "text", "text": "x"}));
        key(&mut m, &mut c, "Backspace");
        assert_eq!(c.plugin_nodes.len(), 1);
        key(&mut m, &mut c, "Escape");
        assert_eq!(c.plugin_nodes.len(), 0);
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod script_input_tests {
    use crate::Canvas;
    use crate::plugins::PluginManager;
    use crate::plugins::manager::{Entry, Settings};
    use crate::plugins::package::Package;
    use serde_json::{Value, json};

    const ID: &str = "org.matesha.text";

    fn editor() -> (PluginManager, Canvas) {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/text-editor");
        let files = ["plugin.aam", "main.lua", "assets/icon.png"]
            .into_iter()
            .map(|f| (f.to_owned(), std::fs::read(root.join(f)).unwrap()))
            .collect();
        let package = Package::from_files(files, "test".into()).unwrap();
        let mut manager = PluginManager::with_settings(Settings::default());
        assert_eq!(package.manifest.id, ID);
        manager.entries.insert(ID.to_owned(), Entry::new(package));
        manager.start_plugin(ID).unwrap();
        let mut canvas = Canvas::new(800.0, 600.0);
        manager.select_tool(ID, "text", &mut canvas);
        (manager, canvas)
    }

    fn send(m: &mut PluginManager, c: &mut Canvas, event: &Value) {
        m.dispatch(ID, event, c);
        assert!(m.notice.is_empty(), "{}", m.notice);
        let nodes = c.plugin_nodes.clone();
        m.project(&nodes, true, 1.0);
        assert!(m.notice.is_empty(), "{}", m.notice);
    }

    fn type_text(m: &mut PluginManager, c: &mut Canvas, text: &str) {
        for phase in ["down", "up"] {
            let event = json!({
                "name": "pointer", "phase": phase, "x": 100.0, "y": 50.0, "shift": false, "ctrl": false
            });
            send(m, c, &event);
        }
        send(m, c, &json!({"name": "text", "text": text}));
    }

    fn shift_key(m: &mut PluginManager, c: &mut Canvas, key: &str) {
        let event = json!({"name": "key", "key": key, "ctrl": false, "shift": true, "alt": false});
        send(m, c, &event);
    }

    #[test]
    fn shift_letter_after_char_makes_superscript() {
        let (mut m, mut c) = editor();
        type_text(&mut m, &mut c, "x");
        shift_key(&mut m, &mut c, "N");
        send(&mut m, &mut c, &json!({"name": "text", "text": "N"}));
        assert_eq!(c.plugin_nodes[0].data["text"], "x^{n}");
        shift_key(&mut m, &mut c, "ArrowUp");
    }

    #[test]
    fn typed_scripts_lay_out_and_render() {
        let samples = [
            "x^2",
            "a_i",
            "x^{n+1}_k",
            "∑_{i=1}^{n} i^2",
            "∫_0^1 x^2 dx",
            "√{x_1}",
            "^",
            "_",
        ];
        for text in samples {
            let (mut m, mut c) = editor();
            type_text(&mut m, &mut c, text);
            assert_eq!(c.plugin_nodes[0].data["text"], text);
        }
    }

    fn press(m: &mut PluginManager, c: &mut Canvas, key: &str, shift: bool, text: &str) {
        let event = json!({"name": "key", "key": key, "ctrl": false, "shift": shift, "alt": false});
        send(m, c, &event);
        send(m, c, &json!({"name": "text", "text": text}));
    }

    fn plain_key(m: &mut PluginManager, c: &mut Canvas, key: &str) {
        let event = json!({"name": "key", "key": key, "ctrl": false, "shift": false, "alt": false});
        send(m, c, &event);
    }

    fn text_of(c: &Canvas) -> &str {
        c.plugin_nodes[0].data["text"].as_str().unwrap()
    }
    #[test]
    fn shift_digit_symbols_are_literal() {
        let (mut m, mut c) = editor();
        type_text(&mut m, &mut c, "f");
        press(&mut m, &mut c, "9", true, "(");
        press(&mut m, &mut c, "x", false, "x");
        press(&mut m, &mut c, "0", true, ")");
        press(&mut m, &mut c, "1", true, "!");
        assert_eq!(text_of(&c), "f(x)!");
        press(&mut m, &mut c, "n", true, "N");
        assert_eq!(text_of(&c), "f(x)!^{n}");
    }

    #[test]
    fn closing_brace_steps_out_of_script_group() {
        let (mut m, mut c) = editor();
        type_text(&mut m, &mut c, "x");
        press(&mut m, &mut c, "n", true, "N");
        assert_eq!(text_of(&c), "x^{n}");
        press(&mut m, &mut c, "]", true, "}");
        press(&mut m, &mut c, "+", false, "+");
        assert_eq!(text_of(&c), "x^{n}+");
    }

    #[test]
    fn open_brace_inserts_pair_and_inner_close_is_literal() {
        let (mut m, mut c) = editor();
        type_text(&mut m, &mut c, "x^");
        press(&mut m, &mut c, "[", true, "{");
        assert_eq!(text_of(&c), "x^{}");
        press(&mut m, &mut c, "a", false, "a");
        plain_key(&mut m, &mut c, "ArrowLeft");
        press(&mut m, &mut c, "]", true, "}");
        assert_eq!(text_of(&c), "x^{\\}a}");
        plain_key(&mut m, &mut c, "Backspace");
        assert_eq!(text_of(&c), "x^{a}");
        plain_key(&mut m, &mut c, "ArrowRight");
        press(&mut m, &mut c, "]", true, "}");
        press(&mut m, &mut c, "]", true, "}");
        assert_eq!(text_of(&c), "x^{a}}");
    }

    #[test]
    fn escaped_braces_render_and_do_not_close_groups() {
        for text in [
            "x^{a\\}b}",
            "\\{1, 2\\}",
            "a//{b\\}",
            "∑_{k\\{}^{n} k",
            "\\",
            "\\}",
        ] {
            let (mut m, mut c) = editor();
            type_text(&mut m, &mut c, text);
            assert_eq!(text_of(&c), text);
            for key in ["Backspace", "ArrowLeft", "Delete"] {
                plain_key(&mut m, &mut c, key);
            }
        }
    }

    #[test]
    fn shift_arrows_over_scripts_and_lines() {
        let (mut m, mut c) = editor();
        type_text(&mut m, &mut c, "a_i\n∑_{k=1}^{n} x^2\n");
        for _ in 0..3 {
            shift_key(&mut m, &mut c, "ArrowUp");
        }
        for key in ["ArrowDown", "ArrowLeft", "ArrowUp"] {
            shift_key(&mut m, &mut c, key);
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod asset_api_tests {
    use crate::Canvas;
    use crate::plugins::PluginManager;
    use crate::plugins::manager::{Entry, Settings};
    use crate::plugins::package::Package;
    use serde_json::{Value, json};
    use std::collections::BTreeMap;

    const ID: &str = "test.assets";
    const LUA: &str = r#"
local P = {}
function P.activate(s)
    return { tools = { { id = "t", label = "T" } } }
end
function P.handle(s, event, ctx)
    if event.name == "probe" then
        s.list = assets.list()
        s.text = assets.read("assets/data.txt")
        s.family = fonts.load("assets/fonts/test.ttf")
        s.families = fonts.list()
    elseif event.name == "read" then
        assets.read(event.path)
    elseif event.name == "font" then
        fonts.load(event.path)
    elseif event.name == "pick" then
        return { { kind = "file.pick", accept = event.accept, token = 7 } }
    elseif event.name == "file.picked" then
        s.picked = event
    end
    return array()
end
return P
"#;

    fn manager(permissions: &str) -> (PluginManager, Canvas) {
        let manifest = format!(
            "id = \"{ID}\"\nname = \"Assets\"\nversion = \"1.0.0\"\ndescription = \"t\"\n\
             authors = [\"t\"]\nrepository = \"https://example.com\"\nicon = \"assets/icon.png\"\n\
             entry = \"main.lua\"\napi = \"^1.0\"\npermissions = [{permissions}]\ndependencies = []\n"
        );
        let font = include_bytes!("../../assets/Roboto-Black.ttf").to_vec();
        let files = BTreeMap::from([
            ("plugin.aam".to_owned(), manifest.into_bytes()),
            ("main.lua".to_owned(), LUA.as_bytes().to_vec()),
            ("assets/data.txt".to_owned(), "∑ data".as_bytes().to_vec()),
            ("assets/fonts/test.ttf".to_owned(), font),
        ]);
        let package = Package::from_files(files, "test".into()).unwrap();
        let mut m = PluginManager::with_settings(Settings::default());
        m.entries.insert(ID.to_owned(), Entry::new(package));
        m.start_plugin(ID).unwrap();
        let mut c = Canvas::new(800.0, 600.0);
        m.select_tool(ID, "t", &mut c);
        (m, c)
    }

    fn state(m: &PluginManager) -> &Value {
        &m.entries[ID].runtime.as_ref().unwrap().state
    }

    fn rejected(m: &mut PluginManager, c: &mut Canvas, event: &Value) -> String {
        m.notice.clear();
        m.dispatch(ID, event, c);
        assert!(!m.notice.is_empty(), "{event} must be rejected");
        std::mem::take(&mut m.notice)
    }

    #[test]
    fn plugin_reads_own_assets_and_loads_fonts() {
        let (mut m, mut c) = manager("");
        m.dispatch(ID, &json!({"name": "probe"}), &mut c);
        assert!(m.notice.is_empty(), "{}", m.notice);
        let s = state(&m);
        assert_eq!(s["text"], "∑ data");
        assert_eq!(s["family"], crate::graphics::fonts::DEFAULT_FAMILY);
        assert_eq!(s["families"][0], crate::graphics::fonts::DEFAULT_FAMILY);
        let paths: Vec<_> = s["list"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["path"].clone())
            .collect();
        assert_eq!(
            paths,
            vec![json!("assets/data.txt"), json!("assets/fonts/test.ttf")]
        );
    }

    #[test]
    fn asset_access_is_sandboxed_to_package_assets() {
        let (mut m, mut c) = manager("");
        for path in [
            "assets/../plugin.aam",
            "main.lua",
            "/etc/passwd",
            "assets/missing.txt",
            "C:/x",
            "",
        ] {
            rejected(&mut m, &mut c, &json!({"name": "read", "path": path}));
        }
        let err = rejected(
            &mut m,
            &mut c,
            &json!({"name": "read", "path": "assets/fonts/test.ttf"}),
        );
        assert!(err.contains("UTF-8"), "{err}");
        let err = rejected(
            &mut m,
            &mut c,
            &json!({"name": "font", "path": "assets/data.txt"}),
        );
        assert!(err.contains(".ttf"), "{err}");
    }

    #[test]
    fn file_pick_needs_permission_and_known_kind() {
        let (mut m, mut c) = manager("");
        let err = rejected(&mut m, &mut c, &json!({"name": "pick", "accept": "font"}));
        assert!(err.contains("files.pick"), "{err}");

        let (mut m, mut c) = manager("\"files.pick\"");
        rejected(&mut m, &mut c, &json!({"name": "pick", "accept": "exe"}));
    }

    #[test]
    fn picked_font_is_registered_and_delivered() {
        let (mut m, mut c) = manager("\"files.pick\"");
        let file = super::super::platform::PluginFile {
            plugin: ID.into(),
            accept: "font".into(),
            token: json!(7),
            name: "Roboto-Black.ttf".into(),
            bytes: include_bytes!("../../assets/Roboto-Black.ttf").to_vec(),
        };
        m.deliver_file(file, &mut c);
        let picked = &state(&m)["picked"];
        assert_eq!(picked["ok"], true);
        assert_eq!(picked["family"], crate::graphics::fonts::DEFAULT_FAMILY);
        assert_eq!(picked["token"], 7);

        let bad = super::super::platform::PluginFile {
            plugin: ID.into(),
            accept: "font".into(),
            token: json!(8),
            name: "x.ttf".into(),
            bytes: b"nope".to_vec(),
        };
        m.deliver_file(bad, &mut c);
        assert_eq!(state(&m)["picked"]["ok"], false);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    fn normalize_archive_root(files: BTreeMap<String, Vec<u8>>) -> BTreeMap<String, Vec<u8>> {
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
                name == "plugin.aam"
                    || std::path::Path::new(name)
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("lua"))
                    || name.starts_with("assets/")
            })
            .collect()
    }

    #[test]
    fn imported_package_keeps_lua_modules() {
        let files = BTreeMap::from([
            ("plugin.aam".to_owned(), vec![1]),
            ("main.lua".to_owned(), vec![2]),
            ("lib/helpers.lua".to_owned(), vec![3]),
            ("assets/icon.png".to_owned(), vec![4]),
            ("README.md".to_owned(), vec![5]),
            ("legacy.rhai".to_owned(), vec![6]),
        ]);
        let files = retain_package_files(files);
        assert_eq!(files.len(), 4);
        assert!(files.contains_key("main.lua"));
        assert!(files.contains_key("lib/helpers.lua"));
        assert!(files.contains_key("assets/icon.png"));
        assert!(!files.contains_key("legacy.rhai"));
    }

    #[test]
    fn imported_package_strips_wrapper_directory() {
        let files = BTreeMap::from([
            ("plugin-1.0/plugin.aam".to_owned(), vec![1]),
            ("plugin-1.0/main.lua".to_owned(), vec![2]),
            ("plugin-1.0/lib/helpers.lua".to_owned(), vec![3]),
            ("outside.txt".to_owned(), vec![4]),
        ]);
        let files = retain_package_files(normalize_archive_root(files));
        assert_eq!(files.len(), 3);
        assert!(files.contains_key("plugin.aam"));
        assert!(files.contains_key("main.lua"));
        assert!(files.contains_key("lib/helpers.lua"));
    }

    #[test]
    fn imported_package_preserves_existing_root() {
        let files = BTreeMap::from([
            ("plugin.aam".to_owned(), vec![1]),
            ("main.lua".to_owned(), vec![2]),
        ]);
        assert_eq!(normalize_archive_root(files.clone()), files);
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod font_tests {
    use crate::Canvas;
    use crate::graphics::fonts::{DEFAULT_FAMILY, DEFAULT_FONT_BYTES};
    use crate::plugins::PluginManager;
    use crate::plugins::manager::{Entry, Settings};
    use crate::plugins::package::Package;
    use serde_json::{Value, json};
    use std::collections::BTreeMap;

    const ID: &str = "org.matesha.text";

    fn renamed_font(family: &str) -> Vec<u8> {
        assert_eq!(family.len(), 6);
        let from: Vec<u8> = "Roboto".encode_utf16().flat_map(u16::to_be_bytes).collect();
        let to: Vec<u8> = family.encode_utf16().flat_map(u16::to_be_bytes).collect();
        let mut bytes = DEFAULT_FONT_BYTES.to_vec();
        let mut i = 0;
        while i + from.len() <= bytes.len() {
            if bytes[i..i + from.len()] == from[..] {
                bytes[i..i + from.len()].copy_from_slice(&to);
                i += from.len();
            } else {
                i += 1;
            }
        }
        bytes
    }

    fn editor() -> (PluginManager, Canvas) {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/text-editor");
        let mut files: BTreeMap<String, Vec<u8>> = ["plugin.aam", "main.lua", "assets/icon.png"]
            .into_iter()
            .map(|f| (f.to_owned(), std::fs::read(root.join(f)).unwrap()))
            .collect();
        files.insert("assets/fonts/Bundl.ttf".into(), renamed_font("Bundld"));
        let package = Package::from_files(files, "test".into()).unwrap();
        let mut manager = PluginManager::with_settings(Settings::default());
        manager.entries.insert(ID.to_owned(), Entry::new(package));
        manager.start_plugin(ID).unwrap();
        let mut canvas = Canvas::new(800.0, 600.0);
        manager.select_tool(ID, "text", &mut canvas);
        (manager, canvas)
    }

    fn send(m: &mut PluginManager, c: &mut Canvas, event: &Value) {
        m.dispatch(ID, event, c);
        assert!(m.notice.is_empty(), "{}", m.notice);
    }

    fn type_text(m: &mut PluginManager, c: &mut Canvas, x: f32, text: &str) {
        for phase in ["down", "up"] {
            let event = json!({
                "name": "pointer", "phase": phase, "x": x, "y": 50.0, "shift": false, "ctrl": false
            });
            send(m, c, &event);
        }
        send(m, c, &json!({"name": "text", "text": text}));
    }

    fn command(m: &mut PluginManager, c: &mut Canvas, id: &str) {
        send(m, c, &json!({"name": "command", "id": id}));
    }

    fn font_of(c: &Canvas, i: usize) -> Value {
        c.plugin_nodes[i]
            .data
            .get("font")
            .cloned()
            .unwrap_or(Value::Null)
    }

    fn rendered_fonts(m: &mut PluginManager, c: &Canvas) -> Vec<Option<String>> {
        let nodes = c.plugin_nodes.clone();
        let scene = m.project(&nodes, false, 1.0);
        assert!(m.notice.is_empty(), "{}", m.notice);
        scene.texts.iter().map(|t| t.font.clone()).collect()
    }

    #[test]
    fn bundled_font_is_loaded_on_activate() {
        let (_m, _c) = editor();
        assert_eq!(
            crate::graphics::fonts::families(),
            vec![DEFAULT_FAMILY.to_owned(), "Bundld".to_owned()]
        );
    }

    #[test]
    fn font_next_switches_block_font_and_undo_restores_it() {
        let (mut m, mut c) = editor();
        type_text(&mut m, &mut c, 100.0, "x^2");
        assert_eq!(font_of(&c, 0), Value::Null);

        command(&mut m, &mut c, "font_next");
        assert_eq!(font_of(&c, 0), "Bundld");
        assert!(
            rendered_fonts(&mut m, &c)
                .iter()
                .all(|f| f.as_deref() == Some("Bundld"))
        );
        assert!(c.plugin_nodes[0].width > 0.0);

        assert!(c.undo());
        assert_eq!(font_of(&c, 0), Value::Null);
        assert!(c.redo());
        assert_eq!(font_of(&c, 0), "Bundld");

        command(&mut m, &mut c, "font_next");
        assert_eq!(
            font_of(&c, 0),
            Value::Null,
            "cycles back to the default font"
        );
    }

    #[test]
    fn new_blocks_inherit_last_chosen_font() {
        let (mut m, mut c) = editor();
        type_text(&mut m, &mut c, 100.0, "a");
        command(&mut m, &mut c, "font_next");
        send(
            &mut m,
            &mut c,
            &json!({"name": "key", "key": "Escape", "ctrl": false, "shift": false, "alt": false}),
        );
        type_text(&mut m, &mut c, 500.0, "b");
        assert_eq!(c.plugin_nodes.len(), 2);
        assert_eq!(font_of(&c, 1), "Bundld");
    }

    #[test]
    fn picked_font_is_applied_to_the_block() {
        let (mut m, mut c) = editor();
        type_text(&mut m, &mut c, 100.0, "a");
        let node_id = c.plugin_nodes[0].id.clone();
        m.entries
            .get_mut(ID)
            .unwrap()
            .runtime
            .as_mut()
            .unwrap()
            .state["font_target"] = json!(node_id);
        let file = super::super::platform::PluginFile {
            plugin: ID.into(),
            accept: "font".into(),
            token: Value::Null,
            name: "Picked.ttf".into(),
            bytes: renamed_font("Picked"),
        };
        m.deliver_file(file, &mut c);
        assert!(m.notice.is_empty(), "{}", m.notice);
        assert_eq!(font_of(&c, 0), "Picked");
        assert!(c.undo());
        assert_eq!(font_of(&c, 0), Value::Null);
    }

    #[test]
    fn unknown_font_in_document_falls_back_without_errors() {
        let (mut m, mut c) = editor();
        type_text(&mut m, &mut c, 100.0, "∑_{i=1}^{n} i");
        c.plugin_nodes[0].data["font"] = json!("Missing Font");
        assert!(
            rendered_fonts(&mut m, &c)
                .iter()
                .all(|f| f.as_deref() == Some("Missing Font"))
        );
        assert_eq!(
            crate::graphics::fonts::resolve(Some("Missing Font")),
            DEFAULT_FAMILY
        );
        send(&mut m, &mut c, &json!({"name": "text", "text": "+"}));
        assert_eq!(
            font_of(&c, 0),
            "Missing Font",
            "the name is kept in the document"
        );
    }

    #[test]
    fn canvas_text_font_is_optional_in_saved_documents() {
        let mut text = crate::CanvasText {
            position: glam::Vec2::ZERO,
            text: "a".into(),
            size: 12.0,
            color: crate::Color::WHITE,
            font: None,
        };
        let plain = serde_json::to_value(&text).unwrap();
        assert!(plain.get("font").is_none());
        text.font = Some("Bundld".into());
        let back: crate::CanvasText =
            serde_json::from_value(serde_json::to_value(&text).unwrap()).unwrap();
        assert_eq!(back, text);
        let old: crate::CanvasText = serde_json::from_value(plain).unwrap();
        assert_eq!(old.font, None);
    }
}
