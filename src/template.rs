use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap},
};

use serde::{de::DeserializeOwned, Deserialize};
use serde_json::{Map, Value};

use crate::{Color, Edges, Error, Length, Result};

/// A parsed template: a list of nodes drawn in order onto the canvas.
///
/// ```json5
/// [
///   { type: "box", fill: "#f4efe6", padding: "4vw", children: [
///     { type: "text", text: "Hello {{name}}", size: "6vw", align: "center" },
///   ]},
/// ]
/// ```
///
/// The top level may also be an object `{ nodes: [...] }`.
#[derive(Clone, Debug, PartialEq)]
pub struct Template {
    pub nodes: Vec<Node>,
}

impl Template {
    /// Parses a JSON5 template.
    pub fn from_json5(src: &str) -> Result<Self> {
        let value: Value = json5::from_str(src).map_err(|e| Error::Syntax(e.to_string()))?;
        Self::from_value(value)
    }

    /// Builds a template from an already parsed JSON value.
    pub fn from_value(value: Value) -> Result<Self> {
        let nodes = match value {
            Value::Array(nodes) => nodes,
            Value::Object(mut map) => match map.remove("nodes") {
                Some(Value::Array(nodes)) if map.is_empty() => nodes,
                Some(Value::Array(_)) => {
                    let keys = map.keys().cloned().collect::<Vec<_>>().join(", ");
                    return Err(template_err(
                        "template",
                        format!("unknown field(s): {keys}"),
                    ));
                }
                _ => return Err(template_err("template", "expected `nodes: [...]`")),
            },
            _ => return Err(template_err("template", "expected an array of nodes")),
        };
        Ok(Self {
            nodes: parse_nodes(nodes, "nodes")?,
        })
    }
}

/// A node: its frame (position and size inside the parent) and what it draws.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub frame: Frame,
    pub kind: NodeKind,
}

/// Position and size of a node, relative to its parent's content box.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Frame {
    pub x: Length,
    pub y: Length,
    pub width: Length,
    pub height: Length,
    /// Space between the node's edge and its content (children or text).
    pub padding: Edges,
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            x: Length::ZERO,
            y: Length::ZERO,
            width: Length::FULL,
            height: Length::FULL,
            padding: Edges::ZERO,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum NodeKind {
    Box(BoxNode),
    Text(TextNode),
    Image(ImageNode),
    /// Any other `type`, drawn by an [`Element`](crate::Element) registered under that name.
    Custom(CustomNode),
}

/// Text style; unset fields inherit from the parent box, then from [`Options`](crate::Options).
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Style {
    /// Font alias added with [`Fonts::add`](crate::Fonts::add).
    pub font: Option<String>,
    /// Font size. `%` is not allowed here; use `px`, `vw` or `vh`.
    pub size: Option<Length>,
    pub color: Option<Color>,
    /// Line height as a multiple of the font size.
    pub line_height: Option<f32>,
}

impl Style {
    pub(crate) fn merged_over(&self, parent: &Style) -> Style {
        Style {
            font: self.font.clone().or_else(|| parent.font.clone()),
            size: self.size.or(parent.size),
            color: self.color.or(parent.color),
            line_height: self.line_height.or(parent.line_height),
        }
    }
}

/// A rectangle that can be filled, bordered and rounded, and holds child nodes.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BoxNode {
    pub fill: Option<Color>,
    pub border: Option<Border>,
    pub radius: Option<Length>,
    /// Default text style for descendants.
    #[serde(skip)]
    pub style: Style,
    #[serde(skip)]
    pub children: Vec<Node>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Border {
    pub width: Length,
    pub color: Color,
}

/// Rich text laid out inside the node's content box.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextNode {
    /// Plain text. Use either `text` or `spans`.
    #[serde(default)]
    pub text: Option<TemplateString>,
    /// Runs of text with their own style.
    #[serde(default)]
    pub spans: Vec<Span>,
    #[serde(skip)]
    pub style: Style,
    #[serde(default)]
    pub align: Align,
    #[serde(default)]
    pub valign: VAlign,
    #[serde(default)]
    pub wrap: Wrap,
    #[serde(default)]
    pub fit: Fit,
    /// Smallest font size `fit: "shrink"` may use (default: 25% of the font size).
    #[serde(default)]
    pub min_size: Option<Length>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Span {
    pub text: TemplateString,
    pub style: Style,
}

