# turbo-dom-raster

Synthetic screenshots with no browser: turn an HTML **snapshot** into a PNG or SVG.
[`turbo-dom-layout`](../turbo-dom-layout) lays the HTML out into a positioned
`Fragment` display list; this crate paints it two ways — a raster painter
(tiny-skia) and a vector painter (SVG) — walking the same fragments in CSS paint
order, so the two agree. Caller-supplied `<img>` / `background-image` bytes
(PNG / JPEG / GIF / WebP / BMP / ICO / CUR / SVG, AVIF under the `avif` feature and JPEG XL under
the `jxl` feature) are
decoded and painted; it does no I/O itself.

It also replays a recorded 2D-canvas op log (`canvas_ops_png` / `canvas_ops_rgba`; style
colours through turbo-dom's CSS colour parser `rtdom::color::parse_color`, so `hsl()`,
percentage `rgb()` and every named colour resolve as in Chrome; `currentcolor` needs the canvas element, so the recorder resolves it: turbo-dom-browser-env's
context records the resolved colour, turbo-dom#108)
and, under `gpu-metal`, a recorded WebGL call batch (`webgl_readback`).

```rust
use turbo_dom_raster::{screenshot_png, Viewport};

fn shot(html: &str) -> Result<Vec<u8>, String> {
    screenshot_png(html, Viewport::default()) // 1280x800
}
```

## Features

| feature | default | what it does |
|---|---|---|
| `gpu-metal` | off | rasterize canvas op logs and WebGL batches on the GPU (wgpu → Metal); any GPU failure falls back to the tiny-skia path |
| `coretext` | off | macOS only: canvas 2D text through CoreText/CoreGraphics (inert elsewhere) |
| `avif` | off | AVIF screenshot pixels through `rav1d` (BSD-2-Clause pure-Rust dav1d port, assembly off, two frame contexts — with one, rav1d 1.1.0 aborts the process on a corrupt AVIF, turbo-dom#94; `src/avif.rs` records the options assessed). A corrupt AVIF decodes to no pixels (laid out at its `ispe` size, painting nothing, as Chrome; a truncated one is broken, turbo-dom#107), never an abort. Without it an AVIF lays out at its size and paints as the placeholder |
| `jxl` | off | JPEG XL screenshot pixels through `jxl-oxide` (MIT OR Apache-2.0, pure Rust, no thread pool; `src/jxl.rs` records the options assessed). Chrome 156 decodes JPEG XL (miaskiewicz/turbo-dom#45). Without it a JPEG XL lays out at its size (turbo-dom-layout's header probe) and paints as the placeholder |

turbo-dom-layout is used with explicit features: `bundled-fonts` and `svg-rasterize` (the
shared SVG rasterizer, at 1x here); its `svg` feature (the PDF SVG pipeline) is never
enabled from here.

## Images

Fetched image bytes decode once, through turbo-dom-layout's shared image layer where it is
byte-identical: PNG via `image::decode_png_rgba` (the PDF emitter's decode, RGBA equal to
`image::to_rgba8`), SVG via `svg::rasterize` at 1x (an axis above 8192 px rejected). JPEG
(zune-jpeg), GIF (frame 0 composited), WebP, BMP and ICO/CUR (the entry a browser shows,
`image::ico_entry`: a PNG entry through the shared PNG decode) decode with the `image`
crate; AVIF demuxes through `image::avif_payload` and decodes with `rav1d` (`avif`), JPEG XL
decodes with `jxl-oxide` (`jxl`; its oriented output is turned back to the stored pixels, the
painters apply the probe's orientation). The
`image` crate also
does the `Triangle` resize for paint and the PNG re-encode of SVG `data:` URIs. Layout
sizes each image from the shared header probe through `ImageResolver::intrinsic` (no PNG
re-encode for sizing). A PNG, JPEG, GIF or WebP loads exactly when that probe sizes it —
Blink's size point (miaskiewicz/turbo-dom#59): a cut-short body before it is not an image, one
past it whose pixels do not decode lays out at its size and paints nothing, as in Chrome — so
does a frame-corrupt AVIF; an AVIF loads once its primary item's `iloc` extents lie within the
data (miaskiewicz/turbo-dom#107). The grey placeholder is for an image the caller did not
supply or this build has no decoder for (`image_paint::truncation_tests`, against
turbo-dom-layout's `tests/fixtures/image_truncation_chrome156.json`; the `undecodable-*`
cases of `tests/fixtures/raster_img_chrome156.json`). `image_paint::shared_layer_tests` asserts the old decode
(`image::load_from_memory(..).to_rgba8()`, the old 1x resvg path) and the old
probe-of-the-re-encode equal the new ones over turbo-dom's `tools/parity` image inputs and
every 8/16-bit sample value. `system_fonts` renders share turbo-dom-layout's process-wide
`FontRegistry::system` registry.

Inline `<svg>`: the layout has no inline-SVG painter, so the screenshot parses the snapshot
into a turbo-dom tree, replaces each outermost SVG-namespace `<svg>` element with an `<img>`
(its `class`/`style`/`width`/`height`) and rasterizes the element re-serialized as
namespace-well-formed XML (miaskiewicz/turbo-dom#25). Only parsed elements convert: a `<svg`
inside an attribute value (a raw-UTF-8 SVG `data:` URL), a `<script>` string or `<noscript>`
text stays as it is. `tests/inline_svg_chrome.rs` checks the pixels against Chrome 156
(`scripts/inline-svg-chrome-control.mjs`).

`<img>` elements (miaskiewicz/turbo-dom#27): an `<img src="data:…">` paints from its own
URL. The screenshot decodes it through turbo-dom-layout's `image::decode_data_url` (base64
or percent-encoded), so no caller bytes are needed; `image_urls` still leaves `data:` URLs
off the fetch list. `image_urls` and the lazy-image fill work on the parsed document's
HTML `<img>` elements, never on source text. `<img` text in an attribute value, a
`<script>` string, a comment or `<template>` content is not an image. `delazy_tree` fills
the parsed tree in place, which is what the screenshot does. `delazy_images` keeps its
string API: it returns the HTML unchanged when no image changes, else the filled document
re-serialized. `tests/raster_img_chrome.rs` checks this against Chrome 156
(`scripts/raster-img-chrome-control.mjs`).

## Provenance

Relocated **verbatim** from `turbo-surf-raster` 0.5.5 (miaskiewicz/turbo-surf @
b1de4eb, including `webgl.rs`) in the turbo-dom consolidation (step S1); the only
code change is the dependency `turbo-html2pdf-core` -> `turbo-dom-layout` (same API,
same names). `scripts/verify-copied-layout.sh` checks the copy.

```bash
cargo test -p turbo-dom-raster        # unit tests, tests/screenshot.rs + the Chrome 156 controls
cargo check -p turbo-dom-raster --features gpu-metal,coretext
cargo test -p turbo-dom-raster --features avif  # AVIF pixels vs the Chrome 156 control (CI raster job)
cargo test -p turbo-dom-raster --features jxl   # JPEG XL pixels vs the Chrome 156 control (CI raster job)
```

Lints: `clippy::pedantic` via `[workspace.lints]` (see turbo-dom-layout's README).

## Coverage

100% line coverage (tarpaulin, LLVM engine) in each configuration CI tests — default,
`--features avif`, `--features jxl` — gated at 100 in turbo-dom's `coverage` job:

```bash
cargo tarpaulin --engine llvm -p turbo-dom-raster [--features avif|jxl] --include-files 'crates/turbo-dom-raster/src/*'
```

Expected pixels come from Chrome 156 controls where Chrome can observe them:
`tests/paint_chrome.rs` (box decorations, clips, translucency; the SVG screenshot is
rasterized with resvg against the same probes), `tests/canvas_ops_chrome.rs` (the 2D-canvas
op log: colour syntaxes, transforms, paths, arcs, strokes, `clearRect`, `globalAlpha`;
`scripts/canvas-ops-chrome-control.mjs`) and `tests/avif_decode_chrome.rs` (every chroma
layout / matrix family / range and a file without `nclx`; fixtures from
`scripts/make_raster_avif_fixtures.py`, control `scripts/raster-avif-chrome-control.mjs`).
Branches that could not run were restructured or removed as provably dead: a `FontFace`'s
bytes always re-parse (`glyph::outline_face`), the bundled faces always resolve
(`measure_text`, canvas text), a clip layer or shadow scratch within its bounds always
allocates, a non-empty `Pixmap` never needs the blur's empty guards, and the AVIF / JPEG XL
size-only fallback is one branch shared by both features.

MSRV 1.89. License: MIT.
