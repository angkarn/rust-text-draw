//! Renders `output.png` from `examples/demo.json5`.
//!
//! cargo run --example demo

use std::collections::HashMap;

use textplate::{image::RgbaImage, Assets, Fonts, Renderer, Template};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut fonts = Fonts::new();
    for (alias, file) in [
        ("body", "poppins-v21-latin-regular.ttf"),
        ("thai", "noto-sans-thai-v25-thai-regular.ttf"),
        ("arabic", "noto-sans-arabic-v28-arabic-regular.ttf"),
        ("jp", "noto-sans-jp-v53-japanese-regular.ttf"),
        ("kr", "noto-sans-kr-v36-korean-regular.ttf"),
        ("sc", "noto-sans-sc-v37-chinese-simplified-regular.ttf"),
        ("hindi", "noto-sans-devanagari-v26-devanagari-regular.ttf"),
        ("emoji", "NotoColorEmoji.ttf"),
        ("display", "Monofett-Regular.ttf"),
        ("barcode", "LibreBarcode39Text-Regular.ttf"),
    ] {
        fonts.add(alias, std::fs::read(format!("fonts/{file}"))?)?;
    }
    let renderer = Renderer::new(fonts);

    let template = Template::from_json5(&std::fs::read_to_string("examples/demo.json5")?)?;
    let vars = HashMap::from([
        ("greeting", "🙋 やあ 안녕 你好 สวัสดี नमस्ते مَرحَباً"),
        ("name", "textplate"),
        ("code", "*TEXTPLATE*"),
    ]);

    let mut canvas = RgbaImage::new(1000, 1000);
    let report = renderer.render(&template, &mut canvas, &vars, &Assets::new())?;
    for overflow in &report.overflows {
        eprintln!("warning: {overflow}");
    }
    canvas.save("output.png")?;
    println!("Wrote output.png");
    Ok(())
}
