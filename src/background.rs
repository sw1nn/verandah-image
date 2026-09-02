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
    pub color: Rgba<u8>,
    pub inset: f32,
    pub radius: f32,
}

impl Default for BackgroundSpec {
    fn default() -> Self {
        Self {
            color: Rgba([0, 0, 0, 0]),
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
    if spec.color.0[3] == 0 {
        return;
    }

    let plate = plate_geometry(icon.width(), icon.height(), spec);
    if plate.is_empty() {
        return;
    }

    // One pass, in place: build the plate pixel, composite the artwork over
    // it, and write the result back where the artwork was.
    for (x, y, pixel) in icon.enumerate_pixels_mut() {
        let coverage = plate.coverage(x, y);
        if coverage <= 0.0 {
            continue;
        }

        let mut ground = spec.color;
        ground.0[3] = (f32::from(spec.color.0[3]) * coverage).round() as u8;
        ground.blend(pixel);
        *pixel = ground;
    }
}

/// Blend the plate colour *over* an already-composed image at `alpha`.
///
/// The counterpart to [`apply_background`], for the case where the image is
/// already opaque: a card tint on a rendered button, or on a plugin's own
/// output. Over rather than under of necessity — there is no transparency left
/// to show a ground through.
pub fn apply_tint(image: &mut RgbaImage, spec: &BackgroundSpec, alpha: f32) {
    let alpha = alpha.clamp(0.0, 1.0);
    if alpha == 0.0 || spec.color.0[3] == 0 {
        return;
    }

    let plate = plate_geometry(image.width(), image.height(), spec);
    if plate.is_empty() {
        return;
    }

    // `alpha` is the whole tint strength; the spec's own alpha only says
    // whether there is a tint at all, checked above.
    let tint_alpha = 255.0 * alpha;

    for (x, y, pixel) in image.enumerate_pixels_mut() {
        let coverage = plate.coverage(x, y);
        if coverage <= 0.0 {
            continue;
        }

        let mut over = spec.color;
        over.0[3] = (tint_alpha * coverage).round() as u8;
        pixel.blend(&over);
    }
}

/// A plate's rectangle and corner radius, in continuous coordinates.
///
/// The edges are pixel *boundaries*, not pixel centres: a plate inset by two
/// pixels has `x0 == 2.0`, so column 2 (centre 2.5) is fully covered and
/// column 1 is not covered at all. That keeps the straight edges crisp while
/// the corner arcs get a coverage ramp.
struct Plate {
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    radius: f32,
}

impl Plate {
    /// Whether there is any plate left to draw: an inset past half the key
    /// collapses the rectangle.
    fn is_empty(&self) -> bool {
        self.x1 <= self.x0 || self.y1 <= self.y0
    }

    /// How much of the pixel at `(x, y)` the plate covers, in `0.0..=1.0`.
    ///
    /// A one-pixel linear ramp across the boundary, the treatment
    /// `badge::circle_coverage` already gives the disc. Without it a
    /// `radius = 0.16` corner is a 12-15 px stair-step on a real key, sitting
    /// right beside that smooth disc.
    fn coverage(&self, x: u32, y: u32) -> f32 {
        let half_width = (self.x1 - self.x0) / 2.0;
        let half_height = (self.y1 - self.y0) / 2.0;

        // Signed distance to the rounded rectangle: fold the pixel centre into
        // one quadrant against the corner arc's centre, and the straight edges
        // fall out of the same expression as the negative case.
        let qx = ((x as f32 + 0.5) - (self.x0 + half_width)).abs() - (half_width - self.radius);
        let qy = ((y as f32 + 0.5) - (self.y0 + half_height)).abs() - (half_height - self.radius);
        let distance = qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - self.radius;

        (0.5 - distance).clamp(0.0, 1.0)
    }
}

/// Compute the plate geometry for a given image size and spec.
fn plate_geometry(width: u32, height: u32, spec: &BackgroundSpec) -> Plate {
    let shorter = width.min(height) as f32;
    let inset = (shorter * spec.inset).round().max(0.0);

    let x0 = inset;
    let y0 = inset;
    let x1 = width as f32 - inset;
    let y1 = height as f32 - inset;

    // Cap the radius at half the plate's shorter side: past that the two arcs
    // on a side overlap and the distance field folds back on itself. Capping
    // degrades an oversized radius to a stadium or a circle instead.
    let half_width = ((x1 - x0) / 2.0).max(0.0);
    let half_height = ((y1 - y0) / 2.0).max(0.0);
    let radius = (shorter * spec.radius)
        .round()
        .clamp(0.0, half_width.min(half_height));

    Plate {
        x0,
        y0,
        x1,
        y1,
        radius,
    }
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
            color: Rgba([0x2f, 0x7f, 0x86, 0xff]),
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
            color: Rgba([0x2f, 0x7f, 0x86, 0xff]),
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
                color: Rgba([255, 255, 255, 255]),
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

        let color = Rgba([255, 255, 255, 255]);
        apply_background(
            &mut rounded,
            &BackgroundSpec {
                color,
                ..Default::default()
            },
        );
        apply_background(
            &mut square,
            &BackgroundSpec {
                color,
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

    /// The corner arc must be antialiased rather than stair-stepped: at least
    /// one pixel on it has to be partly covered. Only 0 and 255 in the corner
    /// square means a hard boolean test is back.
    #[test]
    fn corners_are_antialiased() -> Result<(), String> {
        let size = 96u32;
        let mut icon = RgbaImage::from_pixel(size, size, Rgba([0, 0, 0, 0]));
        apply_background(
            &mut icon,
            &BackgroundSpec {
                color: Rgba([255, 255, 255, 255]),
                ..Default::default()
            },
        );

        // The whole top-left corner: inset plus radius bounds the arc.
        let corner = (size as f32 * (DEFAULT_INSET + DEFAULT_RADIUS)).ceil() as u32;
        let partial = (0..corner)
            .flat_map(|y| (0..corner).map(move |x| (x, y)))
            .any(|(x, y)| matches!(icon.get_pixel(x, y).0[3], 1..=254));

        assert!(partial, "no pixel on the corner arc was partly covered");
        Ok(())
    }

    /// The tint's corners get the same ramp, so a tinted key and a
    /// backgrounded key are the same shape down to the antialiasing.
    #[test]
    fn tinted_corners_are_antialiased() -> Result<(), String> {
        let size = 96u32;
        let mut image = RgbaImage::from_pixel(size, size, Rgba([0, 0, 0, 255]));
        apply_tint(
            &mut image,
            &BackgroundSpec {
                color: Rgba([255, 255, 255, 255]),
                ..Default::default()
            },
            1.0,
        );

        let corner = (size as f32 * (DEFAULT_INSET + DEFAULT_RADIUS)).ceil() as u32;
        let partial = (0..corner)
            .flat_map(|y| (0..corner).map(move |x| (x, y)))
            .any(|(x, y)| matches!(image.get_pixel(x, y).0[0], 1..=254));

        assert!(
            partial,
            "no pixel on the tinted corner arc was partly tinted"
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
                color: Rgba([255, 255, 255, 255]),
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
                color: Rgba([255, 255, 255, 255]),
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
            color: Rgba([255, 255, 255, 255]),
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
                color: Rgba([255, 255, 255, 255]),
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
                color: Rgba([255, 255, 255, 255]),
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
