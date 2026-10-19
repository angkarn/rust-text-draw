use std::collections::HashMap;

use textplate::{
    image::{Rgba, RgbaImage},
    Assets, BoxError, ElementContext, Error, Fonts, Options, Rect, Renderer, Template,
};

fn fonts() -> Fonts {
    let mut fonts = Fonts::new();
    fonts
        .add(
            "body",
            std::fs::read("fonts/poppins-v21-latin-regular.ttf").unwrap(),
        )
        .unwrap();
    fonts
        .add(
            "thai",
            std::fs::read("fonts/noto-sans-thai-v25-thai-regular.ttf").unwrap(),
        )
        .unwrap();
    fonts
}

fn render(
    src: &str,
    size: (u32, u32),
    vars: &HashMap<&str, &str>,
) -> (RgbaImage, textplate::Report) {
    let renderer = Renderer::new(fonts());
    let template = Template::from_json5(src).unwrap();
    let mut canvas = RgbaImage::from_pixel(size.0, size.1, Rgba([255, 255, 255, 255]));
    let report = renderer
        .render(&template, &mut canvas, vars, &Assets::new())
        .unwrap();
    (canvas, report)
}

/// Bounding box `(x0, y0, x1, y1)` of non-white pixels.
fn ink_bounds(img: &RgbaImage) -> Option<(u32, u32, u32, u32)> {
    let mut b: Option<(u32, u32, u32, u32)> = None;
    for (x, y, p) in img.enumerate_pixels() {
        if p.0 != [255, 255, 255, 255] {
            b = Some(match b {
                None => (x, y, x, y),
                Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
            });
        }
    }
    b
}

#[test]
fn text_stays_inside_its_box() {
    let (img, report) = render(
        r#"[{ type: "text", x: 50, y: 40, width: 300, height: 100, text: "Hello world", size: 32 }]"#,
        (400, 200),
        &HashMap::new(),
    );
    assert!(report.overflows.is_empty());
    let (x0, y0, x1, y1) = ink_bounds(&img).expect("text drawn");
    assert!(
        x0 >= 50 && y0 >= 40 && x1 < 350 && y1 < 140,
        "ink {:?}",
        (x0, y0, x1, y1)
    );
}

#[test]
fn center_and_middle_alignment() {
    let (img, _) = render(
        r#"[{ type: "text", text: "Centered", size: 40, align: "center", valign: "middle" }]"#,
        (400, 200),
        &HashMap::new(),
    );
    let (x0, y0, x1, y1) = ink_bounds(&img).unwrap();
    let (cx, cy) = ((x0 + x1) as f32 / 2.0, (y0 + y1) as f32 / 2.0);
    assert!((cx - 200.0).abs() < 6.0, "horizontal center {cx}");
    assert!((cy - 100.0).abs() < 12.0, "vertical center {cy}");
}

#[test]
fn bottom_right_alignment() {
    let (img, _) = render(
        r#"[{ type: "text", text: "Corner", size: 30, align: "right", valign: "bottom" }]"#,
        (300, 150),
        &HashMap::new(),
    );
    let (_, _, x1, y1) = ink_bounds(&img).unwrap();
    assert!(x1 > 285, "right edge {x1}");
    assert!(y1 > 125, "bottom edge {y1}");
}

#[test]
fn shrink_to_fit() {
    let long = "This sentence is far too long to fit inside a small box at the requested size";
    let src = |fit: &str| {
        format!(
            r#"[{{ type: "text", x: 10, y: 10, width: 200, height: 60, size: 40, fit: "{fit}", text: "{long}" }}]"#
        )
    };
    let (_, report) = render(&src("none"), (220, 80), &HashMap::new());
    assert_eq!(report.overflows.len(), 1);
    assert_eq!(report.overflows[0].path, "nodes[0]");

    let (img, report) = render(&src("shrink"), (220, 80), &HashMap::new());
    assert!(report.overflows.is_empty(), "{:?}", report.overflows);
    let (x0, y0, x1, y1) = ink_bounds(&img).unwrap();
    assert!(
        x0 >= 10 && y0 >= 10 && x1 < 211 && y1 < 71,
        "ink {:?}",
        (x0, y0, x1, y1)
    );
}

#[test]
fn strict_mode_fails_on_overflow() {
    let renderer = Renderer::new(fonts()).with_options(Options {
        strict: true,
        ..Options::default()
    });
    let template = Template::from_json5(
        r#"[{ type: "text", width: 20, height: 10, size: 40, text: "overflow" }]"#,
    )
    .unwrap();
    let mut canvas = RgbaImage::new(100, 100);
    let err = renderer
        .render(&template, &mut canvas, &(), &Assets::new())
        .unwrap_err();
    assert!(matches!(err, Error::Overflow(_)), "{err}");
}

