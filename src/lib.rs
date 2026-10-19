//! Render data-driven templates of rich, multilingual text onto images.
//!
//! A [`Template`] is a tree of nodes written in JSON5: boxes (fill, border, rounded corners),
//! text (spans with their own font/size/color, wrapping, alignment, shrink-to-fit), images and
//! custom elements. Strings can contain `{{placeholders}}` filled from [`Vars`] at render time,
//! so one template renders many images.
//!
//! ```no_run
//! use textplate::{image::RgbaImage, Assets, Fonts, Renderer, Template};
//! use std::collections::HashMap;
//!
//! let mut fonts = Fonts::new();
//! fonts.add("body", std::fs::read("NotoSans-Regular.ttf")?)?;
//! let renderer = Renderer::new(fonts);
//!
//! let template = Template::from_json5(r##"[
//!   { type: "box", fill: "#1f2937", radius: 24, padding: 32, color: "#fff", children: [
//!     { type: "text", text: "Hello, {{name}}!", size: "8vw", align: "center", valign: "middle", fit: "shrink" },
//!   ]},
//! ]"##)?;
//!
//! let mut canvas = RgbaImage::new(800, 400);
//! let vars = HashMap::from([("name", "สวัสดี World")]);
//! let report = renderer.render(&template, &mut canvas, &vars, &Assets::new())?;
//! assert!(report.overflows.is_empty());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod assets;
mod color;
mod element;
mod error;
mod fonts;
mod geometry;
pub mod paint;
mod render;
mod template;

pub use assets::Assets;
pub use color::Color;
pub use element::{Element, ElementContext};
pub use error::{BoxError, Error, Overflow, Result};
pub use fonts::Fonts;
pub use geometry::{Edges, Length, Rect};
pub use render::{Options, Renderer, Report};
pub use template::{
    Align, Border, BoxNode, CustomNode, Fit, Frame, ImageFit, ImageNode, Node, NodeKind, Span,
    Style, Template, TemplateString, TextNode, VAlign, Vars, VarsFn, Wrap,
};

/// The `image` crate version used by textplate, for [`image::RgbaImage`].
pub use image;
