use std::{fmt, str::FromStr};

use serde::{Deserialize, Deserializer};

use crate::Error;

/// An RGBA color with straight (non-premultiplied) alpha.
///
/// Parsed from `#rgb`, `#rgba`, `#rrggbb` or `#rrggbbaa`; the leading `#` is optional.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const BLACK: Color = Color::rgb(0, 0, 0);
    pub const WHITE: Color = Color::rgb(255, 255, 255);
    pub const TRANSPARENT: Color = Color::rgba(0, 0, 0, 0);

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn to_array(self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

impl From<[u8; 4]> for Color {
    fn from([r, g, b, a]: [u8; 4]) -> Self {
        Self { r, g, b, a }
    }
}

impl FromStr for Color {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Error> {
        let invalid = || Error::InvalidColor(s.to_string());
        let hex = s.trim().strip_prefix('#').unwrap_or(s.trim());
        if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(invalid());
        }
        let nibble = |i: usize| u8::from_str_radix(&hex[i..i + 1], 16).map(|v| v * 17);
        let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16);
        let parsed = match hex.len() {
            3 => (nibble(0), nibble(1), nibble(2), Ok(255)),
            4 => (nibble(0), nibble(1), nibble(2), nibble(3)),
            6 => (byte(0), byte(2), byte(4), Ok(255)),
            8 => (byte(0), byte(2), byte(4), byte(6)),
            _ => return Err(invalid()),
        };
        match parsed {
            (Ok(r), Ok(g), Ok(b), Ok(a)) => Ok(Self { r, g, b, a }),
            _ => Err(invalid()),
        }
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)?;
        if self.a != 255 {
            write!(f, "{:02x}", self.a)?;
        }
        Ok(())
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_forms() {
        assert_eq!("#fff".parse::<Color>().unwrap(), Color::WHITE);
        assert_eq!("000".parse::<Color>().unwrap(), Color::BLACK);
        assert_eq!(
            "#f008".parse::<Color>().unwrap(),
            Color::rgba(255, 0, 0, 0x88)
        );
        assert_eq!(
            "#336699".parse::<Color>().unwrap(),
            Color::rgb(0x33, 0x66, 0x99)
        );
        assert_eq!(
            "33669980".parse::<Color>().unwrap(),
            Color::rgba(0x33, 0x66, 0x99, 0x80)
        );
    }

    #[test]
    fn rejects_invalid() {
        for s in ["", "#12345", "#gggggg", "red", "#1234567890", "+12"] {
            assert!(s.parse::<Color>().is_err(), "{s:?} should be rejected");
        }
    }

    #[test]
    fn display_round_trips() {
        for s in ["#336699", "#33669980"] {
            assert_eq!(s.parse::<Color>().unwrap().to_string(), s);
        }
    }
}
