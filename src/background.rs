//! Background composition: a rounded plate beneath a key's artwork.

use image::{Pixel, Rgba, RgbaImage};

/// Gap from plate edge to key edge, as a fraction of the shorter side.
pub const DEFAULT_INSET: f32 = 0.03;
/// Corner radius, same fraction.
pub const DEFAULT_RADIUS: f32 = 0.16;

/// A plate's colour and geometry. Geometry is a fraction of the shorter side,
/// so one setting works across every device size.
#[derive(Debug, Clone, PartialEq)]
pub struct BackgroundSpec {
    pub colour: Rgba<u8>,
    pub inset: f32,
    pub radius: f32,
}

impl Default for BackgroundSpec {
    fn default() -> Self {
        Self {
            colour: Rgba([0, 0, 0, 0]),
            inset: DEFAULT_INSET,
            radius: DEFAULT_RADIUS,
        }
    }
}

/// Composite `icon` over a rounded plate, making the plate the ground.
///
/// The plate goes *under* the artwork rather than over it, which is why this
/// takes the whole image rather than blending a colour in: `images::prepare_base`
/// flattens alpha onto a single colour before resizing, so a plate applied
/// afterwards would land on opaque pixels and hide the artwork entirely.
///
/// A fully transparent colour draws nothing, so an unset background is free.
pub fn apply_background(icon: &mut RgbaImage, spec: &BackgroundSpec) {
    if spec.colour.0[3] == 0 {
        return;
    }

    let (width, height) = (icon.width(), icon.height());
    let (left, top, right, bottom, radius) = plate_geometry(width, height, spec);

    let mut plate = RgbaImage::from_pixel(width, height, Rgba([0, 0, 0, 0]));
    for y in 0..height {
        for x in 0..width {
            if inside_rounded_rect(x as f32, y as f32, left, top, right, bottom, radius) {
                plate.put_pixel(x, y, spec.colour);
            }
        }
    }

    // Composite the artwork over the plate, then hand the result back.
    for (x, y, pixel) in icon.enumerate_pixels() {
        let mut under = *plate.get_pixel(x, y);
        under.blend(pixel);
        plate.put_pixel(x, y, under);
    }
    *icon = plate;
}

/// Blend the plate colour *over* an already-composed image at `alpha`.
///
/// The counterpart to [`apply_background`], for the case where the image is
/// already opaque: a card tint on a rendered button, or on a plugin's own
/// output. Over rather than under of necessity — there is no transparency left
/// to show a ground through.
pub fn apply_tint(image: &mut RgbaImage, spec: &BackgroundSpec, alpha: f32) {
    let alpha = alpha.clamp(0.0, 1.0);
    if alpha == 0.0 || spec.colour.0[3] == 0 {
        return;
    }

    let mut over = spec.colour;
    over.0[3] = (255.0 * alpha).round() as u8;

    let (width, height) = (image.width(), image.height());
    let (left, top, right, bottom, radius) = plate_geometry(width, height, spec);

    for y in 0..height {
        for x in 0..width {
            if inside_rounded_rect(x as f32, y as f32, left, top, right, bottom, radius) {
                image.get_pixel_mut(x, y).blend(&over);
            }
        }
    }
}

/// Compute the plate geometry for a given image size and spec.
/// Returns (left, top, right, bottom, radius).
fn plate_geometry(width: u32, height: u32, spec: &BackgroundSpec) -> (f32, f32, f32, f32, f32) {
    let shorter = width.min(height) as f32;
    let inset = (shorter * spec.inset).round().max(0.0);
    let radius = (shorter * spec.radius).round().max(0.0);

    let left = inset;
    let top = inset;
    let right = width as f32 - inset - 1.0;
    let bottom = height as f32 - inset - 1.0;

    (left, top, right, bottom, radius)
}

