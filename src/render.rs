use std::{borrow::Cow, cell::RefCell, collections::HashMap, sync::Arc};

use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping, SwashCache};
use image::{imageops, RgbaImage};
use serde_json::{Map, Value};
use thread_local::ThreadLocal;

use crate::{
    paint, template::TemplateString, Align, Assets, BoxNode, Color, CustomNode, Element,
    ElementContext, Error, Fit, Fonts, ImageFit, ImageNode, Length, Node, NodeKind, Overflow, Rect,
    Result, Style, Template, TextNode, VAlign, Vars, Wrap,
};

/// Defaults for text that sets no style, and rendering behaviour.
#[derive(Clone, Debug)]
pub struct Options {
    pub font_size: Length,
    pub color: Color,
    pub line_height: f32,
    /// Return [`Error::Overflow`] instead of reporting it when text does not fit.
    pub strict: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            font_size: Length::Px(16.0),
            color: Color::BLACK,
            line_height: 1.25,
            strict: false,
        }
    }
}

/// Non-fatal problems found while rendering.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Report {
    /// Text nodes whose text did not fit their box.
    pub overflows: Vec<Overflow>,
}

/// Renders templates onto images.
///
/// A `Renderer` is `Send + Sync`: share one across threads. Each thread keeps its own font and
/// glyph caches, which stay warm between renders.
pub struct Renderer {
    fonts: Fonts,
    options: Options,
    elements: HashMap<String, Box<dyn Element>>,
    caches: ThreadLocal<RefCell<Cache>>,
}

struct Cache {
    font_system: FontSystem,
    swash: SwashCache,
    scaled: HashMap<ScaledKey, Arc<RgbaImage>>,
}

/// (source image address, crop x, y, w, h, output w, h)
type ScaledKey = (usize, [u32; 6]);
const SCALED_CACHE_LIMIT: usize = 64;

impl Renderer {
    pub fn new(fonts: Fonts) -> Self {
        Self {
            fonts,
            options: Options::default(),
            elements: HashMap::new(),
            caches: ThreadLocal::new(),
        }
    }

    pub fn with_options(mut self, options: Options) -> Self {
        self.options = options;
        self
    }

    /// Draws nodes with `type: kind` using `element`.
    pub fn register(
        &mut self,
        kind: impl Into<String>,
        element: impl Element + 'static,
    ) -> &mut Self {
        self.elements.insert(kind.into(), Box::new(element));
        self
    }

    pub fn fonts(&self) -> &Fonts {
        &self.fonts
    }

    pub fn options(&self) -> &Options {
        &self.options
    }

    /// Draws `template` onto `canvas`, filling `{{placeholders}}` from `vars`.
    pub fn render(
        &self,
        template: &Template,
        canvas: &mut RgbaImage,
        vars: &dyn Vars,
        assets: &Assets,
    ) -> Result<Report> {
        let cache = self.caches.get_or(|| {
            RefCell::new(Cache {
                font_system: FontSystem::new_with_locale_and_db(
                    "en-US".into(),
                    self.fonts.db.clone(),
                ),
                swash: SwashCache::new(),
                scaled: HashMap::new(),
            })
        });
        let mut cache = cache.borrow_mut();
        let canvas_size = (canvas.width() as f32, canvas.height() as f32);
        let root = Rect::new(0.0, 0.0, canvas_size.0, canvas_size.1);
        let mut cx = Ctx {
            canvas,
            canvas_size,
            vars,
            assets,
            cache: &mut cache,
            report: Report::default(),
        };
        self.draw_nodes(&mut cx, &template.nodes, root, &Style::default(), "nodes")?;
        Ok(cx.report)
    }