impl<'de> Deserialize<'de> for Span {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error as _;
        let mut map = Map::<String, Value>::deserialize(deserializer)?;
        let text = match map.remove("text") {
            Some(Value::String(s)) => TemplateString::parse(&s),
            _ => return Err(D::Error::custom("span needs a `text` string")),
        };
        let style = serde_json::from_value(Value::Object(map)).map_err(D::Error::custom)?;
        Ok(Span { text, style })
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Align {
    #[default]
    Left,
    Center,
    Right,
    Justify,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VAlign {
    #[default]
    Top,
    Middle,
    Bottom,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Wrap {
    /// Break between words; a word longer than the line is broken between glyphs.
    #[default]
    Word,
    /// Break anywhere.
    Glyph,
    /// Keep each line on one line.
    None,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Fit {
    /// Draw at the given size; report an overflow if it does not fit.
    #[default]
    None,
    /// Reduce the font size until the text fits (down to `min_size`).
    Shrink,
}

/// An image from [`Assets`](crate::Assets), scaled into the node's content box.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageNode {
    /// Asset name.
    pub src: TemplateString,
    #[serde(default)]
    pub fit: ImageFit,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageFit {
    /// Scale to fit inside the box, keeping the aspect ratio.
    #[default]
    Contain,
    /// Scale to cover the box, keeping the aspect ratio (cropped).
    Cover,
    /// Stretch to the box.
    Fill,
}

/// A node with a `type` handled by a custom [`Element`](crate::Element).
#[derive(Clone, Debug, PartialEq)]
pub struct CustomNode {
    pub kind: String,
    /// All fields except the frame fields and `type`. Strings may contain `{{placeholders}}`.
    pub props: Map<String, Value>,
}

fn template_err(path: &str, message: impl Into<String>) -> Error {
    Error::Template {
        path: path.to_string(),
        message: message.into(),
    }
}

fn parse_nodes(values: Vec<Value>, path: &str) -> Result<Vec<Node>> {
    values
        .into_iter()
        .enumerate()
        .map(|(i, v)| parse_node(v, &format!("{path}[{i}]")))
        .collect()
}

const FRAME_KEYS: [&str; 5] = ["x", "y", "width", "height", "padding"];
const STYLE_KEYS: [&str; 4] = ["font", "size", "color", "line_height"];

/// Moves `keys` out of `map` into a new map.
fn take_keys(map: &mut Map<String, Value>, keys: &[&str]) -> Map<String, Value> {
    keys.iter()
        .filter_map(|k| map.remove(*k).map(|v| (k.to_string(), v)))
        .collect()
}

fn parse_node(value: Value, path: &str) -> Result<Node> {
    let Value::Object(mut map) = value else {
        return Err(template_err(path, "expected an object"));
    };
    let kind = match map.remove("type") {
        Some(Value::String(kind)) => kind,
        Some(_) => return Err(template_err(path, "`type` must be a string")),
        None => return Err(template_err(path, "missing `type`")),
    };

    let frame: Frame = from_map(take_keys(&mut map, &FRAME_KEYS), path)?;

    let kind = match kind.as_str() {
        "box" => {
            let children = match map.remove("children") {
                Some(Value::Array(c)) => parse_nodes(c, &format!("{path}.children"))?,
                Some(_) => return Err(template_err(path, "`children` must be an array")),
                None => Vec::new(),
            };
            let style = from_map(take_keys(&mut map, &STYLE_KEYS), path)?;
            let mut node: BoxNode = from_map(map, path)?;
            node.children = children;
            node.style = style;
            check_font_size(&node.style, path)?;
            NodeKind::Box(node)
        }
        "text" => {
            let style = from_map(take_keys(&mut map, &STYLE_KEYS), path)?;
            let mut node: TextNode = from_map(map, path)?;
            node.style = style;
            match (&node.text, node.spans.is_empty()) {
                (Some(_), false) => {
                    return Err(template_err(path, "use either `text` or `spans`, not both"))
                }
                (None, true) => return Err(template_err(path, "missing `text` or `spans`")),
                _ => {}
            }
            check_font_size(&node.style, path)?;
            for (i, span) in node.spans.iter().enumerate() {
                check_font_size(&span.style, &format!("{path}.spans[{i}]"))?;
            }
            if matches!(node.min_size, Some(Length::Percent(_))) {
                return Err(template_err(
                    path,
                    "`min_size` cannot be a percentage; use px, vw or vh",
                ));
            }
            NodeKind::Text(node)
        }
        "image" => NodeKind::Image(from_map(map, path)?),
        _ => NodeKind::Custom(CustomNode { kind, props: map }),
    };
    Ok(Node { frame, kind })
}

fn check_font_size(style: &Style, path: &str) -> Result<()> {
    if matches!(style.size, Some(Length::Percent(_))) {
        return Err(template_err(
            path,
            "font `size` cannot be a percentage; use px, vw or vh",
        ));
    }
    Ok(())
}

fn from_map<T: DeserializeOwned>(map: Map<String, Value>, path: &str) -> Result<T> {
    serde_json::from_value(Value::Object(map)).map_err(|e| template_err(path, e.to_string()))
}

/// A string that may contain `{{name}}` placeholders, parsed once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TemplateString {
    parts: Vec<Part>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Part {
    Literal(String),
    Var(String),
}

impl TemplateString {
    pub fn parse(s: &str) -> Self {
        let mut parts = Vec::new();
        let mut rest = s;
        while let Some(start) = rest.find("{{") {
            let Some(len) = rest[start + 2..].find("}}") else {
                break;
            };
            if start > 0 {
                parts.push(Part::Literal(rest[..start].to_string()));
            }
            parts.push(Part::Var(
                rest[start + 2..start + 2 + len].trim().to_string(),
            ));
            rest = &rest[start + 2 + len + 2..];
        }
        if !rest.is_empty() {
            parts.push(Part::Literal(rest.to_string()));
        }
        Self { parts }
    }

    /// Names of the placeholders in this string.
    pub fn vars(&self) -> impl Iterator<Item = &str> {
        self.parts.iter().filter_map(|p| match p {
            Part::Var(name) => Some(name.as_str()),
            Part::Literal(_) => None,
        })
    }

    /// Fills in the placeholders. Values are inserted as-is (never re-parsed).
    pub fn render<'a>(&'a self, vars: &dyn Vars) -> std::result::Result<Cow<'a, str>, String> {
        match self.parts.as_slice() {
            [] => Ok(Cow::Borrowed("")),
            [Part::Literal(s)] => Ok(Cow::Borrowed(s)),
            parts => {
                let mut out = String::new();
                for part in parts {
                    match part {
                        Part::Literal(s) => out.push_str(s),
                        Part::Var(name) => {
                            out.push_str(&vars.get(name).ok_or_else(|| name.clone())?)
                        }
                    }
                }
                Ok(Cow::Owned(out))
            }
        }
    }
}

impl<'de> Deserialize<'de> for TemplateString {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self::parse(&String::deserialize(deserializer)?))
    }
}

