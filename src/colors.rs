//! Color parsing module.
//!
//! Supports:
//! - CSS named colors (based on CSS Color Module Level 4)
//! - Hex colors: #RGB, #RRGGBB, #RRGGBBAA
//!
//! Reference: https://www.w3.org/TR/css-color-4/#named-colors

use std::collections::HashMap;

use image::Rgba;

/// Parse a color from a string.
///
/// Supports:
/// - CSS named colors (e.g., "red", "steelblue", "rebeccapurple")
/// - Hex with '#' prefix: #RGB, #RRGGBB, #RRGGBBAA
///
/// Alpha defaults to 0xFF if not specified.
/// Lookup is case-insensitive.
pub fn lookup<S>(s: S) -> Option<Rgba<u8>>
where
    S: AsRef<str>,
{
    let s = s.as_ref();

    // Try named color lookup first
    let lowercase = s.to_ascii_lowercase();
    if let Some(rgba) = NAMED_COLORS
        .iter()
        .find(|(n, _)| *n == lowercase)
        .map(|(_, rgba)| *rgba)
    {
        return Some(rgba);
    }

    // Fall back to hex parsing
    parse_hex(s)
}

/// Parse a hex color string.
///
/// Supports #RGB, #RRGGBB, and #RRGGBBAA formats.
/// Alpha defaults to 0xFF if not specified.
/// Returns None for invalid input.
fn parse_hex(s: &str) -> Option<Rgba<u8>> {
    let b = s.as_bytes();
    if b.first() != Some(&b'#') {
        return None;
    }
    match b.len() {
        4 => {
            // #RGB format - each digit is doubled, alpha = 0xFF
            let r = try_hex_digit(b[1])?;
            let g = try_hex_digit(b[2])?;
            let b = try_hex_digit(b[3])?;
            Some(Rgba([r * 17, g * 17, b * 17, 0xFF]))
        }
        7 => {
            // #RRGGBB format, alpha = 0xFF
            Some(Rgba([
                try_hex_digit(b[1])? * 16 + try_hex_digit(b[2])?,
                try_hex_digit(b[3])? * 16 + try_hex_digit(b[4])?,
                try_hex_digit(b[5])? * 16 + try_hex_digit(b[6])?,
                0xFF,
            ]))
        }
        9 => {
            // #RRGGBBAA format
            Some(Rgba([
                try_hex_digit(b[1])? * 16 + try_hex_digit(b[2])?,
                try_hex_digit(b[3])? * 16 + try_hex_digit(b[4])?,
                try_hex_digit(b[5])? * 16 + try_hex_digit(b[6])?,
                try_hex_digit(b[7])? * 16 + try_hex_digit(b[8])?,
            ]))
        }
        _ => None,
    }
}

// ----------------------------------------------------------------------------
// Compile-time hex parsing
// ----------------------------------------------------------------------------

/// Parse a single hex digit to u8, returning None for invalid digits.
const fn try_hex_digit(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'A'..=b'F' => Some(c - b'A' + 10),
        b'a'..=b'f' => Some(c - b'a' + 10),
        _ => None,
    }
}

/// Parse a single hex digit to u8 at compile time. Panics on invalid input.
const fn hex_digit(c: u8) -> u8 {
    match try_hex_digit(c) {
        Some(v) => v,
        None => panic!("invalid hex digit"),
    }
}

/// Parse a hex color string at compile time.
///
/// Supports #RGB, #RRGGBB, and #RRGGBBAA formats.
/// Alpha defaults to 0xFF if not specified.
/// Panics on invalid input.
pub const fn hex(s: &str) -> Rgba<u8> {
    let b = s.as_bytes();
    assert!(b[0] == b'#', "expected hex color with '#' prefix");
    match b.len() {
        4 => {
            // #RGB format - each digit is doubled, alpha = 0xFF
            let r = hex_digit(b[1]);
            let g = hex_digit(b[2]);
            let b = hex_digit(b[3]);
            Rgba([r * 17, g * 17, b * 17, 0xFF])
        }
        7 => {
            // #RRGGBB format, alpha = 0xFF
            Rgba([
                hex_digit(b[1]) * 16 + hex_digit(b[2]),
                hex_digit(b[3]) * 16 + hex_digit(b[4]),
                hex_digit(b[5]) * 16 + hex_digit(b[6]),
                0xFF,
            ])
        }
        9 => {
            // #RRGGBBAA format
            Rgba([
                hex_digit(b[1]) * 16 + hex_digit(b[2]),
                hex_digit(b[3]) * 16 + hex_digit(b[4]),
                hex_digit(b[5]) * 16 + hex_digit(b[6]),
                hex_digit(b[7]) * 16 + hex_digit(b[8]),
            ])
        }
        _ => panic!("expected #RGB, #RRGGBB, or #RRGGBBAA format"),
    }
}

