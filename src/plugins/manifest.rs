use std::collections::BTreeSet;
use aam_rs::from_aam::FromAam;
use anyhow::{bail, ensure, Result};
use semver::{Version, VersionReq};

pub const HOST_API: &str = "1.0.0";
pub const PERMISSIONS: &[&str] = &[
    "document.read",
    "document.write",
    "plugins.call",
    "clipboard.read",
    "clipboard.write",
];

#[derive(Debug, Clone, aam_rs::FromAam)]
pub struct Dependency {
    pub id: String,
    pub version: String,
    pub repository: String,
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

pub fn relative_name(name: &str) -> Result<()> {
    ensure!(!name.is_empty() && name.len() < 256, "invalid package path");
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

// Already high complexity...  TODO!!!
fn data_only(text: &str) -> Result<()> {
    ensure!(text.len() <= 65536, "manifest too large");
    let mut quote = None;
    let mut escape = false;
    let mut comment = false;
    let mut depth = 0usize;
    for ch in text.chars() {
        if comment {
            if ch == '\n' {
                comment = false;
            }
            continue;
        }
        if let Some(q) = quote {
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == q {
                quote = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' => quote = Some(ch),
            '#' => comment = true,
            '@' => bail!("AAM directives are forbidden in plugin metadata"),
            '{' | '[' => {
                depth += 1;
                ensure!(depth <= 16, "manifest nesting too deep");
            }
            '}' | ']' => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| anyhow::anyhow!("unbalanced manifest"))?;
            }
            _ => {}
        }
    }
    ensure!(quote.is_none() && depth == 0, "unterminated manifest");
    Ok(())
}

impl Manifest {
    pub fn parse(text: &str) -> Result<Self> {
        data_only(text)?;
        let m = Self::from_aam_str(text).map_err(|e| anyhow::anyhow!("invalid manifest: {}", e))?;
        ensure!(valid_id(&m.id) && m.id.contains('.'), "invalid plugin ID");
        ensure!(!m.name.trim().is_empty() && m.name.len() <= 128, "invalid name");
        ensure!(m.description.len() <= 4096 && m.authors.len() <= 32, "metadata too large");
        Version::parse(&m.version)?;
        VersionReq::parse(&m.api)?;
        repository(&m.repository)?;
        relative_name(&m.entry)?;
        relative_name(&m.icon)?;
        ensure!(m.entry.ends_with(".lua"), "entry must be a .rhai file");
        let mut deps = BTreeSet::new();
        for dep in &m.dependencies {
            ensure!(valid_id(&dep.id) && dep.id != m.id && deps.insert(dep.id.clone()), "invalid or duplicate dependency");
            VersionReq::parse(&dep.version)?;
            repository(&dep.repository)?;
        }
        let mut perms = BTreeSet::new();
        for p in &m.permissions {
            ensure!(PERMISSIONS.contains(&p.as_str()) && perms.insert(p.clone()), "unknown or duplicate permission: {p}");
        }
        Ok(m)
    }
    pub fn permits(&self, permission: &str) -> bool {
        self.permissions.iter().any(|p| p == permission)
    }
}


pub fn valid_id(s: &str) -> bool {
    !s.is_empty() && s.len() <= 128 && s.bytes().all(|c| c.is_ascii_alphanumeric() || b".-_".contains(&c))
}

fn repository(s: &str) -> Result<()> {
    ensure!(s.starts_with("https://") && s.len() < 2048 && !s.chars().any(char::is_control), "repository must be a HTTPS URL");
    Ok(())
}