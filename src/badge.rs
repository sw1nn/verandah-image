//! Badge composition: a mark on a filled disc, knocked out of the base artwork.

use std::str::FromStr;

use ab_glyph::{Font, FontRef, OutlinedGlyph, PxScale, ScaleFont, point};
use image::{Pixel, Rgba, RgbaImage};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

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

impl Gravity {
    /// The ImageMagick spelling, which is what this serializes as.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NorthWest => "NorthWest",
            Self::North => "North",
            Self::NorthEast => "NorthEast",
            Self::West => "West",
            Self::Center => "Center",
            Self::East => "East",
            Self::SouthWest => "SouthWest",
            Self::South => "South",
            Self::SouthEast => "SouthEast",
        }
    }
}

impl Serialize for Gravity {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
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
pub(crate) struct DiscGeometry {
    pub cx: f32,
    pub cy: f32,
    pub r: f32,
    pub knock_r: f32,
}

/// Resolve `spec`'s proportional geometry against an icon's pixel dimensions.
///
/// This models a 1:1 viewBox-to-canvas mapping: `width` and `height` are assumed to
/// be exactly the pixel grid the fractions are measured against. A caller that
/// rasterizes through a bordered pipeline first — verandah's `convert_svg` renders
/// at a fixed resolution, adds a border, and only then resizes to the target — is
/// not that case: the rasterized artwork sits inset within the requested canvas by
/// a small proportion (measured at roughly 1.7%), so the disc lands a few pixels off
/// centre and slightly oversized at large canvases. This is invisible at real key
/// sizes (72-96px, sub-pixel) and only measurable at large ones (a few px at 512px),
/// so it is not corrected here.
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
fn circle_coverage(px: u32, py: u32, cx: f32, cy: f32, r: f32) -> f32 {
    let dx = px as f32 + 0.5 - cx;
    let dy = py as f32 + 0.5 - cy;
    let distance = (dx * dx + dy * dy).sqrt();
    (r + 0.5 - distance).clamp(0.0, 1.0)
}

/// Half-open pixel range `(x0, y0, x1, y1)` enclosing a disc, clamped to the image.
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

/// The face badge text is drawn with.
///
/// DejaVu Sans Bold specifically, because that is what the pre-generated artwork
/// used. Asking fontconfig for plain `sans-serif` yields Liberation Sans Bold on a
/// typical Arch box, whose digits differ in shape and cap height — moving a key to
/// a declared badge would then be a visible change. Falls back to any bold sans so
/// a machine without DejaVu still gets a badge rather than none.
fn badge_font_bytes() -> Option<&'static [u8]> {
    crate::font::get_system_font("DejaVu Sans", Some("Bold"))
        .or_else(|| crate::font::get_system_font("sans-serif", Some("Bold")))
}

/// Cap height as a fraction of the disc diameter.
///
/// The generator's 0.729em cap height against its 1.22em disc.
const CAP_HEIGHT_FACTOR: f32 = 0.598;

/// The ink box's half-diagonal may not exceed this fraction of the disc radius.
///
/// The generator's stated 0.51em digit half-diagonal against its 0.61em radius.
/// Inactive for a single digit; it is what lets a two-character mark fit.
const FIT_LIMIT: f32 = 0.84;

/// A mark laid out at the scale that fits it to the disc.
struct FittedText {
    scale: f32,
    glyphs: Vec<OutlinedGlyph>,
    /// `(min_x, min_y, max_x, max_y)` of the union of the glyphs' ink bounds.
    bounds: (f32, f32, f32, f32),
}

/// Ink height of a reference capital as a fraction of the px scale.
///
/// `ab_glyph` exposes ascent and descent but not cap height, and deriving the
/// scale from the mark's own ink box would size `1` and `8` differently.
/// Measuring one reference glyph keeps every digit consistent.
fn cap_height_ratio<F>(font: &F) -> Option<f32>
where
    F: Font,
{
    const TRIAL: f32 = 100.0;
    let glyph = font
        .glyph_id('H')
        .with_scale_and_position(PxScale::from(TRIAL), point(0.0, 0.0));
    let bounds = font.outline_glyph(glyph)?.px_bounds();
    Some((bounds.max.y - bounds.min.y) / TRIAL)
}