    fn draw_nodes(
        &self,
        cx: &mut Ctx,
        nodes: &[Node],
        parent: Rect,
        style: &Style,
        path: &str,
    ) -> Result<()> {
        for (i, node) in nodes.iter().enumerate() {
            let path = format!("{path}[{i}]");
            let f = &node.frame;
            let rect = Rect::new(
                parent.x + f.x.resolve(parent.width, cx.canvas_size),
                parent.y + f.y.resolve(parent.height, cx.canvas_size),
                f.width.resolve(parent.width, cx.canvas_size),
                f.height.resolve(parent.height, cx.canvas_size),
            );
            let content = rect.inset(&f.padding, cx.canvas_size);
            match &node.kind {
                NodeKind::Box(b) => self.draw_box(cx, b, rect, content, style, &path)?,
                NodeKind::Text(t) => self.draw_text(cx, t, content, style, &path)?,
                NodeKind::Image(img) => self.draw_image(cx, img, content, &path)?,
                NodeKind::Custom(c) => self.draw_custom(cx, c, content, &path)?,
            }
        }
        Ok(())
    }

    fn draw_box(
        &self,
        cx: &mut Ctx,
        b: &BoxNode,
        rect: Rect,
        content: Rect,
        style: &Style,
        path: &str,
    ) -> Result<()> {
        let border = b.border.as_ref().map(|border| {
            (
                border
                    .width
                    .resolve(rect.width.min(rect.height), cx.canvas_size),
                border.color,
            )
        });
        let radius = b.radius.map_or(0.0, |r| {
            r.resolve(rect.width.min(rect.height), cx.canvas_size)
        });
        paint::fill_box(cx.canvas, rect, b.fill, border, radius);
        let style = b.style.merged_over(style);
        self.draw_nodes(
            cx,
            &b.children,
            content,
            &style,
            &format!("{path}.children"),
        )
    }

    fn draw_text(
        &self,
        cx: &mut Ctx,
        t: &TextNode,
        rect: Rect,
        parent: &Style,
        path: &str,
    ) -> Result<()> {
        if rect.width <= 0.0 || rect.height <= 0.0 {
            return Ok(());
        }
        let base = t.style.merged_over(parent);
        let base_size = self.font_size(&base, cx.canvas_size);

        // Resolve spans: (text, family, size, color, line height)
        let plain = t
            .text
            .iter()
            .map(|text| (text, Style::default(), path.to_string()));
        let spans = t
            .spans
            .iter()
            .enumerate()
            .map(|(i, s)| (&s.text, s.style.clone(), format!("{path}.spans[{i}]")));
        let mut runs: Vec<Run> = Vec::new();
        for (text, style, path) in plain.chain(spans) {
            let style = style.merged_over(&base);
            runs.push(Run {
                text: render_str(text, cx.vars, &path)?,
                family: self.family(&style, &path)?,
                size: self.font_size(&style, cx.canvas_size),
                color: style.color.unwrap_or(self.options.color),
                line_height: style.line_height.unwrap_or(self.options.line_height),
            });
        }

        let fs = &mut cx.cache.font_system;
        let mut layout = TextLayout::new(fs, &runs, 1.0, rect.width, t.wrap, t.align);
        let fits = |l: &TextLayout| l.width <= rect.width + 0.5 && l.height <= rect.height + 0.5;
        if !fits(&layout) && t.fit == Fit::Shrink {
            // Binary search the largest scale (down to min_size) at which the text fits
            let min_size = t
                .min_size
                .map_or(base_size * 0.25, |m| m.resolve(0.0, cx.canvas_size));
            let (mut lo, mut hi) = ((min_size / base_size).min(1.0), 1.0_f32);
            layout = TextLayout::new(fs, &runs, lo, rect.width, t.wrap, t.align);
            if fits(&layout) {
                for _ in 0..8 {
                    let mid = (lo + hi) / 2.0;
                    let candidate = TextLayout::new(fs, &runs, mid, rect.width, t.wrap, t.align);
                    if fits(&candidate) {
                        (lo, layout) = (mid, candidate);
                    } else {
                        hi = mid;
                    }
                }
            }
        }
        if !fits(&layout) {
            let overflow = Overflow {
                path: path.to_string(),
                needed: (layout.width, layout.height),
                available: (rect.width, rect.height),
            };
            if self.options.strict {
                return Err(Error::Overflow(overflow));
            }
            cx.report.overflows.push(overflow);
        }

        let offset = match t.valign {
            VAlign::Top => 0.0,
            VAlign::Middle => (rect.height - layout.height) / 2.0,
            VAlign::Bottom => rect.height - layout.height,
        };
        let (ox, oy) = (rect.x.round() as i64, (rect.y + offset).round() as i64);
        let canvas = &mut *cx.canvas;
        let default_color = to_cosmic(self.options.color);
        layout.buffer.draw(
            fs,
            &mut cx.cache.swash,
            default_color,
            |x, y, w, h, color| {
                let rgba = [color.r(), color.g(), color.b(), color.a()];
                for dy in 0..h as i64 {
                    for dx in 0..w as i64 {
                        paint::blend_pixel(canvas, ox + x as i64 + dx, oy + y as i64 + dy, rgba);
                    }
                }
            },
        );
        Ok(())
    }

