# Changelog

## v0.1.0

First release as **textplate**, a rewrite of rust-text-draw with a new API and template format.
Older templates and the old `draw_text` function are not compatible.

### Template format
- Nodes have a `type`: `box`, `text`, `image`, or any custom element type.
- Readable field names (`align`, `valign`, `size`, `padding`, `children`, ...).
- Lengths in px, `%` of the parent, `vw` / `vh` of the canvas.
- Colors as `#rgb`, `#rgba`, `#rrggbb` or `#rrggbbaa`, so text and fills can be transparent.
- `{{placeholders}}` are parsed once and filled at render time. Values cannot break the template.
- Unknown fields and invalid values are errors that include the node path.

### Rendering
- Boxes: fill, border and anti-aliased rounded corners.
- Text: spans with their own font/size/color/line height, `justify`, `wrap: none | word | glyph`,
  `fit: "shrink"` with `min_size`. Overflows are reported in `Report` (or are errors with `strict`).
- Images from `Assets`, with `contain`, `cover` and `fill`.
- Custom elements through the `Element` trait (e.g. QR codes from qrgen).
- Correct straight-alpha source-over blending.

### API
- `Fonts` with aliases (the first font is the default; all fonts take part in fallback).
- `Renderer` is `Send + Sync` and keeps per-thread font and glyph caches.
- No file system access and no panics on bad input: everything returns `textplate::Error`.
- Builds for `wasm32-unknown-unknown` and `wasm32-wasip1`.