#[test]
fn placeholders_and_fallback_fonts() {
    // `O'Brien` and `{{` in values must not break anything; Thai falls back to the thai font
    let vars = HashMap::from([("name", "O'Brien {{x}}"), ("greeting", "สวัสดี")]);
    let (img, report) = render(
        r##"[{ type: "text", spans: [{ text: "{{greeting}} " }, { text: "{{name}}", color: "#c00" }], size: 30 }]"##,
        (500, 100),
        &vars,
    );
    assert!(report.overflows.is_empty());
    assert!(
        img.pixels().any(|p| p.0[0] > 150 && p.0[1] < 80),
        "red span drawn"
    );
    let (x0, _, x1, _) = ink_bounds(&img).unwrap();
    assert!(x1 - x0 > 150, "both spans drawn");
}

#[test]
fn reports_useful_errors() {
    let renderer = Renderer::new(fonts());
    let mut canvas = RgbaImage::new(50, 50);
    let run = |src: &str| {
        let t = Template::from_json5(src).unwrap();
        renderer
            .render(&t, &mut canvas.clone(), &(), &Assets::new())
            .unwrap_err()
            .to_string()
    };
    assert!(
        run(r#"[{ type: "text", text: "{{missing}}" }]"#).contains("missing variable \"missing\"")
    );
    assert!(
        run(r#"[{ type: "box", children: [{ type: "text", text: "x", font: "nope" }] }]"#)
            .contains("unknown font \"nope\" at nodes[0].children[0]")
    );
    assert!(run(r#"[{ type: "image", src: "logo" }]"#).contains("missing asset \"logo\""));
    assert!(run(r#"[{ type: "qr", data: "x" }]"#).contains("unknown element type \"qr\""));
    let _ = &mut canvas;
}

#[test]
fn custom_elements_get_rect_and_filled_props() {
    let mut renderer = Renderer::new(fonts());
    renderer.register(
        "swatch",
        |cx: &mut ElementContext| -> Result<(), BoxError> {
            assert_eq!(cx.rect, Rect::new(15.0, 25.0, 30.0, 20.0));
            let color: String = cx.prop("color")?.ok_or("missing color")?;
            let c = color.parse::<textplate::Color>()?;
            textplate::paint::fill_rect(cx.canvas, cx.rect, c);
            Ok(())
        },
    );
    let template = Template::from_json5(
        r##"[{ type: "box", x: 10, y: 20, padding: 5, children: [
              { type: "swatch", width: 30, height: 20, color: "{{c}}" } ] }]"##,
    )
    .unwrap();
    let mut canvas = RgbaImage::new(100, 100);
    renderer
        .render(
            &template,
            &mut canvas,
            &HashMap::from([("c", "#00ff00")]),
            &Assets::new(),
        )
        .unwrap();
    assert_eq!(canvas.get_pixel(20, 30).0, [0, 255, 0, 255]);
    assert_eq!(canvas.get_pixel(10, 30).0[3], 0);
}

#[test]
fn image_fit_modes() {
    let renderer = Renderer::new(fonts());
    let mut assets = Assets::new();
    assets.insert("red", RgbaImage::from_pixel(20, 10, Rgba([255, 0, 0, 255])));
    let draw = |fit: &str| {
        let t = Template::from_json5(&format!(
            r#"[{{ type: "image", src: "red", fit: "{fit}" }}]"#
        ))
        .unwrap();
        let mut canvas = RgbaImage::new(100, 100);
        renderer.render(&t, &mut canvas, &(), &assets).unwrap();
        canvas
    };
    let contain = draw("contain");
    assert_eq!(contain.get_pixel(50, 50).0[3], 255);
    assert_eq!(contain.get_pixel(50, 10).0[3], 0, "letterboxed above");
    let cover = draw("cover");
    assert_eq!(cover.get_pixel(50, 5).0[3], 255);
    assert_eq!(cover.get_pixel(99, 99).0[3], 255);
    let fill = draw("fill");
    assert_eq!(fill.get_pixel(0, 0).0[3], 255);
}

#[test]
fn renderer_is_shareable_across_threads() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Renderer>();

    let renderer = Renderer::new(fonts());
    let template =
        Template::from_json5(r#"[{ type: "text", text: "Row {{n}} สวัสดี", size: 24 }]"#).unwrap();
    let draw = |n: usize| {
        let mut canvas = RgbaImage::from_pixel(300, 60, Rgba([255, 255, 255, 255]));
        let n = n.to_string();
        renderer
            .render(
                &template,
                &mut canvas,
                &HashMap::from([("n", n.as_str())]),
                &Assets::new(),
            )
            .unwrap();
        canvas
    };
    let sequential: Vec<_> = (0..8).map(draw).collect();
    let parallel: Vec<_> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..8).map(|n| s.spawn(move || draw(n))).collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    assert!(sequential == parallel, "same output on every thread");
}