/// Parse a HashMap of color strings to RGBA values.
///
/// Invalid colors are logged as warnings and skipped.
pub fn parse_colors(colors: &HashMap<String, String>) -> HashMap<String, Rgba<u8>> {
    let mut parsed = HashMap::new();
    for (key, value) in colors {
        if let Some(rgba) = lookup(value) {
            parsed.insert(key.clone(), rgba);
        } else {
            tracing::warn!(key, value, "Invalid color format");
        }
    }
    parsed
}

/// Get a color from a parsed color map, returning a default if not found.
pub fn get_color(colors: &HashMap<String, Rgba<u8>>, key: &str, default: Rgba<u8>) -> Rgba<u8> {
    colors.get(key).copied().unwrap_or(default)
}

const NUM_COLORS: usize = 148;

/// CSS named colors as hex strings (sorted alphabetically).
#[rustfmt::skip]
const NAMED_COLOR_DATA: [(&str, &str); NUM_COLORS] = [
    ("aliceblue",            "#F0F8FF"),
    ("antiquewhite",         "#FAEBD7"),
    ("aqua",                 "#00FFFF"),
    ("aquamarine",           "#7FFFD4"),
    ("azure",                "#F0FFFF"),
    ("beige",                "#F5F5DC"),
    ("bisque",               "#FFE4C4"),
    ("black",                "#000000"),
    ("blanchedalmond",       "#FFEBCD"),
    ("blue",                 "#0000FF"),
    ("blueviolet",           "#8A2BE2"),
    ("brown",                "#A52A2A"),
    ("burlywood",            "#DED887"),
    ("cadetblue",            "#5F9EA0"),
    ("chartreuse",           "#7FFF00"),
    ("chocolate",            "#D2691E"),
    ("coral",                "#FF7F50"),
    ("cornflowerblue",       "#6495ED"),
    ("cornsilk",             "#FFF8DC"),
    ("crimson",              "#DC143C"),
    ("cyan",                 "#00FFFF"),
    ("darkblue",             "#00008B"),
    ("darkcyan",             "#008B8B"),
    ("darkgoldenrod",        "#B8860B"),
    ("darkgray",             "#A9A9A9"),
    ("darkgreen",            "#006400"),
    ("darkgrey",             "#A9A9A9"),
    ("darkkhaki",            "#BDB76B"),
    ("darkmagenta",          "#8B008B"),
    ("darkolivegreen",       "#556B2F"),
    ("darkorange",           "#FF8C00"),
    ("darkorchid",           "#9932CC"),
    ("darkred",              "#8B0000"),
    ("darksalmon",           "#E9967A"),
    ("darkseagreen",         "#8FBC8F"),
    ("darkslateblue",        "#483D8B"),
    ("darkslategray",        "#2F4F4F"),
    ("darkslategrey",        "#2F4F4F"),
    ("darkturquoise",        "#00CED1"),
    ("darkviolet",           "#9400D3"),
    ("deeppink",             "#FF1493"),
    ("deepskyblue",          "#00BFFF"),
    ("dimgray",              "#696969"),
    ("dimgrey",              "#696969"),
    ("dodgerblue",           "#1E90FF"),
    ("firebrick",            "#B22222"),
    ("floralwhite",          "#FFFAF0"),
    ("forestgreen",          "#228B22"),
    ("fuchsia",              "#FF00FF"),
    ("gainsboro",            "#DCDCDC"),
    ("ghostwhite",           "#F8F8FF"),
    ("gold",                 "#FFD700"),
    ("goldenrod",            "#DAA520"),
    ("gray",                 "#808080"),
    ("green",                "#008000"),
    ("greenyellow",          "#ADFF2F"),
    ("grey",                 "#808080"),
    ("honeydew",             "#F0FFF0"),
    ("hotpink",              "#FF69B4"),
    ("indianred",            "#CD5C5C"),
    ("indigo",               "#4B0082"),
    ("ivory",                "#FFFFF0"),
    ("khaki",                "#F0E68C"),
    ("lavender",             "#E6E6FA"),
    ("lavenderblush",        "#FFF0F5"),
    ("lawngreen",            "#7CFC00"),
    ("lemonchiffon",         "#FFFACD"),
    ("lightblue",            "#ADD8E6"),
    ("lightcoral",           "#F08080"),
    ("lightcyan",            "#E0FFFF"),
    ("lightgoldenrodyellow", "#FAFAD2"),
    ("lightgray",            "#D3D3D3"),
    ("lightgreen",           "#90EE90"),
    ("lightgrey",            "#D3D3D3"),
    ("lightpink",            "#FFB6C1"),
    ("lightsalmon",          "#FFA07A"),
    ("lightseagreen",        "#20B2AA"),
    ("lightskyblue",         "#87CEFA"),
    ("lightslategray",       "#778899"),
    ("lightslategrey",       "#778899"),
    ("lightsteelblue",       "#B0C4DE"),
    ("lightyellow",          "#FFFFE0"),
    ("lime",                 "#00FF00"),
    ("limegreen",            "#32CD32"),
    ("linen",                "#FAF0E6"),
    ("magenta",              "#FF00FF"),
    ("maroon",               "#800000"),
    ("mediumaquamarine",     "#66CDAA"),
    ("mediumblue",           "#0000CD"),
    ("mediumorchid",         "#BA55D3"),
    ("mediumpurple",         "#9370DB"),
    ("mediumseagreen",       "#3CB371"),
    ("mediumslateblue",      "#7B68EE"),
    ("mediumspringgreen",    "#00FA9A"),
    ("mediumturquoise",      "#48D1CC"),
    ("mediumvioletred",      "#C71585"),
    ("midnightblue",         "#191970"),
    ("mintcream",            "#F5FFFA"),
    ("mistyrose",            "#FFE4E1"),
    ("moccasin",             "#FFE4B5"),
    ("navajowhite",          "#FFDEAD"),
    ("navy",                 "#000080"),
    ("oldlace",              "#FDF5E6"),
    ("olive",                "#808000"),
    ("olivedrab",            "#6B8E23"),
    ("orange",               "#FFA500"),
    ("orangered",            "#FF4500"),
    ("orchid",               "#DA70D6"),
    ("palegoldenrod",        "#EEE8AA"),
    ("palegreen",            "#98FB98"),
    ("paleturquoise",        "#AFEEEE"),
    ("palevioletred",        "#DB7093"),
    ("papayawhip",           "#FFEFD5"),
    ("peachpuff",            "#FFDAB9"),
    ("peru",                 "#CD853F"),
    ("pink",                 "#FFC0CB"),
    ("plum",                 "#DDA0DD"),
    ("powderblue",           "#B0E0E6"),
    ("purple",               "#800080"),
    ("rebeccapurple",        "#663399"),
    ("red",                  "#FF0000"),
    ("rosybrown",            "#BC8F8F"),
    ("royalblue",            "#4169E1"),
    ("saddlebrown",          "#8B4513"),
    ("salmon",               "#FA8072"),
    ("sandybrown",           "#F4A460"),
    ("seagreen",             "#2E8B57"),
    ("seashell",             "#FFF5EE"),
    ("sienna",               "#A0522D"),
    ("silver",               "#C0C0C0"),
    ("skyblue",              "#87CEEB"),
    ("slateblue",            "#6A5ACD"),
    ("slategray",            "#708090"),
    ("slategrey",            "#708090"),
    ("snow",                 "#FFFAFA"),
    ("springgreen",          "#00FF7F"),
    ("steelblue",            "#4682B4"),
    ("tan",                  "#D2B48C"),
    ("teal",                 "#008080"),
    ("thistle",              "#D8BFD8"),
    ("tomato",               "#FF6347"),
    ("turquoise",            "#40E0D0"),
    ("violet",               "#EE82EE"),
    ("wheat",                "#F5DEB3"),
    ("white",                "#FFFFFF"),
    ("whitesmoke",           "#F5F5F5"),
    ("yellow",               "#FFFF00"),
    ("yellowgreen",          "#9ACD32"),
];

