//! Badge composition: a mark on a filled disc, knocked out of the base artwork.

use std::str::FromStr;

use image::{Pixel, Rgba, RgbaImage};
use serde::{Deserialize, Deserializer, de};

/// Where the badge sits on the icon.
///
/// Parses both the ImageMagick compass words (`NorthEast`) and the
/// abbreviations verandah already uses for deck placement (`NE`), so a config
/// can spell this the same way it spells `deck@NE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Gravity {
    NorthWest,
    North,
    #[default]
    NorthEast,
    West,
    Center,
    East,
    SouthWest,
    South,
    SouthEast,
}

impl FromStr for Gravity {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "northwest" | "nw" => Ok(Self::NorthWest),
            "north" | "n" => Ok(Self::North),
            "northeast" | "ne" => Ok(Self::NorthEast),
            "west" | "w" => Ok(Self::West),
            "center" | "centre" | "c" => Ok(Self::Center),
            "east" | "e" => Ok(Self::East),
            "southwest" | "sw" => Ok(Self::SouthWest),
            "south" | "s" => Ok(Self::South),
            "southeast" | "se" => Ok(Self::SouthEast),
            other => Err(format!(
                "unrecognized gravity '{other}': expected a compass direction such as NorthEast or NE"
            )),
        }
    }
}

impl<'de> Deserialize<'de> for Gravity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Gravity::from_str(&s).map_err(de::Error::custom)
    }
}

/// Disc diameter as a fraction of the icon's shorter side.
///
/// `2 * 0.61 * 135 / 512`, matching the generated artwork.
pub const DEFAULT_SIZE: f32 = 0.32;

/// Gap from disc edge to icon edge, as a fraction of the shorter side.
///
/// `20 / 512`, matching the generated artwork.
pub const DEFAULT_INSET: f32 = 0.04;

/// Knockout radius as a proportion above the disc radius.
///
/// `0.74 / 0.61 - 1`, matching the generated artwork.
pub const DEFAULT_CLEARANCE: f32 = 0.21;

/// Two steps down from the `#06a51f` artwork accent: dark enough that the mark
/// keeps the eye, still green enough not to read as a hole on a light key.
pub const DEFAULT_DISC_FILL: Rgba<u8> = Rgba([0x03, 0x4a, 0x0e, 0xff]);

/// White mark on the disc.
pub const DEFAULT_TEXT_COLOR: Rgba<u8> = Rgba([0xff, 0xff, 0xff, 0xff]);

/// The mark drawn on the badge disc.
#[derive(Debug, Clone)]
pub enum Mark {
    /// A short text mark, typically a single digit.
    Text(String),
    /// A pre-decoded logo, scaled and centred on the disc.
    ///
    /// This crate has no SVG renderer, so resolving and rasterizing a logo file
    /// is the caller's job. The mark must be centred in its own canvas.
    Logo(RgbaImage),
}

/// A badge: a mark on a filled disc, positioned by gravity.
///
/// All geometry is proportional to the icon's shorter side, so one spec works
/// across every device's key size.
#[derive(Debug, Clone)]
pub struct BadgeSpec {
    pub gravity: Gravity,
    /// Disc diameter as a fraction of the icon's shorter side.
    pub size: f32,
    /// Gap from disc edge to icon edge, same fraction.
    pub inset: f32,
    /// Knockout radius as a proportion above the disc radius.
    pub clearance: f32,
    pub disc_fill: Rgba<u8>,
    pub text_color: Rgba<u8>,
    pub mark: Mark,
}

impl BadgeSpec {
    /// A spec carrying `mark` with every other field at its default.
    pub fn new(mark: Mark) -> Self {
        Self {
            gravity: Gravity::default(),
            size: DEFAULT_SIZE,
            inset: DEFAULT_INSET,
            clearance: DEFAULT_CLEARANCE,
            disc_fill: DEFAULT_DISC_FILL,
            text_color: DEFAULT_TEXT_COLOR,
            mark,
        }
    }
}

/// Resolved pixel geometry of the disc on a particular icon.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub(crate) struct DiscGeometry {
    pub cx: f32,
    pub cy: f32,
    pub r: f32,
    pub knock_r: f32,
}

