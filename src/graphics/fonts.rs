use anyhow::{Context, Result, ensure};
use glyphon::{FontSystem, fontdb};
use std::{cell::RefCell, sync::Arc};

pub const DEFAULT_FAMILY: &str = "Roboto";
pub const DEFAULT_FONT_BYTES: &[u8] = include_bytes!("../../assets/Roboto-Black.ttf");

pub const MAX_FONT_BYTES: usize = 8 * 1024 * 1024;
const MAX_FONTS: usize = 32;
const MAX_FAMILY_BYTES: usize = 64;

type FontData = Arc<dyn AsRef<[u8]> + Send + Sync>;

struct Font {
    family: String,
    data: FontData,
}

#[derive(Default)]
struct Registry {
    fonts: Vec<Font>,
}

thread_local! {
    static REGISTRY: RefCell<Registry> = RefCell::new(Registry::default());
}

pub fn register(bytes: Vec<u8>) -> Result<String> {
    ensure!(
        !bytes.is_empty() && bytes.len() <= MAX_FONT_BYTES,
        "font file too large"
    );

    let data: FontData = Arc::new(bytes);
    let mut db = fontdb::Database::new();
    db.load_font_source(fontdb::Source::Binary(Arc::clone(&data)));
    let family = db
        .faces()
        .find_map(|face| {
            face.families
                .first()
                .map(|(name, _)| name.trim().to_owned())
        })
        .filter(|name| !name.is_empty())
        .context("not a TrueType/OpenType font")?;
    ensure!(
        family.len() <= MAX_FAMILY_BYTES && !family.chars().any(char::is_control),
        "invalid font family name"
    );

    if family == DEFAULT_FAMILY {
        return Ok(family);
    }

    REGISTRY.with_borrow_mut(|registry| {
        if registry.fonts.iter().any(|f| f.family == family) {
            return Ok(family);
        }
        ensure!(registry.fonts.len() < MAX_FONTS, "font limit reached");
        registry.fonts.push(Font {
            family: family.clone(),
            data,
        });
        Ok(family)
    })
}

#[must_use]
pub fn families() -> Vec<String> {
    REGISTRY.with_borrow(|registry| {
        std::iter::once(DEFAULT_FAMILY.to_owned())
            .chain(registry.fonts.iter().map(|f| f.family.clone()))
            .collect()
    })
}
#[must_use]
pub fn resolve(family: Option<&str>) -> String {
    match family {
        Some(name) if name != DEFAULT_FAMILY => REGISTRY.with_borrow(|registry| {
            if registry.fonts.iter().any(|f| f.family == name) {
                name.to_owned()
            } else {
                DEFAULT_FAMILY.to_owned()
            }
        }),
        _ => DEFAULT_FAMILY.to_owned(),
    }
}

#[must_use]
pub fn font_system(loaded: &mut usize) -> FontSystem {
    let mut system = FontSystem::new();
    system.db_mut().load_font_data(DEFAULT_FONT_BYTES.to_vec());
    *loaded = 0;
    sync(&mut system, loaded);
    system
}

pub fn sync(system: &mut FontSystem, loaded: &mut usize) {
    REGISTRY.with_borrow(|registry| {
        for font in registry.fonts.iter().skip(*loaded) {
            system
                .db_mut()
                .load_font_source(fontdb::Source::Binary(Arc::clone(&font.data)));
        }
        *loaded = registry.fonts.len();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_font_is_always_available() {
        assert_eq!(
            register(DEFAULT_FONT_BYTES.to_vec()).unwrap(),
            DEFAULT_FAMILY
        );
        assert_eq!(families()[0], DEFAULT_FAMILY);
        assert_eq!(resolve(None), DEFAULT_FAMILY);
        assert_eq!(resolve(Some("No Such Font")), DEFAULT_FAMILY);
    }

    #[test]
    fn garbage_is_rejected() {
        assert!(register(b"definitely not a font".to_vec()).is_err());
        assert!(register(Vec::new()).is_err());
        assert_eq!(families(), vec![DEFAULT_FAMILY.to_owned()]);
    }
}
