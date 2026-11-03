use super::manifest::{Manifest, relative_name};
use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::Read;

const MAX_PACKAGE_FILES: usize = 256;
const MAX_PACKAGE_BYTES: usize = 8 * 1024 * 1024;
const MAX_FILE_BYTES: usize = 2 * 1024 * 1024;
const MAX_PLUGINS: usize = 128;
const MAX_DIR_DEPTH: usize = 12;
const ICON_SIZE: u32 = 24;
const ICON_SOURCE_MAX_EDGE: u32 = 512;
const ICON_DECODE_MAX_ALLOC: u64 = 4 * 1024 * 1024;

#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

#[derive(Clone)]
pub struct Package {
    pub manifest: Manifest,
    pub files: BTreeMap<String, Vec<u8>>,
    pub fingerprint: String,
    pub source: String,
    pub icon: Option<image::RgbaImage>,
}

impl Package {
    pub fn from_files(files: BTreeMap<String, Vec<u8>>, source: String) -> Result<Self> {
        ensure!(files.len() <= MAX_PACKAGE_FILES, "too many package files");
        ensure!(
            files.values().map(Vec::len).sum::<usize>() <= MAX_PACKAGE_BYTES,
            "package too large"
        );
        for (name, bytes) in &files {
            relative_name(name)?;
            ensure!(bytes.len() <= MAX_FILE_BYTES, "file too large");
        }

        let text = std::str::from_utf8(files.get("plugin.aam").context("missing plugin.aam")?)?;
        let manifest = Manifest::parse(text)?;
        ensure!(files.contains_key(&manifest.entry), "missing entry script");

        let fingerprint = fingerprint(&files);
        let icon = load_icon(&files, &manifest.icon);

        Ok(Self {
            manifest,
            files,
            fingerprint,
            source,
            icon,
        })
    }

    pub fn scripts(&self) -> Result<BTreeMap<String, String>> {
        self.files
            .iter()
            .filter(|(p, _)| p.ends_with(".lua"))
            .map(|(p, b)| Ok((p.clone(), std::str::from_utf8(b)?.to_owned())))
            .collect()
    }
}

fn fingerprint(files: &BTreeMap<String, Vec<u8>>) -> String {
    let mut hash = Sha256::new();

    for (name, bytes) in files {
        hash.update((name.len() as u64).to_le_bytes());
        hash.update(name.as_bytes());
        hash.update((bytes.len() as u64).to_le_bytes());
        hash.update(bytes);
    }

    let digest = hash.finalize();
    let mut out = String::with_capacity(digest.len() * 2);

    for byte in &digest {
        use std::fmt::Write as _;
        write!(&mut out, "{byte:02x}").expect("writing a digest to a String cannot fail");
    }

    out
}