/// Resolve `spec`'s proportional geometry against an icon's pixel dimensions.
#[allow(dead_code)]
pub(crate) fn disc_geometry(width: u32, height: u32, spec: &BadgeSpec) -> DiscGeometry {
    let short = width.min(height) as f32;
    let r = short * spec.size / 2.0;
    let inset = short * spec.inset;
    let (w, h) = (width as f32, height as f32);

    let cx = match spec.gravity {
        Gravity::NorthWest | Gravity::West | Gravity::SouthWest => inset + r,
        Gravity::North | Gravity::Center | Gravity::South => w / 2.0,
        Gravity::NorthEast | Gravity::East | Gravity::SouthEast => w - inset - r,
    };
    let cy = match spec.gravity {
        Gravity::NorthWest | Gravity::North | Gravity::NorthEast => inset + r,
        Gravity::West | Gravity::Center | Gravity::East => h / 2.0,
        Gravity::SouthWest | Gravity::South | Gravity::SouthEast => h - inset - r,
    };

    DiscGeometry {
        cx,
        cy,
        r,
        knock_r: r * (1.0 + spec.clearance),
    }
}

/// Fraction of pixel `(px, py)` covered by a disc of radius `r` at `(cx, cy)`.
///
/// A one-pixel linear ramp at the boundary. Exact enough for a disc tens of
/// pixels across, and much cheaper than supersampling.
#[allow(dead_code)]
fn circle_coverage(px: u32, py: u32, cx: f32, cy: f32, r: f32) -> f32 {
    let dx = px as f32 + 0.5 - cx;
    let dy = py as f32 + 0.5 - cy;
    let distance = (dx * dx + dy * dy).sqrt();
    (r + 0.5 - distance).clamp(0.0, 1.0)
}

/// Half-open pixel range `(x0, y0, x1, y1)` enclosing a disc, clamped to the image.
#[allow(dead_code)]
fn bounding_box(width: u32, height: u32, cx: f32, cy: f32, r: f32) -> (u32, u32, u32, u32) {
    let x0 = (cx - r - 1.0).floor().max(0.0) as u32;
    let y0 = (cy - r - 1.0).floor().max(0.0) as u32;
    let x1 = ((cx + r + 1.0).ceil().max(0.0) as u32).min(width);
    let y1 = ((cy + r + 1.0).ceil().max(0.0) as u32).min(height);
    (x0, y0, x1, y1)
}

/// Erase the base artwork within the knockout radius by scaling its alpha to
/// zero, leaving RGB untouched.
///
/// The knockout radius exceeds the disc radius, so a transparent ring of width
/// `r * clearance` separates the disc from the artwork.
#[allow(dead_code)]
fn knockout(icon: &mut RgbaImage, g: &DiscGeometry) {
    let (x0, y0, x1, y1) = bounding_box(icon.width(), icon.height(), g.cx, g.cy, g.knock_r);
    for y in y0..y1 {
        for x in x0..x1 {
            let coverage = circle_coverage(x, y, g.cx, g.cy, g.knock_r);
            if coverage > 0.0 {
                let pixel = icon.get_pixel_mut(x, y);
                pixel[3] = (f32::from(pixel[3]) * (1.0 - coverage)).round() as u8;
            }
        }
    }
}