/// Values for `{{placeholders}}`.
pub trait Vars {
    fn get(&self, name: &str) -> Option<Cow<'_, str>>;
}

/// No variables: any placeholder is an error.
impl Vars for () {
    fn get(&self, _: &str) -> Option<Cow<'_, str>> {
        None
    }
}

impl<K: std::borrow::Borrow<str> + std::hash::Hash + Eq, V: AsRef<str>> Vars for HashMap<K, V> {
    fn get(&self, name: &str) -> Option<Cow<'_, str>> {
        HashMap::get(self, name).map(|v| Cow::Borrowed(v.as_ref()))
    }
}

impl<K: std::borrow::Borrow<str> + Ord, V: AsRef<str>> Vars for BTreeMap<K, V> {
    fn get(&self, name: &str) -> Option<Cow<'_, str>> {
        BTreeMap::get(self, name).map(|v| Cow::Borrowed(v.as_ref()))
    }
}

impl<V: Vars + ?Sized> Vars for &V {
    fn get(&self, name: &str) -> Option<Cow<'_, str>> {
        (**self).get(name)
    }
}

/// Variables from a closure, e.g. `VarsFn(|name| row.get(name).cloned())`.
pub struct VarsFn<F>(pub F);

impl<F: Fn(&str) -> Option<String>> Vars for VarsFn<F> {
    fn get(&self, name: &str) -> Option<Cow<'_, str>> {
        (self.0)(name).map(Cow::Owned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_string_placeholders() {
        let t = TemplateString::parse("Hi {{ name }}, row {{ROW}}!");
        assert_eq!(t.vars().collect::<Vec<_>>(), ["name", "ROW"]);
        let vars: HashMap<&str, &str> = [("name", "O'Brien {{ROW}}"), ("ROW", "7")].into();
        // values are inserted verbatim, never re-parsed
        assert_eq!(t.render(&vars).unwrap(), "Hi O'Brien {{ROW}}, row 7!");
        assert_eq!(t.render(&()).unwrap_err(), "name");
        assert_eq!(
            TemplateString::parse("a {{ b").render(&()).unwrap(),
            "a {{ b"
        );
    }

    #[test]
    fn parses_nested_template() {
        let t = Template::from_json5(
            r##"[{ type: "box", fill: "#fff", padding: [1, 2], font: "a", children: [
                 { type: "text", x: "10%", text: "x", align: "center", size: "3vw" },
                 { type: "qr", data: "{{1}}", width: 100 },
               ]}]"##,
        )
        .unwrap();
        let NodeKind::Box(b) = &t.nodes[0].kind else {
            panic!()
        };
        assert_eq!(b.style.font.as_deref(), Some("a"));
        assert_eq!(b.children.len(), 2);
        assert_eq!(b.children[0].frame.x, Length::Percent(10.0));
        let NodeKind::Custom(c) = &b.children[1].kind else {
            panic!()
        };
        assert_eq!(c.kind, "qr");
        assert_eq!(b.children[1].frame.width, Length::Px(100.0));
        assert!(c.props.contains_key("data") && !c.props.contains_key("width"));
    }

    #[test]
    fn errors_have_paths() {
        let err = |src: &str| Template::from_json5(src).unwrap_err().to_string();
        assert!(
            err(r#"[{ type: "box", children: [{ type: "text", txt: "x" }] }]"#)
                .contains("nodes[0].children[0]")
        );
        assert!(err(r#"[{ type: "text", text: "x", color: "nope" }]"#).contains("invalid color"));
        assert!(err(r#"[{ type: "text", text: "x", size: "10%" }]"#).contains("percentage"));
        assert!(err(r#"[{ text: "x" }]"#).contains("missing `type`"));
        assert!(err(r#"[{ type: "text" }]"#).contains("missing `text`"));
        assert!(err("[").contains("syntax"));
    }
}
