//! Low-level pixel helpers, also useful for custom [`Element`](crate::Element)s.
//!
//! All colors use straight (non-premultiplied) alpha and blend with source-over.

use image::RgbaImage;

use crate::{Color, Rect};

/// Blends `src` over the RGBA pixel `dst`.
#[inline]
pub fn blend(dst: &mut [u8], src: [u8; 4]) {
    let sa = src[3] as u32;
    if sa == 0 {
        return;
    }
    if sa == 255 {
        dst[..4].copy_from_slice(&src);
        return;
    }
    let da = dst[3] as u32;
    // output alpha, scaled by 255
    let oa = sa * 255 + da * (255 - sa);
    if oa == 0 {
        return;
    }
    for i in 0..3 {
        dst[i] = ((src[i] as u32 * sa * 255 + dst[i] as u32 * da * (255 - sa) + oa / 2) / oa) as u8;
    }
    dst[3] = ((oa + 127) / 255) as u8;
}

/// Blends `color` at `(x, y)`; pixels outside the canvas are ignored.
#[inline]
pub fn blend_pixel(canvas: &mut RgbaImage, x: i64, y: i64, color: [u8; 4]) {
    if x < 0 || y < 0 || x >= canvas.width() as i64 || y >= canvas.height() as i64 {
        return;
    }
    blend(&mut canvas.get_pixel_mut(x as u32, y as u32).0, color);
}

/// Fills `rect` (rounded to whole pixels, clipped to the canvas).
pub fn fill_rect(canvas: &mut RgbaImage, rect: Rect, color: Color) {
    if color.a == 0 {
        return;
    }
    let Some((x0, y0, x1, y1)) = rect.pixel_bounds(canvas.width(), canvas.height()) else {
        return;
    };
    let src = color.to_array();
    let stride = canvas.width() as usize * 4;
    let raw: &mut [u8] = canvas;
    for y in y0 as usize..y1 as usize {
        let row = &mut raw[y * stride + x0 as usize * 4..y * stride + x1 as usize * 4];
        if color.a == 255 {
            for px in row.as_chunks_mut::<4>().0 {
                *px = src;
            }
        } else {
            for px in row.as_chunks_mut::<4>().0 {
                blend(px, src);
            }
        }
    }
}

