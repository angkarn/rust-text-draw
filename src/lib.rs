pub use cosmic_text::{
    self, fontdb, Align, Attrs, Buffer, Color, FontSystem, Metrics, Shaping, SwashCache,
};
pub use image::{self, DynamicImage, GenericImage, GenericImageView, ImageFormat, Rgba};
pub use serde::Deserialize;

#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct Widget {
    a: Option<u8>,           // align text (x axis) 0,1,2
    p: Option<u8>,           // align widget (y axis) 0,1,2
    x: Option<f32>,          // position x axis
    y: Option<f32>,          // position x axis
    w: Option<u32>,          // width
    h: Option<u32>,          // height
    f: Option<u32>,          // font
    fs: Option<f32>,         // font size
    c: Option<String>,       // font color
    fill: Option<String>,    // fill color only w,h
    mlh: Option<f32>,        // multiply line hight (default: 1.5)
    ts: Option<Vec<Texts>>,  // texts item
    wi: Option<Vec<Widget>>, // widget item
    t: Option<String>,       // text
    ml: Option<u32>,         // margin left
    mt: Option<u32>,         // margin top
    mr: Option<u32>,         // margin right
    mb: Option<u32>,         // margin bottom
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct Texts {
    f: Option<u32>,    // font
    fs: Option<f32>,   // font size
    c: Option<String>, // font color
    t: String,         // text
}

pub struct ResultDrawText {
    pub count_pixel_out: u32,
}

