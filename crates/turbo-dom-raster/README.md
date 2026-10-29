# turbo-dom-raster

Synthetic screenshots without a browser: an HTML snapshot becomes a PNG or an SVG.
[turbo-dom-layout](../turbo-dom-layout) lays the page out into positioned `Fragment`s.
This crate paints them in CSS paint order with two painters that agree: a raster painter
(tiny-skia) and a vector painter (SVG). It also replays recorded 2D-canvas op logs and
WebGL call batches. It does no I/O: the caller supplies image bytes.

## Where it sits

```
turbo-dom ──► turbo-dom-layout ──► turbo-dom-raster ──► turbo-surf (static screenshots, canvas, WebGL)
```

It depends on turbo-dom-layout with explicit features: `bundled-fonts` and
`svg-rasterize` (1x). It never enables the PDF `svg` pipeline. Origin: turbo-surf-raster
0.5.5 (turbo-dom 0.6.0 consolidation, [docs/CONSOLIDATION.md](../../docs/CONSOLIDATION.md)).

## Install

```toml
turbo-dom-raster = "0.6"                                        # MSRV 1.89
turbo-dom-raster = { version = "0.6", features = ["avif", "jxl"] }
```

## Features

| feature | default | what it does |
|---|---|---|
| `gpu-metal` | off | rasterize canvas op logs and WebGL batches on the GPU (wgpu → Metal); any GPU failure falls back to the tiny-skia path |
| `coretext` | off | macOS only: canvas 2D text through CoreText/CoreGraphics (inert elsewhere) |
| `avif` | off | AVIF screenshot pixels through `rav1d` (BSD-2-Clause pure-Rust dav1d port, assembly off, two frame contexts — with one, rav1d 1.1.0 aborts the process on a corrupt AVIF, turbo-dom#94; `src/avif.rs` records the options assessed). A corrupt AVIF decodes to no pixels (laid out at its `ispe` size, painting nothing, as Chrome; a truncated one is broken, turbo-dom#107), never an abort. Without it an AVIF lays out at its size and paints as the placeholder |
| `jxl` | off | JPEG XL screenshot pixels through `jxl-oxide` (MIT OR Apache-2.0, pure Rust, no thread pool; `src/jxl.rs` records the options assessed). Chrome 156 decodes JPEG XL (miaskiewicz/turbo-dom#45). Without it a JPEG XL lays out at its size (turbo-dom-layout's header probe) and paints as the placeholder |

## API

```rust
use turbo_dom_raster::{screenshot_png, Viewport};

fn shot(html: &str) -> Result<Vec<u8>, String> {
    screenshot_png(html, Viewport::default()) // 1280x800
}
```

- **Screenshots:**
  - `screenshot(html, viewport, Format::{Png, Svg})`, `screenshot_png`, `screenshot_svg`;
  - `*_with_css` adds external CSS;
  - `*_with_assets` adds caller-fetched images (`ImageAssets` = URL → bytes) and
    `full_page`;
  - `*_with_opts` adds `system_fonts`, which resolves against the installed fonts through
    turbo-dom-layout's `FontRegistry::system`.
- **Resource discovery** (works on the parsed document, never on source text):
  `image_urls`, `image_urls_in_css`, `stylesheet_hrefs`, and `delazy_tree` /
  `delazy_images` (fill lazy `<img>`s).
- **Canvas:** `canvas_ops_png` / `canvas_ops_rgba` replay a 2D-canvas op log. Colours go
  through turbo-dom's `rtdom::color::parse_color`. `measure_text` measures a text run (width, height) with the bundled or system faces.
- **WebGL:** `webgl_readback` and `webgl_metadata` work on a recorded call batch, backed
  by the GPU under `gpu-metal`.

Images decode once. PNG and SVG go through turbo-dom-layout's shared image layer
(`image::decode_png_rgba`, `svg::rasterize` at 1x). JPEG, GIF, WebP, BMP and ICO/CUR use the
`image` crate. AVIF (`avif`) and JPEG XL (`jxl`) have their own decoders.

Images follow Blink's load decision:
- A body cut short before its size point is broken.
- An image whose pixels fail to decode is laid out at its size and paints nothing.
- The grey placeholder is only for an image that was not supplied, or that this build
  cannot decode.

Inline `<svg>` elements and `<img src="data:…">` paint as in Chrome (turbo-dom#25, #27).

## Gates

- Line coverage **100%** in each CI configuration (default, `--features avif`,
  `--features jxl`); each is a blocking gate in turbo-dom's `coverage` job.
  Branches that could not run were restructured or removed as provably dead: a `FontFace`'s
bytes always re-parse (`glyph::outline_face`), the bundled faces always resolve
(`measure_text`, canvas text), a clip layer or shadow scratch within its bounds always
allocates, a non-empty `Pixmap` never needs the blur's empty guards, and the AVIF / JPEG XL
size-only fallback is one branch shared by both features.
- CI `raster` job: clippy pedantic `-D warnings` and tests (default, `avif`, `jxl`), a
  `gpu-metal` check on Linux, and `coretext` / `gpu-metal` on macOS in the release run.
  Pull requests run on Linux only.
- MSRV 1.89. Cyclomatic complexity ≤ 15 is **report-only** until the post-release refactor.

## Testing

```bash
cargo test -p turbo-dom-raster                    # unit tests, tests/screenshot.rs, Chrome 156 controls
cargo test -p turbo-dom-raster --features avif
cargo test -p turbo-dom-raster --features jxl
cargo check -p turbo-dom-raster --features gpu-metal,coretext
cargo tarpaulin --engine llvm -p turbo-dom-raster [--features avif|jxl] --include-files 'crates/turbo-dom-raster/src/*'
```

Pixel expectations come from **Chrome 156 controls**, recorded by the repository's
`scripts/*-chrome-control.mjs`. Each test pairs with one script:

| test | control | what it checks |
|---|---|---|
| `tests/paint_chrome.rs` | `paint-chrome-control.mjs` | box decorations, clips, translucency (SVG output rasterized against the same probes) |
| `tests/canvas_ops_chrome.rs` | `canvas-ops-chrome-control.mjs` | canvas op logs |
| `tests/avif_decode_chrome.rs` | `raster-avif-chrome-control.mjs` | AVIF decode |
| `tests/image_formats_chrome.rs` | `image-formats-chrome-control.mjs` | image formats |
| `tests/image_orientation_chrome.rs` | `image-orientation-chrome-control.mjs` | EXIF orientation |
| `tests/inline_svg_chrome.rs` | `inline-svg-chrome-control.mjs` | inline `<svg>` |
| `tests/raster_img_chrome.rs` | `raster-img-chrome-control.mjs` | `<img>` elements |
| `tests/html_document_chrome.rs` | `html-document-chrome-control.mjs` | document styling |

`image_paint::shared_layer_tests` gates the shared decode byte for byte against the old
decoders over `tools/parity` inputs. Text is painted with turbo-dom-layout's bundled faces
unless `system_fonts` is set, so the tests do not depend on host fonts.

## Links

[turbo-dom-layout](../turbo-dom-layout/README.md) · [turbo-dom](../turbo-dom/README.md) ·
[CHANGELOG](../../CHANGELOG.md) · `scripts/verify-copied-layout.sh` (the one-time move)

License: MIT.
