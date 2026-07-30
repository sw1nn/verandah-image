//! Text rendering utilities.
//!
//! Provides functions for measuring and drawing text on images.

use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
use image::{Rgba, RgbaImage};
use imageproc::drawing::draw_text_mut;

use crate::font::get_system_monospace_font;

/// Calculate the width of a line of text using actual font metrics.
pub fn measure_text_width<F>(font: &F, text: &str) -> f32
where
    F: Font,
{
    let scaled = font.as_scaled(PxScale::from(1.0));
    text.chars()
        .map(|c| scaled.h_advance(font.glyph_id(c)))
        .sum()
}

/// Find optimal font scale to fit text within target dimensions.
///
/// Returns a scale value that will make the text fit within the given
/// width and height constraints, clamped between 8.0 and 96.0.
pub fn find_optimal_scale<F>(font: &F, lines: &[&str], target_width: f32, target_height: f32) -> f32
where
    F: Font,
{
    let num_lines = lines.len().max(1) as f32;

    let max_line_width = lines
        .iter()
        .map(|line| measure_text_width(font, line))
        .fold(0.0_f32, |a, b| a.max(b));

    let scaled = font.as_scaled(PxScale::from(1.0));
    let line_height = scaled.height();

    let scale_for_width = if max_line_width > 0.0 {
        target_width / max_line_width
    } else {
        target_height
    };

    let total_height_at_1 = num_lines * line_height;
    let scale_for_height = if total_height_at_1 > 0.0 {
        target_height / total_height_at_1
    } else {
        target_width
    };

    scale_for_width.min(scale_for_height).clamp(8.0, 96.0)
}

/// Draw text centered on an image.
///
/// # Arguments
/// * `rgba` - The image to draw on
/// * `text` - The text to draw (can be multi-line)
/// * `fg_color` - The foreground (text) color
/// * `padding` - Padding as a fraction of image size (0.0 to 0.4)
pub fn draw_centered_text(rgba: &mut RgbaImage, text: &str, fg_color: Rgba<u8>, padding: f32) {
    let Some(font_bytes) = get_system_monospace_font() else {
        return;
    };
    let Ok(font) = FontRef::try_from_slice(font_bytes) else {
        return;
    };

    let width = rgba.width();
    let height = rgba.height();

    let lines: Vec<&str> = text.lines().collect();
    if lines.is_empty() {
        return;
    }

    // Find optimal scale to fill the image with specified padding on each side
    let content_fraction = 1.0 - (2.0 * padding);
    let target_width = width as f32 * content_fraction;
    let target_height = height as f32 * content_fraction;
    let scale_value = find_optimal_scale(&font, &lines, target_width, target_height);
    let scale = PxScale::from(scale_value);

    // Get actual metrics at the chosen scale
    let scaled_font = font.as_scaled(scale);
    let line_height = scaled_font.height();
    let num_lines = lines.len() as f32;
    let total_height = num_lines * line_height;

    // Center vertically
    let start_y = (height as f32 - total_height) / 2.0;

    for (i, line) in lines.iter().enumerate() {
        // Calculate actual line width using font metrics
        let line_width: f32 = line
            .chars()
            .map(|c| scaled_font.h_advance(font.glyph_id(c)))
            .sum();

        // Center horizontally
        let text_x = ((width as f32 - line_width) / 2.0).max(0.0) as i32;
        let text_y = (start_y + i as f32 * line_height) as i32;

        draw_text_mut(rgba, fg_color, text_x, text_y, scale, &font, line);
    }
}

