use std::str::FromStr;

use serde::{Deserialize, Deserializer};

use crate::Error;

/// A length in a template.
///
/// Written as a number (pixels) or a string with a unit:
/// `"12px"`, `"50%"` (of the parent box along the same axis),
/// `"3vw"` / `"3vh"` (percent of the canvas width / height).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Length {
    Px(f32),
    Percent(f32),
    Vw(f32),
    Vh(f32),
}

impl Length {
    pub const ZERO: Length = Length::Px(0.0);
    pub const FULL: Length = Length::Percent(100.0);

    /// Resolves to pixels. `parent` is the parent box size along the same axis.
    pub fn resolve(self, parent: f32, canvas: (f32, f32)) -> f32 {
        match self {
            Length::Px(v) => v,
            Length::Percent(v) => parent * v / 100.0,
            Length::Vw(v) => canvas.0 * v / 100.0,
            Length::Vh(v) => canvas.1 * v / 100.0,
        }
    }
}

impl FromStr for Length {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Error> {
        let t = s.trim();
        let (num, make): (&str, fn(f32) -> Length) = if let Some(n) = t.strip_suffix("px") {
            (n, Length::Px)
        } else if let Some(n) = t.strip_suffix('%') {
            (n, Length::Percent)
        } else if let Some(n) = t.strip_suffix("vw") {
            (n, Length::Vw)
        } else if let Some(n) = t.strip_suffix("vh") {
            (n, Length::Vh)
        } else {
            (t, Length::Px)
        };
        match num.trim().parse::<f32>() {
            Ok(v) if v.is_finite() => Ok(make(v)),
            _ => Err(Error::InvalidLength(s.to_string())),
        }
    }
}

impl<'de> Deserialize<'de> for Length {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Num(f32),
            Str(String),
        }
        match Raw::deserialize(deserializer)? {
            Raw::Num(v) => Ok(Length::Px(v)),
            Raw::Str(s) => s.parse().map_err(serde::de::Error::custom),
        }
    }
}

/// Space inside a box: `padding: 10`, `padding: [10, 20]` (vertical, horizontal)
/// or `padding: [top, right, bottom, left]`, as in CSS.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Edges {
    pub top: Length,
    pub right: Length,
    pub bottom: Length,
    pub left: Length,
}

impl Edges {
    pub const ZERO: Edges = Edges::all(Length::ZERO);

    pub const fn all(v: Length) -> Self {
        Self {
            top: v,
            right: v,
            bottom: v,
            left: v,
        }
    }
}

impl Default for Edges {
    fn default() -> Self {
        Self::ZERO
    }
}

impl<'de> Deserialize<'de> for Edges {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            One(Length),
            Many(Vec<Length>),
        }
        match Raw::deserialize(deserializer)? {
            Raw::One(v) => Ok(Edges::all(v)),
            Raw::Many(v) => match v.as_slice() {
                [a] => Ok(Edges::all(*a)),
                [v, h] => Ok(Edges {
                    top: *v,
                    right: *h,
                    bottom: *v,
                    left: *h,
                }),
                [t, r, b, l] => Ok(Edges {
                    top: *t,
                    right: *r,
                    bottom: *b,
                    left: *l,
                }),
                _ => Err(serde::de::Error::custom("expected 1, 2 or 4 values")),
            },
        }
    }
}

/// A rectangle in canvas pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }

    /// Shrinks the rectangle by `edges` (sizes never go below zero).
    pub fn inset(&self, edges: &Edges, canvas: (f32, f32)) -> Rect {
        let top = edges.top.resolve(self.height, canvas);
        let bottom = edges.bottom.resolve(self.height, canvas);
        let left = edges.left.resolve(self.width, canvas);
        let right = edges.right.resolve(self.width, canvas);
        Rect {
            x: self.x + left,
            y: self.y + top,
            width: (self.width - left - right).max(0.0),
            height: (self.height - top - bottom).max(0.0),
        }
    }

    /// Integer pixel bounds `(x0, y0, x1, y1)` (exclusive end), clipped to `width`x`height`.
    pub fn pixel_bounds(&self, width: u32, height: u32) -> Option<(u32, u32, u32, u32)> {
        let x0 = self.x.round().clamp(0.0, width as f32) as u32;
        let y0 = self.y.round().clamp(0.0, height as f32) as u32;
        let x1 = self.right().round().clamp(0.0, width as f32) as u32;
        let y1 = self.bottom().round().clamp(0.0, height as f32) as u32;
        (x0 < x1 && y0 < y1).then_some((x0, y0, x1, y1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lengths() {
        assert_eq!("12".parse::<Length>().unwrap(), Length::Px(12.0));
        assert_eq!("12px".parse::<Length>().unwrap(), Length::Px(12.0));
        assert_eq!(" 50% ".parse::<Length>().unwrap(), Length::Percent(50.0));
        assert_eq!("-2.5vw".parse::<Length>().unwrap(), Length::Vw(-2.5));
        assert_eq!("3vh".parse::<Length>().unwrap(), Length::Vh(3.0));
        assert!("12em".parse::<Length>().is_err());
        assert!("%".parse::<Length>().is_err());
    }

    #[test]
    fn resolves_lengths() {
        let canvas = (1000.0, 500.0);
        assert_eq!(Length::Px(7.0).resolve(200.0, canvas), 7.0);
        assert_eq!(Length::Percent(25.0).resolve(200.0, canvas), 50.0);
        assert_eq!(Length::Vw(10.0).resolve(200.0, canvas), 100.0);
        assert_eq!(Length::Vh(10.0).resolve(200.0, canvas), 50.0);
    }

    #[test]
    fn insets_and_clips() {
        let r = Rect::new(10.0, 10.0, 100.0, 50.0);
        let edges = Edges {
            top: Length::Px(5.0),
            right: Length::Percent(10.0),
            bottom: Length::ZERO,
            left: Length::Px(20.0),
        };
        assert_eq!(
            r.inset(&edges, (0.0, 0.0)),
            Rect::new(30.0, 15.0, 70.0, 45.0)
        );
        assert_eq!(
            Rect::new(-5.0, -5.0, 20.0, 20.0).pixel_bounds(10, 10),
            Some((0, 0, 10, 10))
        );
        assert_eq!(Rect::new(20.0, 0.0, 5.0, 5.0).pixel_bounds(10, 10), None);
    }
}
