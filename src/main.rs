use image::DynamicImage;
use rust_text_draw::{cosmic_text, draw_text, fontdb, SwashCache, Widget};
use std::fs::read;

// example to used lib
fn main() {
    let fonts_path = [
        "fonts/poppins-v21-latin-regular.ttf",
        "fonts/noto-sans-thai-v25-thai-regular.ttf",
        "fonts/noto-sans-arabic-v28-arabic-regular.ttf",
        "fonts/noto-sans-jp-v53-japanese-regular.ttf",
        "fonts/noto-sans-kr-v36-korean-regular.ttf",
        "fonts/noto-sans-sc-v37-chinese-simplified-regular.ttf",
        "fonts/noto-sans-devanagari-v26-devanagari-regular.ttf",
        "fonts/NotoColorEmoji.ttf",
        "fonts/Monofett-Regular.ttf",
        "fonts/LibreBarcode39Text-Regular.ttf",
    ];

    let json_txt = "[{fill:'e9e0d4'},{y:3,a:1,c:'000000',ts:[{fs:6,t:'🌈'},{f:8,fs:8,c:'C63658',t:'Hello '},{f:9,fs:8,t:'*world*'}]},{a:1,y:18,fs:4.5,ts:[{t:'🙋 やあ 안녕 你好 สวัสดี नमस्ते مَرحَباً'}]},{y:28,h:0.3,ml:30,mr:30,fill:'b1b2b3'},{y:30,c:'ff999a',ts:[{t:'Lorem Ipsum is simply dummy text of the printing and typesetting industry.'},{c:'0c87a5',t:' Lorem Ipsum has been the industry\\'s standard dummy text ever since the 1500s'}]},{y:40,a:2,ts:[{fs:2,t:'Lorem Ipsum is simply dummy text of the printing and typesetting industry. Lorem Ipsum has been the industry\\'s standard dummy text ever since the 1500s'}]},{y:52,ml:3,mr:3,p:1,w:50,h:40,ts:[{c:'000000',t:'Lorem Ipsum is simply dummy text of the printing and typesetting industry. Lorem Ipsum has been the industry\\'s standard dummy text ever since the 1500s'}]},{x:50,y:52,w:50,h:45,ml:3,mr:3,fill:'8d8282',wi:[{ml:3,mr:3,mb:3,mt:3,fill:'36454F',wi:[{p:2,ml:1,mr:1,ts:[{c:'ffffff',t:'Lorem Ipsum is simply dummy text of the printing and typesetting industry. Lorem Ipsum has been the industry\\'s standard dummy text ever since the 1500s'}]}]}]},{ts:[{c:'e74c3c',t:'a:0'},{c:'2ecc71',t:' p:0'}]},{a:1,ts:[{c:'5dade2',t:'a:1'},{c:'2ecc71',t:' p:0'}]},{a:2,ts:[{c:'f39c12',t:'a:2'},{c:'2ecc71',t:' p:0'}]},{p:1,ts:[{c:'e74c3c',t:'a:0'},{c:'71569b',t:' p:1'}]},{a:1,p:1,ts:[{c:'5dade2',t:'a:1'},{c:'71569b',t:' p:1'}]},{a:2,p:1,ts:[{c:'f39c12',t:'a:2'},{c:'71569b',t:' p:1'}]},{p:2,ts:[{c:'e74c3c',t:'a:0'},{c:'f17ba3',t:' p:2'}]},{a:1,p:2,ts:[{c:'5dade2',t:'a:1'},{c:'f17ba3',t:' p:2'}]},{a:2,p:2,ts:[{c:'f39c12',t:'a:2'},{c:'f17ba3',t:' p:2'}]}]";

    let font_db = fontdb::Database::new();
    let mut font_system =
        cosmic_text::FontSystem::new_with_locale_and_db("en-US".to_string(), font_db);

    for path in fonts_path {
        let font_data = read(path).expect(&format!("Error read font file: \"{}\"", path));
        font_system.db_mut().load_font_data(font_data);
    }

    // A SwashCache stores rasterized glyphs, create one per application
    let mut swash_cache = SwashCache::new();

    // Create an image buffer
    let image_width = 1000.0;
    let image_height = 1000.0;
    let mut image = DynamicImage::new_rgba8(image_width as u32, image_height as u32);

    let font_size_adjust = 2.4;

    let widgets = json5::from_str::<Vec<Widget>>(&json_txt).unwrap();

    let _ = draw_text(
        &mut swash_cache,
        &mut font_system,
        &mut image,
        0.0,
        0.0,
        image_width,
        image_height,
        widgets,
        font_size_adjust,
        &"000000".to_string(),
        true,
    );

    let _ = image.save("output.png");
    println!("Success");
}
