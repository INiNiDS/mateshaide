use crate::plugins::manifest::{Manifest, relative_name};
use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::str::from_utf8;

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
        ensure!(files.len() <= 256, "too many package files");
        ensure!(
            files.values().map(Vec::len).sum::<usize>() <= 8 * 1024 * 1024,
            "package too large"
        );
        for (name, bytes) in &files {
            relative_name(name)?;
            ensure!(bytes.len() <= 2 * 1024 * 1024, "file too large: {}", name);
        }
        let text = from_utf8(files.get("plugin.aam").context("missing plugin.aam")?)?;
        let manifest = Manifest::parse(text)?;
        ensure!(
            files.contains_key(&manifest.entry),
            "missing entry file: {}",
            manifest.entry
        );
        let mut hash = Sha256::new();
        for (name, bytes) in &files {
            Digest::update(&mut hash, (name.len() as u64).to_le_bytes());
            Digest::update(&mut hash, name.as_bytes());
            Digest::update(&mut hash, (bytes.len() as u64).to_le_bytes());
            Digest::update(&mut hash, bytes);
        }
        let fingerprint = format!("{:x}", hash.finalize());
        let icon = files.get(&manifest.icon).and_then(|bytes| {
            let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
                .with_guessed_format()
                .ok()?;
            let mut limits = image::Limits::default();
            limits.max_image_width = Some(512);
            limits.max_image_height = Some(512);
            limits.max_alloc = Some(4 * 1024 * 1024);
            reader.limits(limits);
            let img = reader.decode().ok()?;
            Some(
                img.resize_exact(24, 24, image::imageops::FilterType::Triangle)
                    .to_rgba8(),
            )
        });
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
            .filter(|(p, _)| p.ends_with(".rhai"))
            .map(|(p, b)| Ok((p.clone(), from_utf8(b)?.to_string())))
            .collect()
    }
}

// High Complexity too... TODO
#[cfg(not(target_arch = "wasm32"))]
pub fn discover() -> (Vec<Package>, Vec<String>) {
    use std::path::PathBuf;
    let mut roots = vec![PathBuf::from("plugins")];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            roots.push(parent.join("plugins"));
        }
    }
    let mut packages = Vec::new();
    let mut errors = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for root in roots {
        let Ok(root) = root.canonicalize() else {
            continue;
        };
        if !seen.insert(root.clone()) {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&root) else {
            continue;
        };
        let mut dirs: Vec<_> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        dirs.sort();
        for dir in dirs {
            if packages.len() >= 128 {
                errors.push("Plugin count limit reached".into());
                break;
            }
            if !dir.join("plugin.aam").is_file() {
                continue;
            }
            match read_package(&dir) {
                Ok(p) => packages.push(p),
                Err(e) => errors.push(format!("{}: {e:#}", dir.display())),
            }
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
        ensure!(depth < 12, "package nesting too deep");
        let mut entries: Vec<_> = std::fs::read_dir(dir)?.collect::<std::io::Result<_>>()?;
        entries.sort_by_key(|e| e.file_name());
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
                if rel != "plugin.aam" && !rel.ends_with(".rhai") && !rel.starts_with("assets/") {
                    continue;
                }
                ensure!(out.len() < 256, "too many files");
                let size = entry.metadata()?.len();
                *total += size;
                ensure!(
                    size <= 2 * 1024 * 1024 && *total <= 8 * 1024 * 1024,
                    "package too large"
                );
                // Canonical containment is checked too; execution only uses captured bytes.
                ensure!(
                    path.canonicalize()?.starts_with(root),
                    "file escapes package root"
                );
                use std::io::Read as _;
                let mut bytes = Vec::new();
                std::fs::File::open(path)?
                    .take(2 * 1024 * 1024 + 1)
                    .read_to_end(&mut bytes)?;
                ensure!(bytes.len() <= 2 * 1024 * 1024, "file grew during scan");
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
                    "main.rhai".into(),
                    include_bytes!(concat!("../../plugins/", $dir, "/main.rhai")).to_vec(),
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