/// CSS named colors with their RGBA values (parsed at compile time).
const NAMED_COLORS: [(&str, Rgba<u8>); NUM_COLORS] = {
    let mut result: [(&str, Rgba<u8>); NUM_COLORS] = [("", Rgba([0, 0, 0, 0])); NUM_COLORS];
    let mut i = 0;
    while i < NUM_COLORS {
        let (name, s) = NAMED_COLOR_DATA[i];
        result[i] = (name, hex(s));
        i += 1;
    }
    result
};

#[cfg(test)]
mod tests {
    use super::*;

    // Named color tests
    #[test]
    fn test_lookup_basic_colors() {
        assert_eq!(lookup("red"), Some(Rgba([255, 0, 0, 255])));
        assert_eq!(lookup("green"), Some(Rgba([0, 128, 0, 255])));
        assert_eq!(lookup("blue"), Some(Rgba([0, 0, 255, 255])));
        assert_eq!(lookup("white"), Some(Rgba([255, 255, 255, 255])));
        assert_eq!(lookup("black"), Some(Rgba([0, 0, 0, 255])));
    }

    #[test]
    fn test_lookup_case_insensitive() {
        assert_eq!(lookup("RED"), Some(Rgba([255, 0, 0, 255])));
        assert_eq!(lookup("Red"), Some(Rgba([255, 0, 0, 255])));
        assert_eq!(lookup("rEd"), Some(Rgba([255, 0, 0, 255])));
    }