/// Outline `text` on a baseline at the origin, dropping glyphs with no outline.
fn layout<F>(font: &F, text: &str, scale: f32) -> Vec<OutlinedGlyph>
where
    F: Font,
{
    let scaled = font.as_scaled(PxScale::from(scale));
    let mut pen_x = 0.0;
    let mut glyphs = Vec::new();

    for ch in text.chars() {
        let id = font.glyph_id(ch);
        let glyph = id.with_scale_and_position(PxScale::from(scale), point(pen_x, 0.0));
        pen_x += scaled.h_advance(id);
        if let Some(outlined) = font.outline_glyph(glyph) {
            glyphs.push(outlined);
        }
    }

    glyphs
}

/// Union of the glyphs' ink bounds, or `None` when none has an outline.
fn ink_bounds(glyphs: &[OutlinedGlyph]) -> Option<(f32, f32, f32, f32)> {
    let mut iter = glyphs.iter();
    let first = iter.next()?.px_bounds();
    let mut bounds = (first.min.x, first.min.y, first.max.x, first.max.y);

    for glyph in iter {
        let b = glyph.px_bounds();
        bounds.0 = bounds.0.min(b.min.x);
        bounds.1 = bounds.1.min(b.min.y);
        bounds.2 = bounds.2.max(b.max.x);
        bounds.3 = bounds.3.max(b.max.y);
    }

    Some(bounds)
}

/// Half-diagonal of an ink box.
fn half_diagonal(bounds: (f32, f32, f32, f32)) -> f32 {
    let (min_x, min_y, max_x, max_y) = bounds;
    let (w, h) = (max_x - min_x, max_y - min_y);
    0.5 * (w * w + h * h).sqrt()
}

/// Lay `text` out at the scale that fits it to a disc of radius `r`.
///
/// Returns `None` when the font has no cap-height reference or the mark has no
/// drawable outline at all.
fn fit_text<F>(font: &F, text: &str, r: f32) -> Option<FittedText>
where
    F: Font,
{
    let cap_ratio = cap_height_ratio(font)?;
    if cap_ratio <= 0.0 {
        return None;
    }

    let mut scale = CAP_HEIGHT_FACTOR * 2.0 * r / cap_ratio;
    let mut glyphs = layout(font, text, scale);
    let mut bounds = ink_bounds(&glyphs)?;

    // Ink scales linearly with px scale, so one correction pass is exact enough.
    let limit = FIT_LIMIT * r;
    let extent = half_diagonal(bounds);
    if extent > limit && extent > 0.0 {
        scale *= limit / extent;
        glyphs = layout(font, text, scale);
        bounds = ink_bounds(&glyphs)?;
    }

    Some(FittedText {
        scale,
        glyphs,
        bounds,
    })
}

/// Draw `text` centred on the disc, in `color`.
///
/// Silently draws nothing if no bold sans face is available — a missing badge is
/// better than a missing button.
fn draw_text_mark(icon: &mut RgbaImage, g: &DiscGeometry, text: &str, color: Rgba<u8>) {
    let Some(bytes) = badge_font_bytes() else {
        tracing::warn!("No bold sans face found; badge text not drawn");
        return;
    };
    let Ok(font) = FontRef::try_from_slice(bytes) else {
        tracing::warn!("Bold sans face could not be parsed; badge text not drawn");
        return;
    };
    let Some(fitted) = fit_text(&font, text, g.r) else {
        tracing::debug!(text, "Badge mark has no drawable outline");
        return;
    };

    // Centre the ink box on the disc centre.
    let (min_x, min_y, max_x, max_y) = fitted.bounds;
    let offset_x = g.cx - (min_x + max_x) / 2.0;
    let offset_y = g.cy - (min_y + max_y) / 2.0;
    let (width, height) = (icon.width(), icon.height());

    tracing::debug!(text, scale = fitted.scale, "Drawing badge text mark");

    for glyph in &fitted.glyphs {
        let origin = glyph.px_bounds().min;
        glyph.draw(|gx, gy, coverage| {
            if coverage <= 0.0 {
                return;
            }
            let x = origin.x + offset_x + gx as f32;
            let y = origin.y + offset_y + gy as f32;
            if x < 0.0 || y < 0.0 {
                return;
            }
            let (x, y) = (x.round() as u32, y.round() as u32);
            if x >= width || y >= height {
                return;
            }
            let mut source = color;
            source[3] = (f32::from(color[3]) * coverage.min(1.0)).round() as u8;
            icon.get_pixel_mut(x, y).blend(&source);
        });
    }
}

