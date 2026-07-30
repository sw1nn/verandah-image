//! Font loading utilities.
//!
//! Provides cached access to system fonts via fontconfig.

use std::sync::OnceLock;

static SYSTEM_FONT: OnceLock<Option<Vec<u8>>> = OnceLock::new();

/// Get the system monospace font, cached for reuse.
///
/// Returns `None` if no monospace font could be found.
pub fn get_system_monospace_font() -> Option<&'static Vec<u8>> {
    SYSTEM_FONT.get_or_init(load_system_monospace_font).as_ref()
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
        // First call loads the font
        let font1 = get_system_monospace_font();
        // Second call returns cached value
        let font2 = get_system_monospace_font();

        // Both should point to the same data
        match (font1, font2) {
            (Some(f1), Some(f2)) => assert!(std::ptr::eq(f1, f2)),
            (None, None) => {} // OK if no font available
            _ => panic!("Inconsistent font caching"),
        }
    }
}