    #[test]
    fn test_lookup_aliases() {
        // gray/grey
        assert_eq!(lookup("gray"), lookup("grey"));
        // aqua/cyan
        assert_eq!(lookup("aqua"), lookup("cyan"));
        // fuchsia/magenta
        assert_eq!(lookup("fuchsia"), lookup("magenta"));
    }

    #[test]
    fn test_lookup_rebeccapurple() {
        // CSS4 addition
        assert_eq!(
            lookup("rebeccapurple"),
            Some(Rgba([0x66, 0x33, 0x99, 0xFF]))
        );
    }

    // Hex color tests
    #[test]
    fn test_lookup_hex_rrggbb() {
        let rgba = lookup("#ff6b35").unwrap();
        assert_eq!(rgba, Rgba([255, 107, 53, 255]));
    }

    #[test]
    fn test_lookup_hex_rgb() {
        let rgba = lookup("#fab").unwrap();
        assert_eq!(rgba, Rgba([0xff, 0xaa, 0xbb, 0xff]));
    }

    #[test]
    fn test_lookup_hex_rrggbbaa() {
        let rgba = lookup("#ff6b3580").unwrap();
        assert_eq!(rgba, Rgba([255, 107, 53, 128]));
    }

    #[test]
    fn test_lookup_hex_black() {
        let rgba = lookup("#000000").unwrap();
        assert_eq!(rgba, Rgba([0, 0, 0, 255]));
    }

    #[test]
    fn test_lookup_hex_white() {
        let rgba = lookup("#ffffff").unwrap();
        assert_eq!(rgba, Rgba([255, 255, 255, 255]));
    }

    #[test]
    fn test_lookup_hex_requires_hash() {
        assert!(lookup("ff6b35").is_none());
        assert!(lookup("fab").is_none());
    }

    #[test]
    fn test_lookup_hex_invalid_length() {
        assert!(lookup("#ff").is_none());
        assert!(lookup("#ffff").is_none());
        assert!(lookup("#fffff").is_none());
        assert!(lookup("#fffffff").is_none());
        assert!(lookup("#fffffffff").is_none());
    }

    #[test]
    fn test_lookup_hex_invalid_hex() {
        assert!(lookup("#gggggg").is_none());
        assert!(lookup("#zzzzzz").is_none());
    }

    #[test]
    fn test_lookup_unknown() {
        assert!(lookup("notacolor").is_none());
        assert!(lookup("").is_none());
    }

    #[test]
    fn test_get_color_found() {
        let mut colors = HashMap::new();
        colors.insert("fg".to_string(), Rgba([255, 0, 0, 255]));
        assert_eq!(
            get_color(&colors, "fg", Rgba([0, 0, 0, 255])),
            Rgba([255, 0, 0, 255])
        );
    }

    #[test]
    fn test_get_color_not_found() {
        let colors = HashMap::new();
        assert_eq!(
            get_color(&colors, "fg", Rgba([0, 0, 0, 255])),
            Rgba([0, 0, 0, 255])
        );
    }

    // Const hex function tests
    #[test]
    fn test_const_hex_rgb() {
        const COLOR: Rgba<u8> = hex("#fab");
        assert_eq!(COLOR, Rgba([0xff, 0xaa, 0xbb, 0xff]));
    }

    #[test]
    fn test_const_hex_rrggbb() {
        const COLOR: Rgba<u8> = hex("#ff6b35");
        assert_eq!(COLOR, Rgba([255, 107, 53, 255]));
    }

    #[test]
    fn test_const_hex_rrggbbaa() {
        const COLOR: Rgba<u8> = hex("#ff6b3580");
        assert_eq!(COLOR, Rgba([255, 107, 53, 128]));
    }
}