/// Draw `logo` scaled into the square inscribed in the disc, centred.
///
/// Aspect ratio is preserved. Padding within the mark is the logo author's
/// business — the disc beneath it is drawn separately, as in the generator.
fn draw_logo_mark(icon: &mut RgbaImage, g: &DiscGeometry, logo: &RgbaImage) {
    if logo.width() == 0 || logo.height() == 0 {
        return;
    }

    // Side of the square inscribed in the disc.
    let side = 2.0 * g.r / std::f32::consts::SQRT_2;
    let scale = (side / logo.width() as f32).min(side / logo.height() as f32);
    let width = ((logo.width() as f32 * scale).round() as u32).max(1);
    let height = ((logo.height() as f32 * scale).round() as u32).max(1);

    let resized =
        image::imageops::resize(logo, width, height, image::imageops::FilterType::Lanczos3);

    let x = (g.cx - width as f32 / 2.0).round() as i64;
    let y = (g.cy - height as f32 / 2.0).round() as i64;

    tracing::debug!(width, height, x, y, "Drawing badge logo mark");

    image::imageops::overlay(icon, &resized, x, y);
}

/// Compose `spec`'s badge onto `icon`, in place.
///
/// Three ordered steps: knock the base's alpha out within the clearance radius,
/// fill the disc, then draw the mark. Because the knockout radius exceeds the
/// disc radius, a transparent ring separates the disc from the base artwork.
///
/// The icon's dimensions are unchanged. Geometry is proportional to the shorter
/// side, so one spec serves every key size.
pub fn apply_badge(icon: &mut RgbaImage, spec: &BadgeSpec) {
    if icon.width() == 0 || icon.height() == 0 {
        return;
    }

    let geometry = disc_geometry(icon.width(), icon.height(), spec);

    knockout(icon, &geometry);
    fill_disc(icon, &geometry, spec.disc_fill);

    match &spec.mark {
        Mark::Text(text) => draw_text_mark(icon, &geometry, text, spec.text_color),
        Mark::Logo(logo) => draw_logo_mark(icon, &geometry, logo),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gravity_round_trips_through_serde() -> Result<(), serde_json::Error> {
        for gravity in [Gravity::NorthWest, Gravity::Center, Gravity::SouthEast] {
            let json = serde_json::to_string(&gravity)?;
            assert_eq!(serde_json::from_str::<Gravity>(&json)?, gravity);
        }
        Ok(())
    }

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
    ///
    /// This validates `disc_geometry`'s arithmetic against the generator's own
    /// viewBox — a 1:1 mapping of fractions to pixels — and NOT against what
    /// verandah's `convert_svg` actually produces on disk. `convert_svg` rasterizes
    /// through a bordered ImageMagick pipeline (`set_resolution` then
    /// `border_image` then resize), which insets the artwork by a small proportion
    /// before these fractions ever see it. See the doc comment on `disc_geometry`.
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
        // Straddling the edge: pixel (29, 20) has centre at (29.5, 20.5), distance
        // from (20.0, 20.0) is sqrt(9.5^2 + 0.5^2) ≈ 9.5126, so coverage =
        // clamp(10.5 - 9.5126) = 0.9874: partial, not full.
        let edge = circle_coverage(29, 20, 20.0, 20.0, 10.0);
        assert!(edge > 0.0 && edge < 1.0, "edge coverage was {edge}");
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

    /// The disc edge is antialiased: a partially-covered pixel must land strictly
    /// between the fill and the base, and a partially-knocked-out pixel strictly
    /// between opaque and transparent. Without this, hard-thresholding coverage to
    /// 0 or 1 would pass every other test in this module.
    #[test]
    fn edge_pixels_are_blended_not_thresholded() -> Result<(), String> {
        let base = Rgba([10, 200, 30, 255]);
        let fill = Rgba([3, 74, 14, 255]);
        let spec = spec_at(Gravity::NorthEast);
        let g = disc_geometry(72, 72, &spec);

        // A pixel the disc covers only partially.
        let (px, py) = (0..72)
            .flat_map(|y| (0..72).map(move |x| (x, y)))
            .find(|&(x, y)| {
                let c = circle_coverage(x, y, g.cx, g.cy, g.r);
                c > 0.05 && c < 0.95
            })
            .ok_or("no partially covered pixel found on the disc edge")?;

        let mut img = RgbaImage::from_pixel(72, 72, base);
        fill_disc(&mut img, &g, fill);
        let blended = *img.get_pixel(px, py);
        for channel in 0..3 {
            let (lo, hi) = (
                fill[channel].min(base[channel]),
                fill[channel].max(base[channel]),
            );
            assert!(
                blended[channel] > lo && blended[channel] < hi,
                "channel {channel} at ({px},{py}) was {} — not between {lo} and {hi}",
                blended[channel]
            );
        }

        // And the knockout ramps alpha rather than switching it.
        let (kx, ky) = (0..72)
            .flat_map(|y| (0..72).map(move |x| (x, y)))
            .find(|&(x, y)| {
                let c = circle_coverage(x, y, g.cx, g.cy, g.knock_r);
                c > 0.05 && c < 0.95
            })
            .ok_or("no partially knocked-out pixel found")?;

        let mut img = RgbaImage::from_pixel(72, 72, base);
        knockout(&mut img, &g);
        let alpha = img.get_pixel(kx, ky)[3];
        assert!(
            alpha > 0 && alpha < 255,
            "alpha at ({kx},{ky}) was {alpha}, not partial"
        );

        Ok(())
    }

    /// The face the badge draws with. Hard-required: a test that silently passes
    /// when the font is missing is not a test. Returns `Result` so callers use `?`
    /// rather than unwrapping.
    fn bold_sans() -> Result<FontRef<'static>, String> {
        let bytes = badge_font_bytes()
            .ok_or("DejaVu Sans Bold must be installed: the badge is drawn with it")?;
        FontRef::try_from_slice(bytes).map_err(|e| format!("badge font failed to parse: {e}"))
    }

    // `half_diagonal` is the production function, reached via `use super::*`.
    // Do not redefine it here: a test-local copy would shadow the glob import and
    // the fit tests would stop exercising the real code.

    #[test]
    fn fit_text_sizes_every_digit_the_same() -> Result<(), String> {
        let font = bold_sans()?;
        let one = fit_text(&font, "1", 40.0).ok_or("no fit for 1")?;
        let eight = fit_text(&font, "8", 40.0).ok_or("no fit for 8")?;
        assert!(
            (one.scale - eight.scale).abs() < 0.01,
            "1 scaled to {} but 8 to {}",
            one.scale,
            eight.scale
        );
        Ok(())
    }

    #[test]
    fn fit_text_shrinks_a_two_character_mark() -> Result<(), String> {
        let font = bold_sans()?;
        let one = fit_text(&font, "1", 40.0).ok_or("no fit for 1")?;
        let twelve = fit_text(&font, "12", 40.0).ok_or("no fit for 12")?;
        assert!(
            twelve.scale < one.scale,
            "12 scaled to {} which is not smaller than {}",
            twelve.scale,
            one.scale
        );
        Ok(())
    }

    /// Cross-check `CAP_HEIGHT_FACTOR` against the generator's own arithmetic, in
    /// **pixels** rather than in font-scale units.
    ///
    /// The generator draws at SVG `font-size="135"` on a 512 canvas and DejaVu Sans
    /// Bold's cap height is 0.729em, so its cap height is 98.415px. Ours is
    /// `CAP_HEIGHT_FACTOR * disc diameter` = 97.976px — 0.44px apart, which is what
    /// makes a declared badge indistinguishable from the shipped artwork.
    ///
    /// Measured on `H`, which is flat-topped: round glyphs overshoot the cap line by
    /// design for optical correction, so DejaVu Sans Bold's `8` (yMin -29, yMax 1520)
    /// stands 3.75% taller than its `H` (0, 1493) and would confound this.
    ///
    /// Do NOT instead assert `fitted.scale == 135`. `ab_glyph`'s `PxScale` is relative
    /// to the font's ascent-to-descent span (2384 units for this face), not the em
    /// square (2048), so the `PxScale` equivalent of SVG font-size 135 is 157.15 and
    /// comparing the two directly is a unit error. `fit_text` derives 155.52 here.
    #[test]
    fn fit_text_reproduces_the_generators_cap_height() -> Result<(), String> {
        let font = bold_sans()?;
        let r = 81.92; // the 512-canvas disc radius
        let fitted = fit_text(&font, "H", r).ok_or("no fit for H")?;
        let (_, min_y, _, max_y) = fitted.bounds;
        let generator_cap = 0.729 * 135.0;
        assert!(
            (max_y - min_y - generator_cap).abs() < 1.5,
            "cap height {} differs from the generator's {generator_cap}",
            max_y - min_y
        );
        Ok(())
    }

    /// The cap-height derivation itself, measured on a flat-topped reference glyph
    /// with no overshoot, so this pins `CAP_HEIGHT_FACTOR` without a glyph-shape
    /// confound.
    #[test]
    fn fit_text_puts_cap_height_at_the_normative_fraction() -> Result<(), String> {
        let font = bold_sans()?;
        let r = 81.92;
        let fitted = fit_text(&font, "H", r).ok_or("no fit for H")?;
        let (_, min_y, _, max_y) = fitted.bounds;
        let want = CAP_HEIGHT_FACTOR * 2.0 * r;
        // px_bounds are conservative whole numbers, so allow a pixel either way.
        assert!(
            (max_y - min_y - want).abs() < 2.0,
            "cap height {} differs from target {want}",
            max_y - min_y
        );
        Ok(())
    }

    #[test]
    fn fit_text_keeps_marks_inside_the_disc() -> Result<(), String> {
        let font = bold_sans()?;
        let r = 40.0;
        for text in ["1", "8", "12", "99", "W"] {
            let fitted = fit_text(&font, text, r).ok_or("no fit")?;
            let hd = half_diagonal(fitted.bounds);
            assert!(
                hd <= FIT_LIMIT * r + 1.0,
                "{text} half-diagonal {hd} exceeds limit {}",
                FIT_LIMIT * r
            );
        }
        Ok(())
    }

    #[test]
    fn fit_text_returns_none_for_a_mark_with_no_outline() -> Result<(), String> {
        let font = bold_sans()?;
        assert!(fit_text(&font, "   ", 40.0).is_none());
        assert!(fit_text(&font, "", 40.0).is_none());
        Ok(())
    }

    #[test]
    fn draw_text_mark_paints_inside_the_disc_and_not_outside_it() {
        let mut img = RgbaImage::from_pixel(72, 72, Rgba([0, 0, 0, 255]));
        let spec = spec_at(Gravity::NorthEast);
        let g = disc_geometry(72, 72, &spec);
        let color = Rgba([0xff, 0xff, 0xff, 0xff]);

        fill_disc(&mut img, &g, Rgba([0x03, 0x4a, 0x0e, 0xff]));
        draw_text_mark(&mut img, &g, "8", color);

        let mut painted_inside = 0usize;
        for y in 0..72 {
            for x in 0..72 {
                if *img.get_pixel(x, y) != color {
                    continue;
                }
                let dx = f32::from(x as u16) + 0.5 - g.cx;
                let dy = f32::from(y as u16) + 0.5 - g.cy;
                let distance = (dx * dx + dy * dy).sqrt();
                assert!(
                    distance <= g.r + 1.0,
                    "text pixel at ({x},{y}) is outside the disc"
                );
                painted_inside += 1;
            }
        }
        assert!(painted_inside > 0, "no text pixels were painted");
    }

    #[test]
    fn draw_text_mark_does_not_panic_on_whitespace() {
        let mut img = RgbaImage::from_pixel(72, 72, Rgba([0, 0, 0, 255]));
        let spec = spec_at(Gravity::NorthEast);
        let g = disc_geometry(72, 72, &spec);
        draw_text_mark(&mut img, &g, "   ", Rgba([255, 255, 255, 255]));
    }

    #[test]
    fn draw_logo_mark_centres_the_logo_on_the_disc() {
        let mut img = RgbaImage::from_pixel(72, 72, Rgba([0, 0, 0, 255]));
        let spec = spec_at(Gravity::NorthEast);
        let g = disc_geometry(72, 72, &spec);
        let logo = RgbaImage::from_pixel(64, 64, Rgba([255, 0, 255, 255]));

        draw_logo_mark(&mut img, &g, &logo);

        // The disc centre carries the logo.
        let centre = img.get_pixel(g.cx.round() as u32, g.cy.round() as u32);
        assert_eq!(*centre, Rgba([255, 0, 255, 255]));
        // The far corner does not.
        assert_eq!(*img.get_pixel(0, 71), Rgba([0, 0, 0, 255]));
    }

    #[test]
    fn draw_logo_mark_fits_within_the_inscribed_square() {
        let mut img = RgbaImage::from_pixel(200, 200, Rgba([0, 0, 0, 255]));
        let spec = spec_at(Gravity::Center);
        let g = disc_geometry(200, 200, &spec);
        let logo = RgbaImage::from_pixel(100, 100, Rgba([255, 0, 255, 255]));

        draw_logo_mark(&mut img, &g, &logo);

        // Every logo pixel lies inside the disc, since the inscribed square does.
        for y in 0..200 {
            for x in 0..200 {
                if *img.get_pixel(x, y) != Rgba([255, 0, 255, 255]) {
                    continue;
                }
                let dx = x as f32 + 0.5 - g.cx;
                let dy = y as f32 + 0.5 - g.cy;
                assert!(
                    (dx * dx + dy * dy).sqrt() <= g.r + 1.0,
                    "logo pixel at ({x},{y}) escaped the disc"
                );
            }
        }
    }

    #[test]
    fn draw_logo_mark_preserves_aspect_ratio() {
        let mut img = RgbaImage::from_pixel(200, 200, Rgba([0, 0, 0, 255]));
        let spec = spec_at(Gravity::Center);
        let g = disc_geometry(200, 200, &spec);
        // Twice as wide as it is tall.
        let logo = RgbaImage::from_pixel(80, 40, Rgba([255, 0, 255, 255]));

        draw_logo_mark(&mut img, &g, &logo);

        let mut min_x = u32::MAX;
        let mut max_x = 0u32;
        let mut min_y = u32::MAX;
        let mut max_y = 0u32;
        for y in 0..200 {
            for x in 0..200 {
                if *img.get_pixel(x, y) == Rgba([255, 0, 255, 255]) {
                    min_x = min_x.min(x);
                    max_x = max_x.max(x);
                    min_y = min_y.min(y);
                    max_y = max_y.max(y);
                }
            }
        }
        let w = (max_x - min_x + 1) as f32;
        let h = (max_y - min_y + 1) as f32;
        assert!((w / h - 2.0).abs() < 0.15, "aspect was {}", w / h);
    }

    /// An 8px icon gives a disc about 2.5px across, so the logo downscales to a pixel
    /// or two. It must still land, and must not resize to a zero dimension.
    #[test]
    fn draw_logo_mark_survives_a_tiny_disc() {
        let mut img = RgbaImage::from_pixel(8, 8, Rgba([0, 0, 0, 255]));
        let spec = spec_at(Gravity::NorthEast);
        let g = disc_geometry(8, 8, &spec);
        let logo = RgbaImage::from_pixel(64, 64, Rgba([255, 0, 255, 255]));

        draw_logo_mark(&mut img, &g, &logo);

        assert_eq!(img.dimensions(), (8, 8));
        // At least one pixel changed: the mark was drawn, not silently dropped.
        let painted = img.pixels().any(|p| *p != Rgba([0, 0, 0, 255]));
        assert!(painted, "the logo was not drawn at all");
    }

    #[test]
    fn apply_badge_knocks_out_fills_and_marks_in_order() {
        let mut img = opaque(72, 72);
        let spec = spec_at(Gravity::NorthEast);
        let g = disc_geometry(72, 72, &spec);

        apply_badge(&mut img, &spec);

        // The clearance ring is transparent: knockout ran and the disc did not
        // cover it.
        let ring_x = (g.cx + (g.r + g.knock_r) / 2.0).round() as u32;
        assert_eq!(img.get_pixel(ring_x, g.cy.round() as u32)[3], 0);

        // Somewhere inside the disc is disc_fill or text_color, not the base.
        let mut disc_pixels = 0usize;
        for y in 0..72 {
            for x in 0..72 {
                let dx = x as f32 + 0.5 - g.cx;
                let dy = y as f32 + 0.5 - g.cy;
                if (dx * dx + dy * dy).sqrt() < g.r - 2.0 {
                    let p = *img.get_pixel(x, y);
                    assert_ne!(p, Rgba([10, 200, 30, 255]), "base survived inside the disc");
                    disc_pixels += 1;
                }
            }
        }
        assert!(disc_pixels > 0);

        // The opposite corner is untouched.
        assert_eq!(*img.get_pixel(0, 71), Rgba([10, 200, 30, 255]));
    }

    #[test]
    fn apply_badge_works_for_every_gravity() {
        for gravity in [
            Gravity::NorthWest,
            Gravity::North,
            Gravity::NorthEast,
            Gravity::West,
            Gravity::Center,
            Gravity::East,
            Gravity::SouthWest,
            Gravity::South,
            Gravity::SouthEast,
        ] {
            let mut img = opaque(72, 72);
            let spec = spec_at(gravity);
            let g = disc_geometry(72, 72, &spec);
            apply_badge(&mut img, &spec);

            let centre = *img.get_pixel(g.cx.round() as u32, g.cy.round() as u32);
            assert_ne!(
                centre,
                Rgba([10, 200, 30, 255]),
                "{gravity:?} left the base showing at the disc centre"
            );
        }
    }

    /// Pins the knockout → disc → mark order, which the other apply_badge tests
    /// cannot: they assert only `pixel != base`, and `knockout` zeroing alpha
    /// satisfies that on its own. So deleting `fill_disc`, or running it before
    /// `knockout`, leaves them green.
    ///
    /// An opaque disc-coloured pixel inside `r` can only exist if `fill_disc` ran
    /// AFTER `knockout` — run before, its alpha would have been zeroed to 0; not run
    /// at all, no pixel carries the fill colour.
    #[test]
    fn apply_badge_fills_the_disc_after_knocking_it_out() -> Result<(), String> {
        let mut img = opaque(72, 72);
        let spec = spec_at(Gravity::NorthEast);
        let g = disc_geometry(72, 72, &spec);

        apply_badge(&mut img, &spec);

        // Some pixel strictly inside the disc is opaque and carries disc_fill.
        let found = (0..72)
            .flat_map(|y| (0..72).map(move |x| (x, y)))
            .filter(|&(x, y)| circle_coverage(x, y, g.cx, g.cy, g.r) >= 1.0)
            .any(|(x, y)| *img.get_pixel(x, y) == spec.disc_fill);
        assert!(
            found,
            "no fully-covered pixel carries disc_fill — fill_disc was skipped or ran before knockout"
        );

        // And the separating ring is still transparent, so knockout ran and was not
        // painted over.
        let ring_x = (g.cx + (g.r + g.knock_r) / 2.0).round() as u32;
        assert_eq!(
            img.get_pixel(ring_x, g.cy.round() as u32)[3],
            0,
            "the clearance ring is not transparent"
        );

        Ok(())
    }

    #[test]
    fn apply_badge_accepts_a_logo_mark() {
        let mut img = opaque(72, 72);
        let logo = RgbaImage::from_pixel(64, 64, Rgba([255, 0, 255, 255]));
        let spec = BadgeSpec::new(Mark::Logo(logo));
        let g = disc_geometry(72, 72, &spec);

        apply_badge(&mut img, &spec);

        assert_eq!(
            *img.get_pixel(g.cx.round() as u32, g.cy.round() as u32),
            Rgba([255, 0, 255, 255])
        );
    }

    #[test]
    fn apply_badge_preserves_dimensions() {
        let mut img = opaque(100, 50);
        apply_badge(&mut img, &spec_at(Gravity::NorthEast));
        assert_eq!(img.dimensions(), (100, 50));
    }
}