/// Fills a box with optional rounded corners and border, anti-aliased.
pub fn fill_box(
    canvas: &mut RgbaImage,
    rect: Rect,
    fill: Option<Color>,
    border: Option<(f32, Color)>,
    radius: f32,
) {
    let border = border.filter(|(w, c)| *w > 0.0 && c.a > 0);
    if radius <= 0.0 && border.is_none() {
        if let Some(fill) = fill {
            fill_rect(canvas, rect, fill);
        }
        return;
    }
    let bounds = Rect::new(
        rect.x - 1.0,
        rect.y - 1.0,
        rect.width + 2.0,
        rect.height + 2.0,
    );
    let Some((x0, y0, x1, y1)) = bounds.pixel_bounds(canvas.width(), canvas.height()) else {
        return;
    };
    let radius = radius.min(rect.width / 2.0).min(rect.height / 2.0).max(0.0);
    let (cx, cy) = (rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
    let outer = (rect.width / 2.0, rect.height / 2.0, radius);
    let inner = border.map(|(w, _)| {
        (
            (rect.width / 2.0 - w).max(0.0),
            (rect.height / 2.0 - w).max(0.0),
            (radius - w).max(0.0),
        )
    });
    // Pixels at least this far inside the edges are fully covered by the fill and untouched
    // by the border; only the remaining edge and corner pixels need anti-aliasing.
    let bw = border.map_or(0.0, |(w, _)| w);
    let solid = |inset: f32| {
        Rect::new(
            rect.x + inset,
            rect.y + inset,
            rect.width - 2.0 * inset,
            rect.height - 2.0 * inset,
        )
    };
    let edge = solid(bw + 1.0);
    let corner = solid(bw + radius + 1.0);
    let fill_src = fill.map(Color::to_array);
    for y in y0..y1 {
        let yc = y as f32 + 0.5;
        let interior = if yc > corner.y && yc < corner.bottom() {
            Some((edge.x, edge.right()))
        } else if yc > edge.y && yc < edge.bottom() {
            Some((corner.x, corner.right()))
        } else {
            None
        };
        let (ix0, ix1) = interior.map_or((x1, x1), |(l, r)| {
            (
                l.ceil().clamp(x0 as f32, x1 as f32) as u32,
                r.floor().clamp(x0 as f32, x1 as f32) as u32,
            )
        });
        if let (Some(src), true) = (fill_src, ix0 < ix1) {
            let stride = canvas.width() as usize * 4;
            let raw: &mut [u8] = canvas;
            let row = &mut raw
                [y as usize * stride + ix0 as usize * 4..y as usize * stride + ix1 as usize * 4];
            for px in row.as_chunks_mut::<4>().0 {
                blend(px, src);
            }
        }
        for x in (x0..x1).filter(|x| *x < ix0 || *x >= ix1) {
            let (px, py) = (x as f32 + 0.5 - cx, yc - cy);
            let outer_cov = coverage(px, py, outer);
            if outer_cov <= 0.0 {
                continue;
            }
            let inner_cov = inner.map_or(outer_cov, |i| coverage(px, py, i));
            let dst = &mut canvas.get_pixel_mut(x, y).0;
            if let Some(fill) = fill {
                blend(dst, with_coverage(fill, inner_cov));
            }
            if let Some((_, color)) = border {
                blend(dst, with_coverage(color, outer_cov - inner_cov));
            }
        }
    }
}

/// Coverage of a pixel centered at `(px, py)` (relative to the box center) by a rounded box.
fn coverage(px: f32, py: f32, (half_w, half_h, r): (f32, f32, f32)) -> f32 {
    let qx = px.abs() - (half_w - r);
    let qy = py.abs() - (half_h - r);
    let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
    let dist = outside + qx.max(qy).min(0.0) - r;
    (0.5 - dist).clamp(0.0, 1.0)
}

fn with_coverage(color: Color, coverage: f32) -> [u8; 4] {
    let a = (color.a as f32 * coverage.clamp(0.0, 1.0)).round() as u8;
    [color.r, color.g, color.b, a]
}

/// Blends `image` onto the canvas with its top-left corner at `(x, y)`.
pub fn draw_image(canvas: &mut RgbaImage, image: &RgbaImage, x: i64, y: i64) {
    let (cw, ch) = (canvas.width() as i64, canvas.height() as i64);
    let sx0 = (-x).max(0);
    let sy0 = (-y).max(0);
    let sx1 = (image.width() as i64).min(cw - x);
    let sy1 = (image.height() as i64).min(ch - y);
    for sy in sy0..sy1 {
        for sx in sx0..sx1 {
            let src = image.get_pixel(sx as u32, sy as u32).0;
            blend(
                &mut canvas.get_pixel_mut((x + sx) as u32, (y + sy) as u32).0,
                src,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blend_rules() {
        let mut px = [10, 20, 30, 255];
        blend(&mut px, [200, 100, 0, 255]);
        assert_eq!(px, [200, 100, 0, 255]);
        blend(&mut px, [0, 0, 0, 0]);
        assert_eq!(px, [200, 100, 0, 255]);
        let mut px = [0, 0, 0, 0];
        blend(&mut px, [255, 0, 0, 128]);
        assert_eq!(
            px,
            [255, 0, 0, 128],
            "over transparent keeps the source color"
        );
        let mut px = [255, 255, 255, 255];
        blend(&mut px, [0, 0, 0, 128]);
        assert_eq!(px, [127, 127, 127, 255]);
    }

    #[test]
    fn rounded_box_corners() {
        let mut img = RgbaImage::new(40, 40);
        fill_box(
            &mut img,
            Rect::new(0.0, 0.0, 40.0, 40.0),
            Some(Color::BLACK),
            None,
            10.0,
        );
        assert_eq!(
            img.get_pixel(0, 0).0[3],
            0,
            "corner outside the radius stays empty"
        );
        assert_eq!(img.get_pixel(20, 20).0, [0, 0, 0, 255]);
        assert_eq!(img.get_pixel(20, 0).0[3], 255, "edge midpoint is covered");
    }

    #[test]
    fn border_only() {
        let mut img = RgbaImage::new(20, 20);
        fill_box(
            &mut img,
            Rect::new(0.0, 0.0, 20.0, 20.0),
            None,
            Some((2.0, Color::WHITE)),
            0.0,
        );
        assert_eq!(img.get_pixel(0, 10).0, [255, 255, 255, 255]);
        assert_eq!(
            img.get_pixel(10, 10).0[3],
            0,
            "inside of a border-only box stays empty"
        );
    }

    #[test]
    fn image_clipping() {
        let mut canvas = RgbaImage::new(4, 4);
        let img = RgbaImage::from_pixel(3, 3, image::Rgba([9, 9, 9, 255]));
        draw_image(&mut canvas, &img, -1, 2);
        assert_eq!(canvas.get_pixel(0, 2).0, [9, 9, 9, 255]);
        assert_eq!(canvas.get_pixel(2, 2).0[3], 0);
        assert_eq!(canvas.get_pixel(0, 1).0[3], 0);
    }
}