/// Composite the disc over the icon, its edge antialiased by coverage.
#[allow(dead_code)]
fn fill_disc(icon: &mut RgbaImage, g: &DiscGeometry, fill: Rgba<u8>) {
    let (x0, y0, x1, y1) = bounding_box(icon.width(), icon.height(), g.cx, g.cy, g.r);
    for y in y0..y1 {
        for x in x0..x1 {
            let coverage = circle_coverage(x, y, g.cx, g.cy, g.r);
            if coverage > 0.0 {
                let mut source = fill;
                source[3] = (f32::from(fill[3]) * coverage).round() as u8;
                icon.get_pixel_mut(x, y).blend(&source);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec_at(gravity: Gravity) -> BadgeSpec {
        BadgeSpec {
            gravity,
            ..BadgeSpec::new(Mark::Text("1".to_owned()))
        }
    }

    fn opaque(width: u32, height: u32) -> RgbaImage {
        RgbaImage::from_pixel(width, height, Rgba([10, 200, 30, 255]))
    }

    /// The disc centre the shell generator produces at --font-size 135 on a 512
    /// canvas is (409.65, 102.35). Rounding the derived fractions to the two
    /// decimals the config exposes moves it 0.05px, to (409.60, 102.40). This
    /// test is what certifies the defaults reproduce the existing artwork.
    #[test]
    fn disc_geometry_matches_generated_artwork_on_a_512_canvas() {
        let g = disc_geometry(512, 512, &spec_at(Gravity::NorthEast));
        assert!((g.cx - 409.60).abs() < 0.01, "cx was {}", g.cx);
        assert!((g.cy - 102.40).abs() < 0.01, "cy was {}", g.cy);
        assert!((g.r - 81.92).abs() < 0.01, "r was {}", g.r);
        // knockout radius = r * (1 + clearance)
        assert!(
            (g.knock_r - 99.12).abs() < 0.01,
            "knock_r was {}",
            g.knock_r
        );
    }

    #[test]
    fn disc_geometry_places_all_nine_gravities() {
        // 200x100: shorter side 100, so r = 16.0 and inset = 4.0.
        let r = 16.0;
        let i = 4.0;
        let left = i + r;
        let right = 200.0 - i - r;
        let top = i + r;
        let bottom = 100.0 - i - r;

        let cases = [
            (Gravity::NorthWest, left, top),
            (Gravity::North, 100.0, top),
            (Gravity::NorthEast, right, top),
            (Gravity::West, left, 50.0),
            (Gravity::Center, 100.0, 50.0),
            (Gravity::East, right, 50.0),
            (Gravity::SouthWest, left, bottom),
            (Gravity::South, 100.0, bottom),
            (Gravity::SouthEast, right, bottom),
        ];

        for (gravity, want_cx, want_cy) in cases {
            let g = disc_geometry(200, 100, &spec_at(gravity));
            assert!((g.cx - want_cx).abs() < 0.01, "{gravity:?} cx {}", g.cx);
            assert!((g.cy - want_cy).abs() < 0.01, "{gravity:?} cy {}", g.cy);
            assert!((g.r - r).abs() < 0.01, "{gravity:?} r {}", g.r);
        }
    }

    #[test]
    fn disc_geometry_scales_with_the_shorter_side() {
        // A 72px key: disc diameter is 0.32 * 72 = 23.04px.
        let g = disc_geometry(72, 72, &spec_at(Gravity::NorthEast));
        assert!((g.r * 2.0 - 23.04).abs() < 0.01, "diameter {}", g.r * 2.0);
    }

    #[test]
    fn badge_spec_new_applies_normative_defaults() {
        let spec = BadgeSpec::new(Mark::Text("3".to_owned()));
        assert_eq!(spec.gravity, Gravity::NorthEast);
        assert_eq!(spec.size, 0.32);
        assert_eq!(spec.inset, 0.04);
        assert_eq!(spec.clearance, 0.21);
        assert_eq!(spec.disc_fill, Rgba([0x03, 0x4a, 0x0e, 0xff]));
        assert_eq!(spec.text_color, Rgba([0xff, 0xff, 0xff, 0xff]));
    }

    #[test]
    fn gravity_accepts_imagemagick_words() -> Result<(), String> {
        assert_eq!("NorthEast".parse::<Gravity>()?, Gravity::NorthEast);
        assert_eq!("SouthWest".parse::<Gravity>()?, Gravity::SouthWest);
        assert_eq!("Center".parse::<Gravity>()?, Gravity::Center);
        Ok(())
    }

    #[test]
    fn gravity_accepts_compass_abbreviations() -> Result<(), String> {
        assert_eq!("NE".parse::<Gravity>()?, Gravity::NorthEast);
        assert_eq!("sw".parse::<Gravity>()?, Gravity::SouthWest);
        assert_eq!("C".parse::<Gravity>()?, Gravity::Center);
        Ok(())
    }

    #[test]
    fn gravity_is_case_insensitive() -> Result<(), String> {
        assert_eq!("nOrThEaSt".parse::<Gravity>()?, Gravity::NorthEast);
        Ok(())
    }

    #[test]
    fn gravity_accepts_centre_spelling() -> Result<(), String> {
        assert_eq!("centre".parse::<Gravity>()?, Gravity::Center);
        Ok(())
    }

    #[test]
    fn gravity_rejects_unknown_and_placement_meaningless_values() {
        for bad in ["", "middle", "none", "forget", "up"] {
            assert!(bad.parse::<Gravity>().is_err(), "{bad} should not parse");
        }
    }

    #[test]
    fn gravity_defaults_to_north_east() {
        assert_eq!(Gravity::default(), Gravity::NorthEast);
    }

    #[test]
    fn gravity_deserializes_from_string() -> Result<(), serde_json::Error> {
        let g: Gravity = serde_json::from_str("\"NE\"")?;
        assert_eq!(g, Gravity::NorthEast);
        Ok(())
    }

    #[test]
    fn gravity_maps_every_variant_from_both_spellings() -> Result<(), String> {
        let cases = [
            ("NorthWest", "NW", Gravity::NorthWest),
            ("North", "N", Gravity::North),
            ("NorthEast", "NE", Gravity::NorthEast),
            ("West", "W", Gravity::West),
            ("Center", "C", Gravity::Center),
            ("East", "E", Gravity::East),
            ("SouthWest", "SW", Gravity::SouthWest),
            ("South", "S", Gravity::South),
            ("SouthEast", "SE", Gravity::SouthEast),
        ];
        for (word, abbrev, want) in cases {
            assert_eq!(word.parse::<Gravity>()?, want, "full word {word}");
            assert_eq!(abbrev.parse::<Gravity>()?, want, "abbreviation {abbrev}");
            // and case-insensitively
            assert_eq!(
                word.to_ascii_lowercase().parse::<Gravity>()?,
                want,
                "lowercased {word}"
            );
            assert_eq!(
                abbrev.to_ascii_lowercase().parse::<Gravity>()?,
                want,
                "lowercased {abbrev}"
            );
        }
        Ok(())
    }

    #[test]
    fn circle_coverage_is_one_inside_zero_outside_and_partial_at_the_edge() {
        // Centre of a radius-10 disc at (20.0, 20.0).
        assert_eq!(circle_coverage(20, 20, 20.0, 20.0, 10.0), 1.0);
        // Far outside.
        assert_eq!(circle_coverage(0, 0, 20.0, 20.0, 10.0), 0.0);
        // Straddling the edge: pixel centre at 30.5 is 10.5 from centre, so
        // coverage = 10 + 0.5 - 10.5 = 0.0; at 29.5 it is fully covered.
        let edge = circle_coverage(29, 20, 20.0, 20.0, 10.0);
        assert!(edge > 0.0 && edge <= 1.0, "edge coverage was {edge}");
    }

    #[test]
    fn knockout_zeroes_alpha_inside_and_leaves_the_rest_alone() {
        let mut img = opaque(72, 72);
        let spec = spec_at(Gravity::NorthEast);
        let g = disc_geometry(72, 72, &spec);

        knockout(&mut img, &g);

        // Disc centre is fully knocked out.
        let centre = img.get_pixel(g.cx.round() as u32, g.cy.round() as u32);
        assert_eq!(centre[3], 0, "centre alpha");

        // A pixel in the clearance ring, just outside the disc but inside the
        // knockout, is also fully transparent.
        let ring_x = (g.cx + (g.r + g.knock_r) / 2.0).round() as u32;
        let ring = img.get_pixel(ring_x, g.cy.round() as u32);
        assert_eq!(ring[3], 0, "clearance ring alpha");

        // The opposite corner is untouched.
        assert_eq!(*img.get_pixel(0, 71), Rgba([10, 200, 30, 255]));
    }

    #[test]
    fn knockout_preserves_rgb_so_only_alpha_carries_the_hole() {
        let mut img = opaque(72, 72);
        let spec = spec_at(Gravity::Center);
        let g = disc_geometry(72, 72, &spec);

        knockout(&mut img, &g);

        let centre = img.get_pixel(36, 36);
        assert_eq!([centre[0], centre[1], centre[2]], [10, 200, 30]);
    }

    #[test]
    fn fill_disc_paints_the_disc_and_nothing_beyond_the_knockout() {
        let mut img = opaque(72, 72);
        let spec = spec_at(Gravity::NorthEast);
        let g = disc_geometry(72, 72, &spec);
        let fill = Rgba([0x03, 0x4a, 0x0e, 0xff]);

        fill_disc(&mut img, &g, fill);

        assert_eq!(
            *img.get_pixel(g.cx.round() as u32, g.cy.round() as u32),
            fill
        );
        // Well outside the disc, the base survives.
        assert_eq!(*img.get_pixel(0, 71), Rgba([10, 200, 30, 255]));
    }

    /// A disc larger than the icon must clip to the image rather than index out
    /// of bounds, and must still paint what falls inside.
    #[test]
    fn fill_disc_clips_a_disc_larger_than_the_image() {
        let mut img = opaque(20, 20);
        let spec = BadgeSpec {
            size: 1.5, // deliberately larger than the icon
            inset: 0.0,
            ..BadgeSpec::new(Mark::Text("1".to_owned()))
        };
        let g = disc_geometry(20, 20, &spec);
        let fill = Rgba([1, 2, 3, 255]);

        knockout(&mut img, &g);
        fill_disc(&mut img, &g, fill);

        assert_eq!(
            img.dimensions(),
            (20, 20),
            "clipping changed the image size"
        );
        // The disc centre lies inside the image and must carry the fill.
        let (cx, cy) = (g.cx.round() as u32, g.cy.round() as u32);
        assert!(cx < 20 && cy < 20, "centre ({cx},{cy}) left the image");
        assert_eq!(*img.get_pixel(cx, cy), fill);
    }
}
