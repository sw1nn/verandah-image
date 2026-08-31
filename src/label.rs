//! Caption composition: a text strip along the bottom edge of a key.

use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
use image::{Pixel, Rgba, RgbaImage};
use imageproc::drawing::draw_text_mut;

/// Strip height as a fraction of the icon's shorter side.
pub const DEFAULT_HEIGHT: f32 = 0.25;
/// Type size as a fraction of the icon's shorter side.
pub const DEFAULT_TEXT_SIZE: f32 = 0.14;
/// Horizontal padding as a fraction of the icon's shorter side.
pub const DEFAULT_PADDING: f32 = 0.04;
/// White caption.
pub const DEFAULT_TEXT_COLOR: Rgba<u8> = Rgba([0xff, 0xff, 0xff, 0xff]);
/// Translucent black strip; the alpha is what lets artwork show through.
pub const DEFAULT_BACKGROUND: Rgba<u8> = Rgba([0x00, 0x00, 0x00, 0xa0]);

/// A caption's geometry and colours, all fractions of the shorter side.
#[derive(Debug, Clone)]
pub struct LabelSpec {
    pub height: f32,
    pub text_size: f32,
    pub padding: f32,
    pub text_color: Rgba<u8>,
    pub background: Rgba<u8>,
}

impl Default for LabelSpec {
    fn default() -> Self {
        Self {
            height: DEFAULT_HEIGHT,
            text_size: DEFAULT_TEXT_SIZE,
            padding: DEFAULT_PADDING,
            text_color: DEFAULT_TEXT_COLOR,
            background: DEFAULT_BACKGROUND,
        }
    }
}

/// Whether the caption was drawn whole.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelFit {
    Fitted,
    Truncated,
}

/// The caption as it will be drawn, clipped to `max_width` with an ellipsis.
///
/// Characters, not bytes: a byte-wise cut would split a multi-byte glyph.
fn fit_caption<F>(font: &F, text: &str, scale: f32, max_width: f32) -> (String, LabelFit)
where
    F: Font,
{
    let width_of = |s: &str| crate::text::measure_text_width(font, s) * scale;

    if width_of(text) <= max_width {
        return (text.to_owned(), LabelFit::Fitted);
    }

    let mut kept = String::new();
    for c in text.chars() {
        let mut candidate = kept.clone();
        candidate.push(c);
        if width_of(&format!("{candidate}…")) > max_width {
            break;
        }
        kept = candidate;
    }

    (format!("{kept}…"), LabelFit::Truncated)
}

/// Compose a caption strip along the bottom of `icon`.
///
/// A blank caption draws nothing, strip included: an empty band over the
/// artwork reads as a rendering fault rather than as an absent label.
pub fn apply_label(icon: &mut RgbaImage, spec: &LabelSpec, text: &str) -> LabelFit {
    if text.trim().is_empty() {
        return LabelFit::Fitted;
    }

    let (width, height) = (icon.width(), icon.height());
    let shorter = width.min(height) as f32;

    let strip_height = (shorter * spec.height).round().max(1.0) as u32;
    let strip_top = height.saturating_sub(strip_height);

    blend_strip(icon, strip_top, spec.background);

    let Some(font_bytes) = crate::font::get_system_monospace_font_for_text(text) else {
        tracing::warn!(
            text,
            "No font covers this caption; strip drawn without text"
        );
        return LabelFit::Fitted;
    };
    let Ok(font) = FontRef::try_from_slice(font_bytes) else {
        return LabelFit::Fitted;
    };

    let scale_value = shorter * spec.text_size;
    let scale = PxScale::from(scale_value);

    let padding = shorter * spec.padding;
    let max_width = (width as f32 - padding * 2.0).max(0.0);
    let (caption, fit) = fit_caption(&font, text, scale_value, max_width);

    let text_width = crate::text::measure_text_width(&font, &caption) * scale_value;
    let line_height = font.as_scaled(scale).height();

    let x = ((width as f32 - text_width) / 2.0).max(0.0) as i32;
    let y = (strip_top as f32 + (strip_height as f32 - line_height) / 2.0).max(0.0) as i32;

    draw_text_mut(icon, spec.text_color, x, y, scale, &font, &caption);
    fit
}