/// Draw text centered on an image with reserved space at top and bottom.
///
/// This is useful when you need to reserve space for other UI elements
/// like phase indicators or progress dots.
///
/// # Arguments
/// * `rgba` - The image to draw on
/// * `text` - The text to draw
/// * `fg_color` - The foreground (text) color
/// * `padding` - Padding as a fraction of available space (0.0 to 0.4)
/// * `reserved_top` - Pixels reserved at top
/// * `reserved_bottom` - Pixels reserved at bottom
/// * `y_offset` - Additional vertical offset
pub fn draw_centered_text_with_reserved(
    rgba: &mut RgbaImage,
    text: &str,
    fg_color: Rgba<u8>,
    padding: f32,
    reserved_top: f32,
    reserved_bottom: f32,
    y_offset: f32,
) {
    let Some(font_bytes) = get_system_monospace_font() else {
        return;
    };
    let Ok(font) = FontRef::try_from_slice(font_bytes) else {
        return;
    };

    let width = rgba.width();
    let height = rgba.height();

    let available_height = height as f32 - reserved_top - reserved_bottom;

    // Calculate optimal scale
    let content_fraction = 1.0 - (2.0 * padding);
    let target_width = width as f32 * content_fraction;
    let target_height = available_height * content_fraction;
    let scale_value = find_optimal_scale(&font, &[text], target_width, target_height);
    let scale = PxScale::from(scale_value);

    let scaled_font = font.as_scaled(scale);
    let line_height = scaled_font.height();

    // Calculate text width
    let text_width: f32 = text
        .chars()
        .map(|c| scaled_font.h_advance(font.glyph_id(c)))
        .sum();

    // Center horizontally and vertically in available space
    let x = ((width as f32 - text_width) / 2.0).max(0.0) as i32;
    let y = (reserved_top + (available_height - line_height) / 2.0 + y_offset) as i32;

    draw_text_mut(rgba, fg_color, x, y, scale, &font, text);
}

/// Draw text horizontally centered at a given y position with a fixed scale.
///
/// Handles font loading internally. If the system font cannot be loaded,
/// this is a no-op.
///
/// # Arguments
/// * `rgba` - The image to draw on
/// * `text` - The text to draw
/// * `color` - The text color
/// * `scale_value` - Font scale in pixels (e.g., 14.0, 18.0, 24.0)
/// * `y` - Vertical position (pixels from top)
pub fn draw_text_hcentered(
    rgba: &mut RgbaImage,
    text: &str,
    color: Rgba<u8>,
    scale_value: f32,
    y: i32,
) {
    let Some(font_bytes) = get_system_monospace_font() else {
        return;
    };
    let Ok(font) = FontRef::try_from_slice(font_bytes) else {
        return;
    };

    let width = rgba.width();
    let scale = PxScale::from(scale_value);
    let scaled_font = font.as_scaled(scale);

    let text_width: f32 = text
        .chars()
        .map(|c| scaled_font.h_advance(font.glyph_id(c)))
        .sum();

    let x = ((width as f32 - text_width) / 2.0).max(0.0) as i32;

    draw_text_mut(rgba, color, x, y, scale, &font, text);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ab_glyph::FontRef;

    // Helper to get a test font
    fn get_test_font() -> Option<FontRef<'static>> {
        let font_bytes = get_system_monospace_font()?;
        FontRef::try_from_slice(font_bytes).ok()
    }

    #[test]
    fn test_measure_text_width_empty() {
        if let Some(font) = get_test_font() {
            assert_eq!(measure_text_width(&font, ""), 0.0);
        }
    }

    #[test]
    fn test_measure_text_width_increases_with_length() {
        if let Some(font) = get_test_font() {
            let w1 = measure_text_width(&font, "a");
            let w2 = measure_text_width(&font, "aa");
            let w3 = measure_text_width(&font, "aaa");
            assert!(w2 > w1);
            assert!(w3 > w2);
        }
    }

    #[test]
    fn test_find_optimal_scale_clamps_minimum() {
        if let Some(font) = get_test_font() {
            // Very large target should still clamp to max 96.0
            let scale = find_optimal_scale(&font, &["a"], 10000.0, 10000.0);
            assert!(scale <= 96.0);
        }
    }

    #[test]
    fn test_find_optimal_scale_clamps_maximum() {
        if let Some(font) = get_test_font() {
            // Very small target should still clamp to min 8.0
            let scale = find_optimal_scale(&font, &["a"], 1.0, 1.0);
            assert!(scale >= 8.0);
        }
    }
}
