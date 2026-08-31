//! Font loading utilities.
//!
//! Provides cached access to system fonts via fontconfig.
//!
//! Selection is coverage aware: the characters a caller is about to draw are
//! handed to fontconfig as an `FcCharSet`, which fontconfig scores above the
//! family name. A face that covers the text therefore wins over the requested
//! family when the requested family has no glyph for it, while text the
//! requested family does cover still resolves to that family. Without this a
//! Nerd Font or emoji codepoint drew as `.notdef` — an empty box — with nothing
//! in the log to say why.

use std::collections::{BTreeSet, HashMap};
use std::ffi::CString;
use std::sync::OnceLock;

use ab_glyph::{Font, FontRef};
use parking_lot::Mutex;

/// Cache key: family, style, and the characters the face must cover.
///
/// The coverage component is the distinct non-blank characters in order, so
/// `"88"`, `"8"` and `"8 8"` share a single entry.
type FontKey = (String, Option<String>, String);

type FontCache = Mutex<HashMap<FontKey, Option<&'static [u8]>>>;

/// Faces already read, keyed by the file fontconfig resolved to.
type FaceCache = Mutex<HashMap<String, Option<&'static [u8]>>>;

/// Cache of requests already resolved, keyed by family, style and coverage.
fn font_cache() -> &'static FontCache {
    static CACHE: OnceLock<FontCache> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Cache of faces already read, keyed by the file fontconfig resolved to.
///
/// Distinct requests routinely land on the same file — every digit badge asks for
/// a different coverage set and gets DejaVu Sans Bold — so without this the same
/// megabyte would be read and leaked once per distinct mark. It also makes the
/// leaked slice a stable identity for a face, which the tests rely on.
fn face_cache() -> &'static FaceCache {
    static CACHE: OnceLock<FaceCache> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Get the system monospace font, cached for reuse.
///
/// Prefer [`get_system_monospace_font_for_text`] when the text to be drawn is
/// known: this function asks for no particular coverage, so a face without the
/// glyphs the caller needs is a valid answer.
pub fn get_system_monospace_font() -> Option<&'static [u8]> {
    get_system_font("monospace", None)
}

/// Get a system monospace font that covers `text`, cached for reuse.
pub fn get_system_monospace_font_for_text(text: &str) -> Option<&'static [u8]> {
    get_system_font_for_text("monospace", None, text)
}

/// Get a system font by family and optional style, cached for reuse.
///
/// `family` is a fontconfig family name or generic alias such as `sans-serif`;
/// `style` is a face name such as `Bold`. Returns `None` if fontconfig is
/// unavailable, or the face could not be read or parsed.
///
/// Prefer [`get_system_font_for_text`] when the text to be drawn is known.
pub fn get_system_font(family: &str, style: Option<&str>) -> Option<&'static [u8]> {
    get_system_font_for_text(family, style, "")
}

/// Get a system font covering `text`, preferring `family` and `style`.
///
/// The bytes are leaked deliberately to keep the `'static` lifetime callers
/// need for `ab_glyph::FontRef`. The number of distinct faces requested over a
/// process lifetime is a handful: the cache is keyed on the set of characters
/// rather than on the string, so faces are shared across every mark and label
/// needing the same coverage.
///
/// `parking_lot::Mutex` rather than `std::sync::Mutex`: it does not poison, so a
/// panic elsewhere while this lock is held cannot make every future lookup return
/// `None` for the rest of the process's life — the badge draws with this cache on
/// every render.
///
/// The lock is dropped for the fontconfig lookup and file read, so a slow first
/// lookup for one face does not block a concurrent lookup for another; the cache is
/// only re-locked, briefly, to record the result.
pub fn get_system_font_for_text(
    family: &str,
    style: Option<&str>,
    text: &str,
) -> Option<&'static [u8]> {
    let coverage = coverage_of(text);
    let key = (
        family.to_owned(),
        style.map(str::to_owned),
        coverage.iter().collect::<String>(),
    );

    if let Some(hit) = font_cache().lock().get(&key) {
        return *hit;
    }

    let loaded = load_font(family, style, &coverage);

    // Another thread may have raced this one and already inserted for the same
    // key while the lock above was dropped. Return whatever the cache settles on
    // rather than this thread's own answer, so every caller holding the same key
    // holds the same slice.
    *font_cache().lock().entry(key).or_insert(loaded)
}

