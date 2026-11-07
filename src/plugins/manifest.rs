use aam_rs::from_aam::FromAam;
use anyhow::{Result, bail, ensure};
use semver::{Version, VersionReq};
use std::collections::BTreeSet;
use std::path::Path;

pub const HOST_API: &str = "1.0.0";
pub const PERMISSIONS: &[&str] = &[
    "document.read",
    "document.write",
    "plugins.call",
    "clipboard.read",
    "clipboard.write",
    "files.pick",
    "help.read"
];

const MAX_MANIFEST_BYTES: usize = 65_536;
const MAX_NESTING: usize = 16;
const MAX_ID_BYTES: usize = 128;
const MAX_NAME_BYTES: usize = 128;
const MAX_DESCRIPTION_BYTES: usize = 4096;
const MAX_AUTHORS: usize = 32;
const MAX_PATH_BYTES: usize = 256;
const MAX_REPOSITORY_BYTES: usize = 2048;

#[derive(Debug, Clone, aam_rs::FromAam)]
pub struct Dependency {
    pub id: String,
    pub version: String,
    pub repository: String,
}

impl Dependency {
    fn normalize(&mut self) {
        document_string(&mut self.id);
        document_string(&mut self.version);
        document_string(&mut self.repository);
    }
}

#[derive(Debug, Clone, aam_rs::FromAam)]
pub struct Manifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub authors: Vec<String>,
    pub repository: String,
    pub icon: String,
    pub entry: String,
    pub api: String,
    pub permissions: Vec<String>,
    pub dependencies: Vec<Dependency>,
}

impl Manifest {
    pub fn parse(text: &str) -> Result<Self> {
        data_only(text)?;

        let mut manifest =
            Self::from_aam_str(text).map_err(|e| anyhow::anyhow!("invalid manifest: {e}"))?;

        manifest.normalize();
        manifest.validate()?;

        Ok(manifest)
    }

    pub fn permits(&self, permission: &str) -> bool {
        self.permissions.iter().any(|p| p == permission)
    }

    fn normalize(&mut self) {
        document_string(&mut self.id);
        document_string(&mut self.name);
        document_string(&mut self.version);
        document_string(&mut self.description);
        document_string(&mut self.repository);
        document_string(&mut self.icon);
        document_string(&mut self.entry);
        document_string(&mut self.api);

        for author in &mut self.authors {
            document_string(author);
        }
        for perm in &mut self.permissions {
            document_string(perm);
        }
        for dep in &mut self.dependencies {
            dep.normalize();
        }
    }

    fn validate(&self) -> Result<()> {
        ensure!(
            valid_id(&self.id) && self.id.contains('.'),
            "invalid plugin ID"
        );
        ensure!(
            !self.name.trim().is_empty() && self.name.len() <= MAX_NAME_BYTES,
            "invalid name"
        );
        ensure!(
            self.description.len() <= MAX_DESCRIPTION_BYTES && self.authors.len() <= MAX_AUTHORS,
            "metadata too large"
        );

        Version::parse(&self.version)?;
        VersionReq::parse(&self.api)?;
        repository(&self.repository)?;
        relative_name(&self.entry)?;
        relative_name(&self.icon)?;

        ensure!(
            Path::new(&self.entry)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("lua")),
            "entry must be a .lua file"
        );

        let mut deps = BTreeSet::new();
        for dep in &self.dependencies {
            ensure!(
                valid_id(&dep.id) && dep.id != self.id && deps.insert(dep.id.clone()),
                "invalid or duplicate dependency"
            );
            VersionReq::parse(&dep.version)?;
            repository(&dep.repository)?;
        }

        let mut perms = BTreeSet::new();
        for p in &self.permissions {
            ensure!(
                PERMISSIONS.contains(&p.as_str()) && perms.insert(p.clone()),
                "unknown or duplicate permission: {p}"
            );
        }

        Ok(())
    }
}

pub fn relative_name(name: &str) -> Result<()> {
    ensure!(
        !name.is_empty() && name.len() < MAX_PATH_BYTES,
        "invalid package path"
    );
    ensure!(
        !name.contains('\\') && !name.contains(':') && !name.starts_with('/'),
        "absolute path forbidden"
    );
    ensure!(
        name.split('/')
            .all(|p| !p.is_empty() && p != "." && p != ".."),
        "path traversal forbidden"
    );
    Ok(())
}

fn data_only(text: &str) -> Result<()> {
    ensure!(text.len() <= MAX_MANIFEST_BYTES, "manifest too large");

    let mut scanner = DataScanner::default();

    for ch in text.chars() {
        scanner.feed(ch)?;
    }

    scanner.finish()
}

#[derive(Default)]
struct DataScanner {
    quote: Option<char>,
    escape: bool,
    comment: bool,
    depth: usize,
}

impl DataScanner {
    fn feed(&mut self, ch: char) -> Result<()> {
        if self.comment {
            self.comment = ch != '\n';
            return Ok(());
        }

        if let Some(quote) = self.quote {
            if self.escape {
                self.escape = false;
            } else if ch == '\\' {
                self.escape = true;
            } else if ch == quote {
                self.quote = None;
            }
            return Ok(());
        }

        match ch {
            '"' | '\'' => self.quote = Some(ch),
            '#' => self.comment = true,
            '@' => bail!("AAM directives are forbidden in plugin metadata"),
            '{' | '[' => {
                self.depth += 1;
                ensure!(self.depth <= MAX_NESTING, "manifest nesting too deep");
            }
            '}' | ']' => {
                self.depth = self
                    .depth
                    .checked_sub(1)
                    .ok_or_else(|| anyhow::anyhow!("unbalanced manifest"))?;
            }
            _ => {}
        }

        Ok(())
    }

    fn finish(&self) -> Result<()> {
        ensure!(
            self.quote.is_none() && self.depth == 0,
            "unterminated manifest"
        );
        Ok(())
    }
}

pub fn valid_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= MAX_ID_BYTES
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || b".-_".contains(&c))
}

fn repository(s: &str) -> Result<()> {
    ensure!(
        s.starts_with("https://")
            && s.len() < MAX_REPOSITORY_BYTES
            && !s.chars().any(char::is_control),
        "repository must be a HTTPS URL"
    );
    Ok(())
}

fn document_string(value: &mut String) {
    let trimmed = value.trim();

    for quote in ['"', '\''] {
        if let Some(inner) = trimmed
            .strip_prefix(quote)
            .and_then(|s| s.strip_suffix(quote))
        {
            *value = inner.to_owned();
            return;
        }
    }

    if trimmed.len() != value.len() {
        *value = trimmed.to_owned();
    }
}