/// Whether a point lies inside a rounded rectangle, corners included.
fn inside_rounded_rect(
    x: f32,
    y: f32,
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
    radius: f32,
) -> bool {
    if x < left || x > right || y < top || y > bottom {
        return false;
    }

    // Cap the radius at half the rect's smaller dimension: past that, the
    // corner clamp below would invert (min > max) and panic. Capping instead
    // degrades an oversized radius to a stadium or circle.
    let radius = radius
        .min((right - left) / 2.0)
        .min((bottom - top) / 2.0)
        .max(0.0);
    if radius <= 0.0 {
        return true;
    }

    // Clamp the point to the rectangle inset by `radius`; the distance from
    // that clamped point is what the corner arc tests against.
    let cx = x.clamp(left + radius, right - radius);
    let cy = y.clamp(top + radius, bottom - radius);
    let (dx, dy) = (x - cx, y - cy);
    dx * dx + dy * dy <= radius * radius
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fully transparent icon over an opaque plate: the centre must become
    /// the plate colour, because the plate is the ground, not an overlay.
    #[test]
    fn plate_shows_through_transparent_artwork() -> Result<(), String> {
        let mut icon = RgbaImage::from_pixel(96, 96, Rgba([0, 0, 0, 0]));
        let spec = BackgroundSpec {
            colour: Rgba([0x2f, 0x7f, 0x86, 0xff]),
            ..Default::default()
        };

        apply_background(&mut icon, &spec);

        let centre = icon.get_pixel(48, 48);
        assert_eq!(
            *centre,
            Rgba([0x2f, 0x7f, 0x86, 0xff]),
            "centre was not the plate colour"
        );
        Ok(())
    }

    /// Opaque artwork must survive intact: the plate goes under it.
    #[test]
    fn opaque_artwork_is_not_tinted() -> Result<(), String> {
        let mut icon = RgbaImage::from_pixel(96, 96, Rgba([200, 10, 10, 255]));
        let spec = BackgroundSpec {
            colour: Rgba([0x2f, 0x7f, 0x86, 0xff]),
            ..Default::default()
        };

        apply_background(&mut icon, &spec);

        assert_eq!(
            *icon.get_pixel(48, 48),
            Rgba([200, 10, 10, 255]),
            "opaque artwork was modified by the plate beneath it"
        );
        Ok(())
    }

    /// The inset leaves the extreme corner clear of the plate, so neighbouring
    /// keys stay visually separate.
    #[test]
    fn inset_leaves_the_corner_clear() -> Result<(), String> {
        for size in [72u32, 96] {
            let mut icon = RgbaImage::from_pixel(size, size, Rgba([0, 0, 0, 0]));
            let spec = BackgroundSpec {
                colour: Rgba([255, 255, 255, 255]),
                ..Default::default()
            };

            apply_background(&mut icon, &spec);

            assert_eq!(
                icon.get_pixel(0, 0).0[3],
                0,
                "top-left corner was painted at size {size}"
            );
            assert_eq!(
                *icon.get_pixel(size / 2, size / 2),
                Rgba([255, 255, 255, 255]),
                "centre was not painted at size {size}"
            );
        }
        Ok(())
    }

    /// The corner radius must actually round: a pixel just inside the inset at
    /// the corner is outside a rounded plate but inside a square one.
    #[test]
    fn radius_rounds_the_corners() -> Result<(), String> {
        let size = 96u32;
        let mut rounded = RgbaImage::from_pixel(size, size, Rgba([0, 0, 0, 0]));
        let mut square = RgbaImage::from_pixel(size, size, Rgba([0, 0, 0, 0]));

        let colour = Rgba([255, 255, 255, 255]);
        apply_background(
            &mut rounded,
            &BackgroundSpec {
                colour,
                ..Default::default()
            },
        );
        apply_background(
            &mut square,
            &BackgroundSpec {
                colour,
                radius: 0.0,
                ..Default::default()
            },
        );

        let inset_px = (size as f32 * DEFAULT_INSET).round() as u32;
        let probe = inset_px + 1;

        assert_eq!(
            square.get_pixel(probe, probe).0[3],
            255,
            "square plate did not reach its own corner"
        );
        assert!(
            rounded.get_pixel(probe, probe).0[3] < 255,
            "rounded plate painted a corner a square one would"
        );
        Ok(())
    }

    /// A radius requested past half the plate's shorter side must not invert
    /// the corner clamp and panic; it should instead degrade to a stadium or
    /// circle, still painting a sane plate.
    #[test]
    fn oversized_radius_does_not_panic() -> Result<(), String> {
        for radius in [0.5f32, 1.0, 5.0] {
            let size = 96u32;
            let mut icon = RgbaImage::from_pixel(size, size, Rgba([0, 0, 0, 0]));
            let spec = BackgroundSpec {
                colour: Rgba([255, 255, 255, 255]),
                radius,
                ..Default::default()
            };

            apply_background(&mut icon, &spec);

            assert_eq!(
                *icon.get_pixel(size / 2, size / 2),
                Rgba([255, 255, 255, 255]),
                "centre was not painted with radius fraction {radius}"
            );
            assert_eq!(
                icon.get_pixel(0, 0).0[3],
                0,
                "top-left corner was painted despite the inset with radius fraction {radius}"
            );
        }
        Ok(())
    }

    /// A fully transparent plate colour must leave the image untouched, so an
    /// unset background costs nothing.
    #[test]
    fn transparent_plate_draws_nothing() -> Result<(), String> {
        let mut icon = RgbaImage::from_pixel(72, 72, Rgba([10, 20, 30, 255]));
        let before = icon.as_raw().clone();

        apply_background(&mut icon, &BackgroundSpec::default());

        assert_eq!(
            icon.as_raw(),
            &before,
            "a transparent plate modified the image"
        );
        Ok(())
    }

    /// Non-square keys resolve geometry off the shorter side.
    #[test]
    fn geometry_uses_the_shorter_side() -> Result<(), String> {
        let mut icon = RgbaImage::from_pixel(120, 60, Rgba([0, 0, 0, 0]));
        apply_background(
            &mut icon,
            &BackgroundSpec {
                colour: Rgba([255, 255, 255, 255]),
                ..Default::default()
            },
        );

        let inset_px = (60.0 * DEFAULT_INSET).round() as u32;
        assert_eq!(
            icon.get_pixel(60, inset_px + 4).0[3],
            255,
            "plate did not reach the expected inset on the shorter side"
        );
        Ok(())
    }

    /// A tint blends over opaque artwork rather than replacing it: the result
    /// must move towards the tint without becoming it.
    #[test]
    fn tint_blends_over_opaque_artwork() -> Result<(), String> {
        let mut image = RgbaImage::from_pixel(96, 96, Rgba([0, 0, 0, 255]));
        let spec = BackgroundSpec {
            colour: Rgba([255, 255, 255, 255]),
            ..Default::default()
        };

        apply_tint(&mut image, &spec, 0.45);

        let centre = image.get_pixel(48, 48).0[0];
        assert!(centre > 0, "tint did not lighten black artwork");
        assert!(
            centre < 255,
            "tint replaced the artwork instead of blending"
        );
        Ok(())
    }

    /// The tint honours the same plate geometry, so a tinted key and a
    /// backgrounded key are the same shape.
    #[test]
    fn tint_respects_the_inset() -> Result<(), String> {
        let mut image = RgbaImage::from_pixel(96, 96, Rgba([0, 0, 0, 255]));
        apply_tint(
            &mut image,
            &BackgroundSpec {
                colour: Rgba([255, 255, 255, 255]),
                ..Default::default()
            },
            0.45,
        );

        assert_eq!(
            *image.get_pixel(0, 0),
            Rgba([0, 0, 0, 255]),
            "tint painted outside the plate inset"
        );
        Ok(())
    }

    /// An alpha of zero is a no-op, so a card colour can be suppressed without
    /// a branch at the call site.
    #[test]
    fn zero_alpha_tint_draws_nothing() -> Result<(), String> {
        let mut image = RgbaImage::from_pixel(72, 72, Rgba([10, 20, 30, 255]));
        let before = image.as_raw().clone();

        apply_tint(
            &mut image,
            &BackgroundSpec {
                colour: Rgba([255, 255, 255, 255]),
                ..Default::default()
            },
            0.0,
        );

        assert_eq!(
            image.as_raw(),
            &before,
            "a zero-alpha tint modified the image"
        );
        Ok(())
    }
}