/// The distinct characters a face must have a glyph for to draw `text`.
///
/// Blanks and control characters are dropped: no font's cmap covers `\n`, so
/// asking fontconfig for it would penalise every candidate equally and would make
/// the missing-glyph warning fire on text that renders perfectly well.
fn coverage_of(text: &str) -> BTreeSet<char> {
    text.chars()
        .filter(|c| !c.is_whitespace() && !c.is_control())
        .collect()
}

/// Resolve a family, style and coverage through fontconfig and read the file.
fn load_font(
    family: &str,
    style: Option<&str>,
    coverage: &BTreeSet<char>,
) -> Option<&'static [u8]> {
    use fontconfig::{CharSet, FC_FAMILY, FC_STYLE, Fontconfig, Pattern};

    let fc = Fontconfig::new()?;

    let mut pattern = Pattern::new(&fc);
    pattern.add_string(FC_FAMILY, &CString::new(family).ok()?);
    if let Some(style) = style {
        pattern.add_string(FC_STYLE, &CString::new(style).ok()?);
    }
    if !coverage.is_empty() {
        let mut char_set = CharSet::new(&fc);
        for &c in coverage {
            char_set.add_char(c);
        }
        pattern.add_charset(char_set);
    }

    let matched = pattern.font_match();
    let name = matched.name()?.to_owned();
    let path = matched.filename()?.to_owned();

    let bytes = read_face(&name, &path)?;

    // The face is shared across coverage sets, but whether it covers *this* text
    // is a property of the request, so it is checked here rather than in `read_face`.
    if let Ok(font) = FontRef::try_from_slice(bytes) {
        warn_missing_glyphs(&font, &name, &path, coverage);
    }

    tracing::debug!(
        family,
        ?style,
        resolved = name,
        path,
        "Resolved system font"
    );
    Some(bytes)
}

/// Read and leak a face by path, once per file.
fn read_face(name: &str, path: &str) -> Option<&'static [u8]> {
    if let Some(hit) = face_cache().lock().get(path) {
        return *hit;
    }

    let loaded = read_face_uncached(name, path);

    *face_cache().lock().entry(path.to_owned()).or_insert(loaded)
}

fn read_face_uncached(name: &str, path: &str) -> Option<&'static [u8]> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            tracing::warn!(font = name, path, %error, "Failed to read system font");
            return None;
        }
    };
    let bytes: &'static [u8] = Box::leak(bytes.into_boxed_slice());

    // A face that will not parse is no face: returning `None` lets a caller's own
    // fallback take over instead of handing back bytes it can only reject.
    if FontRef::try_from_slice(bytes).is_err() {
        tracing::warn!(font = name, path, "System font could not be parsed");
        return None;
    }

    tracing::info!(font = name, path, "Drawing with system font");
    Some(bytes)
}

/// Warn about characters the resolved face still has no glyph for.
///
/// Reached when no installed font covers the character. `draw_text_mut` renders
/// those as `.notdef`, which in most faces is an empty box, so without this the
/// only symptom is tofu on the key.
fn warn_missing_glyphs(font: &FontRef<'_>, name: &str, path: &str, coverage: &BTreeSet<char>) {
    let missing = missing_glyphs(font, coverage);

    if !missing.is_empty() {
        tracing::warn!(
            resolved = name,
            path,
            missing = missing.join(" "),
            "No installed font has a glyph for these characters; they draw as an empty box"
        );
    }
}