fn draw_item(
    swash_cache: &mut SwashCache,
    font_system: &mut FontSystem,
    image: &mut DynamicImage,
    start_x: f32,
    start_y: f32,
    text_layout_width: u32,
    text_layout_height: u32,
    widgets: Vec<Widget>,
    default_font_family: u32,
    default_font_size: f32,
    default_color: &String,
    skip_size_check: bool,
    faces: Vec<String>,
) -> Result<ResultDrawText, String> {
    let mut count_pixel_out = 0;
    for widget in &widgets {
        let widget_margin_left = widget.ml.unwrap_or(0);
        let widget_margin_top = widget.mt.unwrap_or(0);
        let widget_margin_right = widget.mr.unwrap_or(0);
        let widget_margin_bottom = widget.mb.unwrap_or(0);

        let widget_x = widget.x.unwrap_or(0.0) + start_x + widget_margin_left as f32;
        let widget_y = widget.y.unwrap_or(0.0) + start_y + widget_margin_top as f32;
        let widget_a = widget.a.unwrap_or(0);
        let widget_p = widget.p.unwrap_or(0);

        let widget_width =
            widget.w.unwrap_or(text_layout_width) - widget_margin_left - widget_margin_right;
        let widget_height =
            widget.h.unwrap_or(text_layout_height) - widget_margin_top - widget_margin_bottom;
        let widget_font_size = widget.fs.unwrap_or(default_font_size);
        let widget_mlh = widget.mlh.unwrap_or(1.5);

        // Text metrics indicate the font size and line height of a buffer
        let metrics = Metrics::new(widget_font_size, widget_font_size * widget_mlh);

        // A Buffer provides shaping and layout for a UTF-8 string, create one per text widget
        let mut buffer = Buffer::new_empty(metrics.scale(2.0));
        // let mut buffer = Buffer::new_empty(metrics.scale(1.0));

        // Borrow buffer together with the font system for more convenient method calls
        let mut buffer = buffer.borrow_with(font_system);

        // Set a size for the text buffer, in pixels
        buffer.set_size(Some(widget_width as f32), Some(widget_height as f32));

        // wrap text
        // buffer.set_wrap(cosmic_text::Wrap::None);

        // Attributes indicate what font to choose
        let attrs = Attrs::new();

        // Set widget color
        let widget_rgba: [u8; 4] = u32::from_str_radix(
            format!(
                "{}{}",
                widget.c.clone().unwrap_or(default_color.to_string()),
                "ff"
            )
            .as_str(),
            16,
        )
        .unwrap()
        .to_be_bytes();

        let widget_color = Color::rgba(
            widget_rgba[0],
            widget_rgba[1],
            widget_rgba[2],
            widget_rgba[3],
        );

        // Fill color widget
        if widget.fill.is_some() {
            let rgba_fill: [u8; 4] = u32::from_str_radix(
                format!("{}{}", widget.fill.clone().unwrap(), "ff").as_str(),
                16,
            )
            .unwrap()
            .to_be_bytes();

            // fill bg base image
            for y in 0..widget_height {
                for x in 0..widget_width {
                    // println!("x{} y{}", x, y);
                    image.put_pixel(x + widget_x as u32, y + widget_y as u32, Rgba(rgba_fill));
                }
            }
        }

        // Process items
        if widget.ts.is_some() {
            let mut spans = Vec::new();

            let widget_items = &widget.ts.clone().unwrap();
            for it in widget_items {
                if !it.t.is_empty() {
                    let font_id = it.f.unwrap_or(widget.f.unwrap_or(default_font_family));
                    let font_size = it.fs.unwrap_or(widget_font_size);

                    let color = match it.c.is_some() {
                        true => {
                            let rgba: [u8; 4] = u32::from_str_radix(
                                format!("{}{}", it.c.clone().unwrap(), "ff").as_str(),
                                16,
                            )
                            .unwrap()
                            .to_be_bytes();
                            Color::rgba(rgba[0], rgba[1], rgba[2], rgba[3])
                        }
                        false => widget_color,
                    };

                    let metrics = Metrics::new(font_size, font_size * widget_mlh);

                    spans.push((
                        it.t.as_str(),
                        attrs
                            .family(cosmic_text::Family::Name(
                                faces.get(font_id as usize).unwrap_or(faces.get(0).unwrap()),
                            ))
                            .metrics(metrics)
                            .color(color),
                    ))
                }
            }

            buffer.set_rich_text(spans.to_vec(), attrs, Shaping::Advanced);
        }

        // Sum size area
        let mut sum_width = 0.0;
        let mut sum_height = 0.0;

        for run in buffer.layout_runs() {
            if run.line_w > sum_width {
                sum_width = run.line_w
            }
            sum_height += run.line_height;
        }

        // println!("sum_height:{}",sum_height);

        // Inspect the output runs
        if !skip_size_check {
            if (widget_width as f32) < sum_width || (widget_height as f32) < sum_height {
                return Err(format!(
                    "Text size (w:{},h:{}) is more than layout size",
                    sum_width, sum_height
                ));
            }
        }

        // set align
        for buffer_line in buffer.lines.iter_mut() {
            if widget_a == 1 {
                buffer_line.set_align(Some(Align::Center));
            }
            if widget_a == 2 {
                buffer_line.set_align(Some(Align::Right));
            }
        }

        // Perform shaping as desired
        buffer.shape_until_scroll(true);

        // buffer.redraw();

        // Draw the buffer (for performance, instead use SwashCache directly)
        buffer.draw(swash_cache, widget_color, |d_x, d_y, d_w, d_h, color| {
            if color.a() == 0 || d_w != 1 || d_h != 1 || d_x < 0 || d_y < 0 {
                return;
            }

            let px = d_x as f32 + widget_x;
            let py = match widget_p {
                // TOP
                0 => d_y as f32 + widget_y,
                // Middle
                1 => {
                    (d_y as f32 + (widget_height / 2) as f32 - (sum_height / 2.0)) as f32 + widget_y
                }
                // Bottom
                2 => d_y as f32 + widget_y + widget_height as f32 - sum_height,
                _ => d_y as f32 + widget_y,
            };

            if px < image.width() as f32 && py < image.height() as f32 {
                let base_color = image.get_pixel(px as u32, py as u32);
                let new_alpha = color.a() as f32 / 255.0;
                let base_alpha = base_color[3] as f32 / 255.0;

                let scale = |dc: u8, bc: u8| {
                    (dc as f32 * new_alpha) + (bc as f32 * base_alpha * (1.0 - new_alpha))
                };

                let r = scale(color.r(), base_color[0]);
                let g = scale(color.g(), base_color[1]);
                let b = scale(color.b(), base_color[2]);
                let alpha = 255.0 * (new_alpha + base_alpha * (1.0 - new_alpha));

                image.put_pixel(
                    px as u32,
                    py as u32,
                    Rgba([r as u8, g as u8, b as u8, alpha as u8]),
                );

                // Scale by alpha (mimics blending with black)

                // let scale = |c: u8| (c as i32 * color.a() as i32 / 255).clamp(0, 255) as u8;

                // let r = scale(color.r());
                // let g = scale(color.g());
                // let b = scale(color.b());
                // // let a = scale(color.a());
                // let a = 255.0 * (new_alpha + base_alpha * (1.0 - new_alpha));
                // image.put_pixel(px, py, Rgba([r as u8, g as u8, b as u8, a as u8]));
            } else {
                println!("{:#?}", (d_x, d_y, d_w, d_h));
                count_pixel_out += 1;
            }
        });

        if widget.wi.is_some() {
            let result_draw_item = draw_item(
                swash_cache,
                font_system,
                image,
                widget_x,
                widget_y,
                widget_width,
                widget_height,
                widget.wi.clone().unwrap(),
                widget.f.unwrap_or(0),
                widget.fs.unwrap_or(default_font_size),
                &widget.c.clone().unwrap_or(default_color.to_string()),
                skip_size_check,
                faces.clone(),
            );

            if result_draw_item.is_ok() {
                count_pixel_out += result_draw_item.unwrap().count_pixel_out;
            }
        }
    }

    Ok(ResultDrawText {
        count_pixel_out: count_pixel_out,
    })
}

pub fn draw_text(
    swash_cache: &mut SwashCache,
    font_system: &mut FontSystem,
    image: &mut DynamicImage,
    start_x: f32,
    start_y: f32,
    text_layout_width: u32,
    text_layout_height: u32,
    widgets: Vec<Widget>,
    default_font_size: f32,
    default_color: &String,
    skip_size_check: bool,
) -> Result<ResultDrawText, String> {
    let faces = font_system
        .db_mut()
        .faces()
        .map(|x| x.families.get(0).unwrap().0.to_string())
        .collect::<Vec<String>>()
        .clone();

    let result = draw_item(
        swash_cache,
        font_system,
        image,
        start_x,
        start_y,
        text_layout_width,
        text_layout_height,
        widgets,
        0,
        default_font_size,
        default_color,
        skip_size_check,
        faces,
    );

    result
}
