use std::fmt;

/// Errors returned by textplate.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The template is not valid JSON5.
    #[error("invalid template syntax: {0}")]
    Syntax(String),

    /// The template is valid JSON5 but does not describe a valid template.
    #[error("invalid template at {path}: {message}")]
    Template { path: String, message: String },

    /// A color string could not be parsed.
    #[error("invalid color {0:?}: expected #rgb, #rgba, #rrggbb or #rrggbbaa")]
    InvalidColor(String),

    /// A length string could not be parsed.
    #[error("invalid length {0:?}: expected a number (px) or a value ending in px, %, vw or vh")]
    InvalidLength(String),

    /// Font data could not be parsed.
    #[error("font {0:?} contains no usable font faces")]
    InvalidFont(String),

    /// A node refers to a font alias that was not added to [`Fonts`](crate::Fonts).
    #[error("unknown font {font:?} at {path}")]
    UnknownFont { path: String, font: String },

    /// No fonts were added, so text cannot be drawn.
    #[error("no fonts loaded")]
    NoFonts,

    /// A `{{placeholder}}` has no value in the supplied variables.
    #[error("missing variable {name:?} at {path}")]
    MissingVar { path: String, name: String },

    /// An image node refers to an asset that was not supplied.
    #[error("missing asset {name:?} at {path}")]
    MissingAsset { path: String, name: String },

    /// An encoded image could not be decoded.
    #[error("cannot decode asset {name:?}: {source}")]
    Decode {
        name: String,
        #[source]
        source: image::ImageError,
    },

    /// A node has a `type` with no registered [`Element`](crate::Element).
    #[error("unknown element type {kind:?} at {path}")]
    UnknownElement { path: String, kind: String },

    /// A custom [`Element`](crate::Element) failed.
    #[error("element {kind:?} at {path} failed: {source}")]
    Element {
        path: String,
        kind: String,
        #[source]
        source: BoxError,
    },

    /// Text did not fit its box and [`Options::strict`](crate::Options::strict) is on.
    #[error("{0}")]
    Overflow(Overflow),
}

/// Error type returned by custom elements.
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub type Result<T, E = Error> = std::result::Result<T, E>;

/// A non-fatal layout problem found while rendering.
#[derive(Debug, Clone, PartialEq)]
pub struct Overflow {
    /// Path of the node in the template, e.g. `nodes[1].children[0]`.
    pub path: String,
    /// Size the content needed.
    pub needed: (f32, f32),
    /// Size the node had.
    pub available: (f32, f32),
}

impl fmt::Display for Overflow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "text at {} needs {:.0}x{:.0}px but its box is {:.0}x{:.0}px",
            self.path, self.needed.0, self.needed.1, self.available.0, self.available.1
        )
    }
}
