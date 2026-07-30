//! Image composition and rendering helpers shared by verandah and its plugins.
//!
//! - **badge**: compose a badge — a mark on a filled disc — onto an icon
//! - **colors**: CSS color parsing (named colors and hex formats)
//! - **font**: system font loading via fontconfig
//! - **text**: text measurement and rendering utilities
//! - **image**: image effects (brightness pulse) and format conversions
//!
//! Nothing in this crate crosses the verandah plugin ABI, so verandah and a
//! plugin may each link their own version without conflict.

pub mod colors;
pub mod font;
pub mod image;
pub mod text;

/// Prelude module for convenient imports.
///
/// ```ignore
/// use verandah_image::prelude::*;
/// ```
pub mod prelude {
    // Re-export image types that callers commonly use
    pub use ::image::{Pixel, Rgb, RgbImage, Rgba, RgbaImage};

    // Re-export font types for callers that need fine-grained text control
    pub use ab_glyph::{Font, FontRef, PxScale, ScaleFont};

    // Re-export drawing primitives for callers that need custom rendering
    pub use imageproc::drawing::{draw_filled_rect_mut, draw_text_mut};
    pub use imageproc::rect::Rect;

    // Colors
    pub use crate::colors::{get_color, hex as rgb, lookup as lookup_color, parse_colors};

    // Font
    pub use crate::font::get_system_monospace_font;

    // Text
    pub use crate::text::{
        draw_centered_text, draw_centered_text_with_reserved, draw_text_hcentered,
        find_optimal_scale, measure_text_width,
    };

    // Image utilities
    pub use crate::image::{
        apply_brightness_pulse, bytes_to_rgb, bytes_to_rgba, rgb_to_rgba, rgba_to_rgb, scale_image,
        to_greyscale,
    };
}