/// Alpha-blend `colour` over every row from `top` to the bottom edge.
///
/// Blended rather than replaced so a translucent background lets the artwork
/// show through, which is the whole point of the strip sitting *over* the icon.
fn blend_strip(icon: &mut RgbaImage, top: u32, colour: Rgba<u8>) {
    let (width, height) = (icon.width(), icon.height());
    for y in top..height {
        for x in 0..width {
            icon.get_pixel_mut(x, y).blend(&colour);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A solid mid-grey canvas, so a blended strip is detectable as a change.
    fn canvas(size: u32) -> RgbaImage {
        RgbaImage::from_pixel(size, size, Rgba([128, 128, 128, 255]))
    }

    /// The strip covers exactly the bottom `height` fraction and nothing above it.
    #[test]
    fn strip_covers_the_bottom_fraction_only() -> Result<(), String> {
        for size in [72u32, 96] {
            let mut icon = canvas(size);
            apply_label(&mut icon, &LabelSpec::default(), "Rotate");

            let strip_top = size - (size as f32 * DEFAULT_HEIGHT).round() as u32;

            // One row above the strip is untouched.
            let above = icon.get_pixel(0, strip_top - 1);
            assert_eq!(
                *above,
                Rgba([128, 128, 128, 255]),
                "row above the strip was modified at size {size}"
            );

            // The strip's leftmost column is darkened (blended, not replaced).
            let inside = icon.get_pixel(0, strip_top + 1);
            assert!(
                inside.0[0] < 128,
                "strip was not blended at size {size}: {inside:?}"
            );
            assert!(
                inside.0[0] > 0,
                "strip replaced rather than blended at size {size}: {inside:?}"
            );
        }
        Ok(())
    }

    /// The caption must actually put ink on the strip.
    #[test]
    fn caption_draws_ink_on_the_strip() -> Result<(), String> {
        let size = 96u32;
        let mut with = canvas(size);
        let mut without = canvas(size);

        apply_label(&mut with, &LabelSpec::default(), "Rotate");
        apply_label(&mut without, &LabelSpec::default(), "");

        assert_ne!(
            with.as_raw(),
            without.as_raw(),
            "a caption drew the same pixels as no caption"
        );
        Ok(())
    }

    /// An empty or blank caption draws nothing at all — no strip. A bare band
    /// over the artwork reads as a rendering fault, not as an empty label.
    #[test]
    fn blank_caption_draws_nothing() -> Result<(), String> {
        for text in ["", "   ", "\t"] {
            let mut icon = canvas(72);
            let before = icon.as_raw().clone();
            apply_label(&mut icon, &LabelSpec::default(), text);
            assert_eq!(
                icon.as_raw(),
                &before,
                "blank caption {text:?} modified the image"
            );
        }
        Ok(())
    }

    /// A caption that fits must be drawn whole, with no ellipsis.
    #[test]
    fn short_caption_is_not_truncated() -> Result<(), String> {
        let mut icon = canvas(96);
        assert!(matches!(
            apply_label(&mut icon, &LabelSpec::default(), "Go"),
            LabelFit::Fitted
        ));
        Ok(())
    }

    /// A caption past the strip's width is truncated rather than shrunk: one
    /// size across a deck is what makes a row of captions scannable.
    #[test]
    fn long_caption_is_truncated() -> Result<(), String> {
        let mut icon = canvas(72);
        assert!(matches!(
            apply_label(
                &mut icon,
                &LabelSpec::default(),
                "Sketcher Select Elements Associated With Constraints"
            ),
            LabelFit::Truncated
        ));
        Ok(())
    }

    /// The boundary itself: the longest caption that fits is untouched, and one
    /// character more truncates. Asserting only the extremes would still pass
    /// with an off-by-many truncation rule.
    #[test]
    fn truncation_boundary_is_one_character_wide() -> Result<(), String> {
        let spec = LabelSpec::default();
        let mut icon = canvas(72);

        let mut longest_fitting = String::new();
        for _ in 0..64 {
            let mut candidate = longest_fitting.clone();
            candidate.push('W');
            let mut probe = canvas(72);
            match apply_label(&mut probe, &spec, &candidate) {
                LabelFit::Fitted => longest_fitting = candidate,
                LabelFit::Truncated => break,
            }
        }

        assert!(
            !longest_fitting.is_empty(),
            "no caption fitted at all, so the boundary test proves nothing"
        );
        assert!(matches!(
            apply_label(&mut icon, &spec, &longest_fitting),
            LabelFit::Fitted
        ));

        let one_more = format!("{longest_fitting}W");
        assert!(matches!(
            apply_label(&mut icon, &spec, &one_more),
            LabelFit::Truncated
        ));
        Ok(())
    }

    /// Truncation must not split a multi-byte character.
    #[test]
    fn truncation_respects_char_boundaries() -> Result<(), String> {
        let mut icon = canvas(72);
        // Every character is multi-byte; a byte-wise truncation would panic.
        let text = "ααααααααααααααααααααααααααααα";
        let _ = apply_label(&mut icon, &LabelSpec::default(), text);
        Ok(())
    }
}
