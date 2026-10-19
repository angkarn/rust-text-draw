use image::RgbaImage;
use serde_json::{Map, Value};

use crate::{BoxError, Rect};

/// Draws nodes of a custom `type`, e.g. a QR code or a barcode.
///
/// Register it with [`Renderer::register`](crate::Renderer::register). It is called with the
/// node's content box and its fields (frame fields and `type` removed, `{{placeholders}}` filled in).
pub trait Element: Send + Sync {
    fn draw(&self, cx: &mut ElementContext<'_>) -> Result<(), BoxError>;
}

impl<F> Element for F
where
    F: Fn(&mut ElementContext<'_>) -> Result<(), BoxError> + Send + Sync,
{
    fn draw(&self, cx: &mut ElementContext<'_>) -> Result<(), BoxError> {
        self(cx)
    }
}

pub struct ElementContext<'a> {
    pub canvas: &'a mut RgbaImage,
    /// The node's content box in canvas pixels (may extend past the canvas).
    pub rect: Rect,
    pub props: &'a Map<String, Value>,
}

impl ElementContext<'_> {
    /// Reads a field, e.g. `cx.prop::<String>("data")`.
    pub fn prop<T: serde::de::DeserializeOwned>(&self, name: &str) -> Result<Option<T>, BoxError> {
        match self.props.get(name) {
            None | Some(Value::Null) => Ok(None),
            Some(v) => serde_json::from_value(v.clone())
                .map(Some)
                .map_err(|e| format!("field `{name}`: {e}").into()),
        }
    }
}
