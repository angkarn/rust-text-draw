# textplate

Render data-driven templates of rich, multilingual text onto images.

Describe a layout once in JSON5 — boxes, text, images — put `{{placeholders}}` in it, and render
it for every row of your data. Text shaping and font fallback come from
[cosmic-text](https://github.com/pop-os/cosmic-text) (HarfRust + skrifa + swash), so Thai, Arabic,
Devanagari, CJK and color emoji render correctly in the same line.

![output](https://raw.githubusercontent.com/angkarn/rust-text-draw/main/output.png)

`cargo run --example demo` renders the image above from [`examples/demo.json5`](examples/demo.json5).

## Install

```toml
[dependencies]
textplate = { git = "https://github.com/angkarn/rust-text-draw", tag = "v0.1.0" }
```

## Usage

```rust
use std::collections::HashMap;
use textplate::{image::RgbaImage, Assets, Fonts, Renderer, Template};

let mut fonts = Fonts::new();
fonts.add("body", std::fs::read("NotoSans-Regular.ttf")?)?;     // first font = default
fonts.add("thai", std::fs::read("NotoSansThai-Regular.ttf")?)?; // also used for fallback
let renderer = Renderer::new(fonts);

let template = Template::from_json5(r##"[
  { type: "box", fill: "#1f2937", radius: 24, padding: 32, color: "#fff", children: [
    { type: "text", text: "Hello, {{name}}!", size: "8vw", align: "center", valign: "middle", fit: "shrink" },
  ]},
]"##)?;

let mut canvas = RgbaImage::new(800, 400);
let vars = HashMap::from([("name", "สวัสดี World")]);
let report = renderer.render(&template, &mut canvas, &vars, &Assets::new())?;
for overflow in &report.overflows {
    eprintln!("warning: {overflow}");
}
canvas.save("hello.png")?;
```

- `Renderer` is `Send + Sync`. Share one across threads; each thread keeps its own font and glyph
  caches, so rendering many images in parallel stays fast.
- textplate never touches the file system. You pass font bytes, images (`Assets`) and a canvas.
- Errors are values, never panics: invalid templates, unknown fonts, missing variables or assets
  all return an [`Error`](src/error.rs) with the path of the node, e.g. `nodes[2].children[0]`.

## Templates

A template is an array of nodes (or `{ nodes: [...] }`). Nodes are drawn in order.

### Frame (every node)

| field | default | |
|---|---|---|
| `x`, `y` | `0` | offset inside the parent's content box |
| `width`, `height` | `"100%"` | size |
| `padding` | `0` | `10`, `[vertical, horizontal]` or `[top, right, bottom, left]` |

Lengths are a number (pixels) or a string: `"12px"`, `"50%"` (of the parent, same axis),
`"3vw"` / `"3vh"` (percent of the canvas width / height).

### Text style

`font` (alias from `Fonts::add`), `size` (px, vw or vh), `color` (`#rgb`, `#rgba`, `#rrggbb`,
`#rrggbbaa`), `line_height` (multiple of the size). Boxes set defaults for their descendants;
anything unset falls back to `Options` (16px, black, 1.25).

### `type: "box"`

| field | |
|---|---|
| `fill` | background color |
| `border` | `{ width: 2, color: "#000" }` |
| `radius` | corner radius |
| `children` | child nodes, laid out inside the box minus `padding` |
| text style | defaults for descendants |

### `type: "text"`

| field | default | |
|---|---|---|
| `text` | | plain text, or use `spans` |
| `spans` | | `[{ text, font?, size?, color?, line_height? }, ...]` |
| `align` | `left` | `left`, `center`, `right`, `justify` |
| `valign` | `top` | `top`, `middle`, `bottom` |
| `wrap` | `word` | `word`, `glyph`, `none` |
| `fit` | `none` | `shrink`: reduce the size until the text fits |
| `min_size` | 25% of size | smallest size for `fit: "shrink"` |

Text that does not fit is reported in `Report::overflows` (or is an error with `Options::strict`).

### `type: "image"`

`src` names an image added with `Assets::insert` / `Assets::insert_encoded`. `fit`: `contain`
(default), `cover` or `fill`.

### Custom elements

Any other `type` is drawn by an `Element` you register — for example a QR code:

```rust
renderer.register("qr", |cx: &mut textplate::ElementContext| -> Result<(), textplate::BoxError> {
    let data: String = cx.prop("data")?.ok_or("missing data")?;
    // draw into cx.canvas inside cx.rect, e.g. with textplate::paint helpers
    Ok(())
});
```

```json5
{ type: "qr", x: "70%", y: "10%", width: 200, height: 200, data: "{{url}}" }
```

### Placeholders

`{{name}}` in `text`, span `text`, image `src` and custom element fields is replaced from the
`Vars` passed to `render` (`HashMap`, `BTreeMap`, or `VarsFn(|name| ...)`). Values are inserted
as-is: quotes or `{{` inside a value never break the template.

## Features

- `decode` (default): `Assets::insert_encoded` for png, jpeg, webp, gif and bmp.

Builds for `wasm32-unknown-unknown` and `wasm32-wasip1`.

## Development

```sh
cargo test
cargo clippy --all-targets
cargo run --example demo
```

Releases: bump `version` in `Cargo.toml`, add a `## vX.Y.Z` section to [CHANGELOG.md](CHANGELOG.md),
merge, then run the **Release** workflow with that version.
