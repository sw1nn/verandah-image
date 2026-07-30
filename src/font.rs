//! Font loading utilities.
//!
//! Provides cached access to system fonts via fontconfig.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

static SYSTEM_FONT: OnceLock<Option<Vec<u8>>> = OnceLock::new();

type FontCache = Mutex<HashMap<(String, Option<String>), Option<&'static [u8]>>>;

/// Cache of faces already resolved, keyed by family and style.
fn font_cache() -> &'static FontCache {
    static CACHE: OnceLock<FontCache> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Get the system monospace font, cached for reuse.
///
/// Returns `None` if no monospace font could be found.
pub fn get_system_monospace_font() -> Option<&'static Vec<u8>> {
    SYSTEM_FONT.get_or_init(load_system_monospace_font).as_ref()
}

/// Get a system font by family and optional style, cached for reuse.
///
/// `family` is a fontconfig family name or generic alias such as `sans-serif`;
/// `style` is a face name such as `Bold`. Returns `None` if fontconfig is
/// unavailable or the face could not be read.
///
/// The bytes are leaked deliberately to keep the `'static` lifetime callers
/// need for `ab_glyph::FontRef`. The number of distinct faces requested over a
/// process lifetime is a handful.
pub fn get_system_font(family: &str, style: Option<&str>) -> Option<&'static [u8]> {
    let key = (family.to_owned(), style.map(str::to_owned));

    let mut cache = font_cache().lock().ok()?;
    if let Some(hit) = cache.get(&key) {
        return *hit;
    }

    let loaded = load_font(family, style);
    cache.insert(key, loaded);
    loaded
}

/// Resolve a family and style through fontconfig and read the file.
fn load_font(family: &str, style: Option<&str>) -> Option<&'static [u8]> {
    use fontconfig::Fontconfig;

    let fc = Fontconfig::new()?;
    let font = fc.find(family, style)?;
    let path = font.path.to_string_lossy().into_owned();

    match std::fs::read(&path) {
        Ok(bytes) => {
            tracing::debug!(family, ?style, path, "Loaded system font via fontconfig");
            Some(Box::leak(bytes.into_boxed_slice()))
        }
        Err(error) => {
            tracing::warn!(family, ?style, path, %error, "Failed to read system font");
            None
        }
    }
}

/// Load the system monospace font via fontconfig.
fn load_system_monospace_font() -> Option<Vec<u8>> {
    use fontconfig::Fontconfig;

    let fc = Fontconfig::new()?;
    if let Some(font) = fc.find("monospace", None) {
        let path = font.path.to_string_lossy();
        if let Ok(bytes) = std::fs::read(&*path) {
            return Some(bytes);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_system_monospace_font_cached() {
        let font1 = get_system_monospace_font();
        let font2 = get_system_monospace_font();

        match (font1, font2) {
            (Some(f1), Some(f2)) => assert!(std::ptr::eq(f1, f2)),
            (None, None) => {}
            _ => panic!("Inconsistent font caching"),
        }
    }

    #[test]
    fn get_system_font_returns_same_slice_for_repeat_requests() {
        let a = get_system_font("sans-serif", Some("Bold"));
        let b = get_system_font("sans-serif", Some("Bold"));

        match (a, b) {
            (Some(a), Some(b)) => assert!(std::ptr::eq(a.as_ptr(), b.as_ptr())),
            (None, None) => {} // no fontconfig or no such face in this environment
            _ => panic!("Inconsistent font caching"),
        }
    }

    /// `style` must be part of the cache key. Asserting only that two calls agree
    /// on `is_some()` would still pass if the key dropped `style` and one style's
    /// bytes were served for the other's.
    #[test]
    fn get_system_font_keys_the_cache_on_style() -> Result<(), String> {
        let _ = get_system_font("DejaVu Sans", Some("Bold"));
        let _ = get_system_font("DejaVu Sans", None);

        let cache = font_cache().lock().map_err(|e| format!("cache lock poisoned: {e}"))?;
        assert!(
            cache.contains_key(&("DejaVu Sans".to_owned(), Some("Bold".to_owned()))),
            "bold request did not create its own cache entry"
        );
        assert!(
            cache.contains_key(&("DejaVu Sans".to_owned(), None)),
            "unstyled request did not create its own cache entry"
        );
        Ok(())
    }

    /// The badge draws with DejaVu Sans Bold specifically — `sans-serif:Bold`
    /// resolves to Liberation Sans Bold on a typical Arch box, whose digits do not
    /// match the existing artwork.
    #[test]
    fn get_system_font_finds_dejavu_sans_bold() -> Result<(), String> {
        let bytes = get_system_font("DejaVu Sans", Some("Bold"))
            .ok_or("DejaVu Sans Bold must be installed: the badge is drawn with it")?;
        assert!(!bytes.is_empty(), "font file was empty");
        Ok(())
    }
}