    fn draw_image(&self, cx: &mut Ctx, node: &ImageNode, rect: Rect, path: &str) -> Result<()> {
        let name = render_str(&node.src, cx.vars, path)?;
        let src = cx.assets.get(&name).ok_or_else(|| Error::MissingAsset {
            path: path.to_string(),
            name: name.to_string(),
        })?;
        let (iw, ih) = (src.width() as f32, src.height() as f32);
        if iw == 0.0 || ih == 0.0 || rect.width < 0.5 || rect.height < 0.5 {
            return Ok(());
        }
        // crop: source region; dest: output rect
        let (crop, dest) = match node.fit {
            ImageFit::Fill => ((0.0, 0.0, iw, ih), rect),
            ImageFit::Contain => {
                let s = (rect.width / iw).min(rect.height / ih);
                let (w, h) = (iw * s, ih * s);
                let dest = Rect::new(
                    rect.x + (rect.width - w) / 2.0,
                    rect.y + (rect.height - h) / 2.0,
                    w,
                    h,
                );
                ((0.0, 0.0, iw, ih), dest)
            }
            ImageFit::Cover => {
                let s = (rect.width / iw).max(rect.height / ih);
                let (cw, ch) = (rect.width / s, rect.height / s);
                (((iw - cw) / 2.0, (ih - ch) / 2.0, cw, ch), rect)
            }
        };
        let crop = [
            crop.0.round(),
            crop.1.round(),
            crop.2.round().max(1.0),
            crop.3.round().max(1.0),
        ]
        .map(|v| v as u32);
        let (dw, dh) = (
            dest.width.round().max(1.0) as u32,
            dest.height.round().max(1.0) as u32,
        );
        let key: ScaledKey = (
            Arc::as_ptr(src) as usize,
            [crop[0], crop[1], crop[2], crop[3], dw, dh],
        );
        let scaled = match cx.cache.scaled.get(&key) {
            Some(img) => img.clone(),
            None => {
                let view = imageops::crop_imm(src.as_ref(), crop[0], crop[1], crop[2], crop[3]);
                let img = Arc::new(imageops::resize(
                    &*view,
                    dw,
                    dh,
                    imageops::FilterType::Triangle,
                ));
                if cx.cache.scaled.len() >= SCALED_CACHE_LIMIT {
                    cx.cache.scaled.clear();
                }
                cx.cache.scaled.insert(key, img.clone());
                img
            }
        };
        paint::draw_image(
            cx.canvas,
            &scaled,
            dest.x.round() as i64,
            dest.y.round() as i64,
        );
        Ok(())
    }

    fn draw_custom(&self, cx: &mut Ctx, node: &CustomNode, rect: Rect, path: &str) -> Result<()> {
        let element = self
            .elements
            .get(&node.kind)
            .ok_or_else(|| Error::UnknownElement {
                path: path.to_string(),
                kind: node.kind.clone(),
            })?;
        let props = fill_props(&node.props, cx.vars, path)?;
        let mut ecx = ElementContext {
            canvas: cx.canvas,
            rect,
            props: &props,
        };
        element.draw(&mut ecx).map_err(|source| Error::Element {
            path: path.to_string(),
            kind: node.kind.clone(),
            source,
        })
    }