/// The characters of `coverage` that `font` maps to `.notdef`, as `U+XXXX`.
fn missing_glyphs<F>(font: &F, coverage: &BTreeSet<char>) -> Vec<String>
where
    F: Font,
{
    coverage
        .iter()
        .copied()
        .filter(|&c| font.glyph_id(c).0 == 0)
        .map(|c| format!("U+{:04X}", c as u32))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A Nerd Font wrench, in the private use area.
    const WRENCH: char = '\u{f0ad}';

    #[test]
    fn test_get_system_monospace_font_cached() {
        let font1 = get_system_monospace_font();
        let font2 = get_system_monospace_font();

        match (font1, font2) {
            (Some(f1), Some(f2)) => assert!(std::ptr::eq(f1.as_ptr(), f2.as_ptr())),
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

        let cache = font_cache().lock();
        assert!(
            cache.contains_key(&(
                "DejaVu Sans".to_owned(),
                Some("Bold".to_owned()),
                String::new()
            )),
            "bold request did not create its own cache entry"
        );
        assert!(
            cache.contains_key(&("DejaVu Sans".to_owned(), None, String::new())),
            "unstyled request did not create its own cache entry"
        );
        Ok(())
    }

    /// Coverage must be part of the cache key too: without it the first face
    /// resolved for a family would be served for every later string, and the
    /// fallback would never be reached a second time.
    #[test]
    fn get_system_font_keys_the_cache_on_coverage() -> Result<(), String> {
        let _ = get_system_font_for_text("DejaVu Sans", Some("Bold"), "8");
        let _ = get_system_font_for_text("DejaVu Sans", Some("Bold"), &WRENCH.to_string());

        let cache = font_cache().lock();
        assert!(
            cache.contains_key(&(
                "DejaVu Sans".to_owned(),
                Some("Bold".to_owned()),
                "8".to_owned()
            )),
            "digit request did not create its own cache entry"
        );
        assert!(
            cache.contains_key(&(
                "DejaVu Sans".to_owned(),
                Some("Bold".to_owned()),
                WRENCH.to_string()
            )),
            "wrench request did not create its own cache entry"
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

    /// The point of the module: asking for a family that cannot draw the text
    /// yields a face that can.
    #[test]
    fn coverage_wins_over_family_when_the_family_cannot_draw_the_text() -> Result<(), String> {
        let bytes = get_system_font_for_text("DejaVu Sans", Some("Bold"), &WRENCH.to_string())
            .ok_or("no face resolved for U+F0AD")?;
        let font =
            FontRef::try_from_slice(bytes).map_err(|e| format!("face failed to parse: {e}"))?;

        assert_ne!(
            font.glyph_id(WRENCH).0,
            0,
            "resolved face has no glyph for U+F0AD"
        );
        Ok(())
    }

    /// ...and asking for a family that *can* draw the text still yields that
    /// family. Coverage must not quietly move well-covered text off its face.
    #[test]
    fn coverage_leaves_the_family_alone_when_it_covers_the_text() -> Result<(), String> {
        let plain = get_system_font("DejaVu Sans", Some("Bold"))
            .ok_or("DejaVu Sans Bold must be installed")?;
        let covered = get_system_font_for_text("DejaVu Sans", Some("Bold"), "0123456789")
            .ok_or("no face resolved for digits")?;

        assert!(
            std::ptr::eq(plain.as_ptr(), covered.as_ptr()),
            "digits resolved away from DejaVu Sans Bold"
        );
        Ok(())
    }

    /// Coverage is a set, not a string: the cache must not grow an entry per
    /// distinct label drawn from the same characters.
    #[test]
    fn coverage_key_ignores_order_repeats_and_blanks() {
        assert_eq!(coverage_of("ab"), coverage_of("b a b\n"));
        assert!(coverage_of(" \t\n").is_empty());
    }

    /// When no installed font covers the character there is nothing left to fall
    /// back to, so the only thing that keeps the blank key diagnosable is naming
    /// the codepoint in the log.
    #[test]
    fn missing_glyphs_names_the_uncovered_codepoints() -> Result<(), String> {
        // An unassigned plane-16 codepoint: no font has a glyph for it.
        const UNASSIGNED: char = '\u{10fffd}';

        let bytes = get_system_monospace_font().ok_or("no monospace font installed")?;
        let font =
            FontRef::try_from_slice(bytes).map_err(|e| format!("face failed to parse: {e}"))?;

        assert_eq!(
            missing_glyphs(&font, &coverage_of("a")),
            Vec::<String>::new(),
            "a covered character was reported missing"
        );
        assert_eq!(
            missing_glyphs(&font, &coverage_of(&UNASSIGNED.to_string())),
            vec!["U+10FFFD".to_owned()]
        );
        Ok(())
    }
}
