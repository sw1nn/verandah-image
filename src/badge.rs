//! Badge composition: a mark on a filled disc, knocked out of the base artwork.

use std::str::FromStr;

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