    fn family(&self, style: &Style, path: &str) -> Result<String> {
        let alias = match style.font.as_deref() {
            Some(alias) => alias,
            None => self.fonts.default_alias().ok_or(Error::NoFonts)?,
        };
        self.fonts
            .family(alias)
            .map(str::to_string)
            .ok_or_else(|| Error::UnknownFont {
                path: path.to_string(),
                font: alias.to_string(),
            })
    }

    fn font_size(&self, style: &Style, canvas: (f32, f32)) -> f32 {
        style
            .size
            .unwrap_or(self.options.font_size)
            .resolve(0.0, canvas)
            .max(0.1)
    }
}

struct Ctx<'a> {
    canvas: &'a mut RgbaImage,
    canvas_size: (f32, f32),
    vars: &'a dyn Vars,
    assets: &'a Assets,
    cache: &'a mut Cache,
    report: Report,
}

struct Run<'a> {
    text: Cow<'a, str>,
    family: String,
    size: f32,
    color: Color,
    line_height: f32,
}

struct TextLayout {
    buffer: Buffer,
    width: f32,
    height: f32,
}

impl TextLayout {
    fn new(
        fs: &mut FontSystem,
        runs: &[Run],
        scale: f32,
        width: f32,
        wrap: Wrap,
        align: Align,
    ) -> Self {
        let first = runs
            .first()
            .map_or((16.0, 1.25), |r| (r.size, r.line_height));
        let mut buffer =
            Buffer::new_empty(Metrics::new(first.0 * scale, first.0 * scale * first.1));
        buffer.set_wrap(match wrap {
            Wrap::Word => cosmic_text::Wrap::WordOrGlyph,
            Wrap::Glyph => cosmic_text::Wrap::Glyph,
            Wrap::None => cosmic_text::Wrap::None,
        });
        buffer.set_size(Some(width), None);
        let spans = runs.iter().map(|r| {
            let size = r.size * scale;
            let attrs = Attrs::new()
                .family(Family::Name(&r.family))
                .metrics(Metrics::new(size, size * r.line_height))
                .color(to_cosmic(r.color));
            (r.text.as_ref(), attrs)
        });
        let align = match align {
            Align::Left => cosmic_text::Align::Left,
            Align::Center => cosmic_text::Align::Center,
            Align::Right => cosmic_text::Align::Right,
            Align::Justify => cosmic_text::Align::Justified,
        };
        buffer.set_rich_text(spans, &Attrs::new(), Shaping::Advanced, Some(align));
        buffer.shape_until_scroll(fs, false);
        let (mut w, mut h) = (0.0_f32, 0.0_f32);
        for run in buffer.layout_runs() {
            w = w.max(run.line_w);
            h = h.max(run.line_top + run.line_height);
        }
        Self {
            buffer,
            width: w,
            height: h,
        }
    }
}

fn to_cosmic(c: Color) -> cosmic_text::Color {
    cosmic_text::Color::rgba(c.r, c.g, c.b, c.a)
}

fn render_str<'a>(s: &'a TemplateString, vars: &dyn Vars, path: &str) -> Result<Cow<'a, str>> {
    s.render(vars).map_err(|name| Error::MissingVar {
        path: path.to_string(),
        name,
    })
}

fn fill_props(
    props: &Map<String, Value>,
    vars: &dyn Vars,
    path: &str,
) -> Result<Map<String, Value>> {
    fn fill(v: &Value, vars: &dyn Vars, path: &str) -> Result<Value> {
        Ok(match v {
            Value::String(s) => {
                Value::String(render_str(&TemplateString::parse(s), vars, path)?.into_owned())
            }
            Value::Array(a) => Value::Array(
                a.iter()
                    .map(|v| fill(v, vars, path))
                    .collect::<Result<_>>()?,
            ),
            Value::Object(m) => Value::Object(fill_props(m, vars, path)?),
            other => other.clone(),
        })
    }
    props
        .iter()
        .map(|(k, v)| Ok((k.clone(), fill(v, vars, path)?)))
        .collect()
}
