use image::DynamicImage;
use rust_text_draw::{cosmic_text, draw_text, fontdb, SwashCache, Widget};
use std::fs::read;

// example to used lib
fn main() {
    let fonts_path = [
        "fonts/noto-sans-thai-v25-thai-regular.ttf",
        "fonts/Monofett-Regular.ttf",
        "fonts/LibreBarcode39Text-Regular.ttf",
        "fonts/noto-sans-arabic-v28-arabic-regular.ttf",
        "fonts/noto-sans-jp-v53-japanese-regular.ttf",
        "fonts/noto-sans-kr-v36-korean-regular.ttf",
        "fonts/noto-sans-sc-v37-chinese-simplified-regular.ttf",
        "fonts/noto-sans-devanagari-v26-devanagari-regular.ttf",
        "fonts/NotoColorEmoji.ttf",
    ];

    let json_txt = "[{fill:'e9e0d4'},{y:30,a:1,c:'000000',ts:[{fs:60,t:'🌈'},{f:1,fs:80,c:'C63658',t:'Hello '},{f:2,fs:80,t:'*world*'}]},{y:280,h:2,ml:300,mr:300,fill:'b1b2b3'},{y:300,c:'ff999a',ts:[{t:'Lorem Ipsum is simply dummy text of the printing and typesetting industry.'},{c:'0c87a5',t:' Lorem Ipsum has been the industry\\'s standard dummy text ever since the 1500s'}]},{y:400,a:2,ts:[{fs:20,t:'Lorem Ipsum is simply dummy text of the printing and typesetting industry. Lorem Ipsum has been the industry\\'s standard dummy text ever since the 1500s'}]},{a:1,y:180,fs:40,ts:[{t:'🙋 やあ 안녕 你好 สวัสดี नमस्ते مَرحَباً'}]},{y:580,ml:20,mr:10,p:1,w:500,h:400,ts:[{c:'000000',t:'Lorem Ipsum is simply dummy text of the printing and typesetting industry. Lorem Ipsum has been the industry\\'s standard dummy text ever since the 1500s'}]},{x:500,y:580,w:500,h:400,ml:10,mr:20,fill:'8d8282',wi:[{ml:10,mr:10,mb:10,mt:10,fill:'36454F',wi:[{p:2,ml:10,mr:10,ts:[{c:'ffffff',t:'Lorem Ipsum is simply dummy text of the printing and typesetting industry. Lorem Ipsum has been the industry\\'s standard dummy text ever since the 1500s'}]}]}]},{ts:[{c:'e74c3c',t:'a:0'},{c:'2ecc71',t:' p:0'}]},{a:1,ts:[{c:'5dade2',t:'a:1'},{c:'2ecc71',t:' p:0'}]},{a:2,ts:[{c:'f39c12',t:'a:2'},{c:'2ecc71',t:' p:0'}]},{p:1,ts:[{c:'e74c3c',t:'a:0'},{c:'71569b',t:' p:1'}]},{a:1,p:1,ts:[{c:'5dade2',t:'a:1'},{c:'71569b',t:' p:1'}]},{a:2,p:1,ts:[{c:'f39c12',t:'a:2'},{c:'71569b',t:' p:1'}]},{p:2,ts:[{c:'e74c3c',t:'a:0'},{c:'f17ba3',t:' p:2'}]},{a:1,p:2,ts:[{c:'5dade2',t:'a:1'},{c:'f17ba3',t:' p:2'}]},{a:2,p:2,ts:[{c:'f39c12',t:'a:2'},{c:'f17ba3',t:' p:2'}]}]";

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
    let image_width = 1000;
    let image_height = 1000;
    let mut image = DynamicImage::new_rgba8(image_width as u32, image_height as u32);

    let font_size_adjust = 25.0;

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