fn load_icon(files: &BTreeMap<String, Vec<u8>>, icon: &str) -> Option<image::RgbaImage> {
    let bytes = files.get(icon)?;

    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;

    let mut limits = image::Limits::default();
    limits.max_image_width = Some(ICON_SOURCE_MAX_EDGE);
    limits.max_image_height = Some(ICON_SOURCE_MAX_EDGE);
    limits.max_alloc = Some(ICON_DECODE_MAX_ALLOC);
    reader.limits(limits);

    let img = reader.decode().ok()?;

    Some(
        img.resize_exact(ICON_SIZE, ICON_SIZE, image::imageops::FilterType::Triangle)
            .to_rgba8(),
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn plugin_roots() -> Vec<PathBuf> {
    let mut roots = vec![PathBuf::from("plugins")];

    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent() {
            roots.push(parent.join("plugins"));
        }

    roots
}

#[cfg(not(target_arch = "wasm32"))]
fn plugin_dirs() -> Vec<PathBuf> {
    use std::collections::BTreeSet;

    let mut seen = BTreeSet::new();
    let mut dirs = Vec::new();

    for root in plugin_roots() {
        let Ok(root) = root.canonicalize() else {
            continue;
        };

        if !seen.insert(root.clone()) {
            continue;
        }

        let Ok(entries) = std::fs::read_dir(&root) else {
            continue;
        };

        let mut children: Vec<PathBuf> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect();
        children.sort();
        dirs.extend(children);
    }

    dirs
}

#[cfg(not(target_arch = "wasm32"))]
pub fn discover() -> (Vec<Package>, Vec<String>) {
    let mut packages = Vec::new();
    let mut errors = Vec::new();

    for dir in plugin_dirs() {
        if packages.len() >= MAX_PLUGINS {
            errors.push("Plugin count limit reached".into());
            break;
        }

        if !dir.join("plugin.aam").is_file() {
            continue;
        }

        match read_package(&dir) {
            Ok(package) => packages.push(package),
            Err(error) => errors.push(format!("{}: {error:#}", dir.display())),
        }
    }

    (packages, errors)
}

#[cfg(not(target_arch = "wasm32"))]
fn read_package(root: &std::path::Path) -> Result<Package> {
    fn walk(
        root: &std::path::Path,
        dir: &std::path::Path,
        out: &mut BTreeMap<String, Vec<u8>>,
        total: &mut u64,
        depth: usize,
    ) -> Result<()> {
        ensure!(depth < MAX_DIR_DEPTH, "package nesting too deep");

        let mut entries: Vec<_> = std::fs::read_dir(dir)?.collect::<std::io::Result<_>>()?;
        entries.sort_by_key(std::fs::DirEntry::file_name);

        for entry in entries {
            let name = entry.file_name();
            let name = name.to_str().context("non-UTF8 package filename")?;

            if name.starts_with('.') || name == "target" {
                continue;
            }

            let ty = entry.file_type()?;
            ensure!(!ty.is_symlink(), "symlinks in plugins are not allowed");

            if ty.is_dir() {
                walk(root, &entry.path(), out, total, depth + 1)?;
            } else if ty.is_file() {
                let path = entry.path();
                let rel = path
                    .strip_prefix(root)?
                    .to_str()
                    .context("invalid path")?
                    .replace('\\', "/");

                if rel != "plugin.aam" && !rel.ends_with(".lua") && !rel.starts_with("assets/") {
                    continue;
                }

                ensure!(out.len() < MAX_PACKAGE_FILES, "too many files");

                let size = entry.metadata()?.len();
                *total += size;
                ensure!(
                    size <= MAX_FILE_BYTES as u64 && *total <= MAX_PACKAGE_BYTES as u64,
                    "package too large"
                );
                ensure!(
                    path.canonicalize()?.starts_with(root),
                    "file escapes package root"
                );

                let mut bytes = Vec::new();
                std::fs::File::open(path)?
                    .take(MAX_FILE_BYTES as u64 + 1)
                    .read_to_end(&mut bytes)?;
                ensure!(bytes.len() <= MAX_FILE_BYTES, "file grew during scan");

                out.insert(rel, bytes);
            }
        }

        Ok(())
    }

    let root = root.canonicalize()?;
    let mut files = BTreeMap::new();
    let mut total = 0;

    walk(&root, &root, &mut files, &mut total, 0)?;
    Package::from_files(files, root.display().to_string())
}

#[cfg(target_arch = "wasm32")]
pub fn discover() -> (Vec<Package>, Vec<String>) {
    let mut out = Vec::new();
    let mut errors = Vec::new();
    macro_rules! bundled {
        ($dir:literal) => {{
            let files = BTreeMap::from([
                (
                    "plugin.aam".into(),
                    include_bytes!(concat!("../../plugins/", $dir, "/plugin.aam")).to_vec(),
                ),
                (
                    "main.lua".into(),
                    include_bytes!(concat!("../../plugins/", $dir, "/main.lua")).to_vec(),
                ),
                (
                    "assets/icon.png".into(),
                    include_bytes!(concat!("../../plugins/", $dir, "/assets/icon.png")).to_vec(),
                ),
            ]);

            match Package::from_files(files, concat!("bundled:", $dir).into()) {
                Ok(p) => out.push(p),
                Err(e) => errors.push(e.to_string()),
            }
        }};
    }

    bundled!("text-editor");
    bundled!("math-snippets");
    bundled!("shapes");

    for stored in super::platform::browser_packages() {
        match Package::from_files(stored, "browser storage".into()) {
            Ok(p) => out.push(p),
            Err(e) => errors.push(e.to_string()),
        }
    }

    (out, errors)
}
