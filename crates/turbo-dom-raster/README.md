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
turbo-dom-raster = "0.8"                                        # MSRV 1.89
turbo-dom-raster = { version = "0.8", features = ["avif", "jxl"] }
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
  - `PreparedScreenshot::new(html, external_css, viewport, images, &options)` owns an
    immutable prepared layout and decoded image pixels. Repeated `.png()` / `.svg()`
    calls skip parse, image decode and layout. Prepare a replacement whenever HTML,
    CSS, viewport, assets, fonts, canvas contents or IDL state changes; old scenes
    retain their original output independently and release resources on drop. This
    explicit snapshot API has no shared mutable cache or revision-key contract.
  - `screenshot(html, viewport, Format::{Png, Svg})`, `screenshot_png`, `screenshot_svg`;
  - `*_with_css` adds external CSS;
  - `*_with_assets` adds caller-fetched images (`ImageAssets` = URL → bytes) and
    `full_page`;
  - `*_with_opts` adds `system_fonts`, which resolves against the installed fonts through
    turbo-dom-layout's `FontRegistry::system`;
  - `screenshot_png_with_options(html, external_css, viewport, images, &options) -> Result<Vec<u8>, String>`
    and `screenshot_svg_with_options(...) -> Result<String, String>` take a
    `ScreenshotOptions<'a>` (`#[non_exhaustive]`; start from
    `ScreenshotOptions::default()` and set the fields you need):

    | field | type | effect |
    |---|---|---|
    | `device_scale` | `f64` | output pixels per CSS pixel (default 1), without changing layout; dimensions round to the nearest device pixel, with positive half-pixel ties up (Chrome 156); finite positive values representable by the native painter only |
    | `omit_background` | `bool` | leave the initial canvas transparent; authored root/body backgrounds still paint |
    | `masks` | `&[ScreenshotMask]` | CSS-pixel overlays painted last; `ScreenshotMask::new(rect)` uses opaque magenta, or set its RGBA color |
    | `full_page` | `bool` | paint the full content height instead of the viewport |
    | `system_fonts` | `bool` | resolve against installed system fonts (`FontRegistry::system`) |
    | `canvases` | `Option<&'a CanvasContents>` | the embedder's canvas pixels (below) |
    | `fonts` | `Option<&'a FontRegistry>` | lay out and paint with the caller's registry instead of the bundled / system set, so a caller that loads the page's `@font-face` sources (`FontFace::from_web_font_bytes`; `FontFace::is_web_font`) gets web fonts at Chrome's metrics (turbo-dom#304). The raster does not read `@font-face` rules itself |
    | `idl_states` | `Option<&'a IdlStates>` | live IDL-state flags (below) |

  - **Canvas content** (`canvases`, turbo-dom#291). `CanvasContents = HashMap<usize,
    CanvasContent>` keys each canvas by its index among the document's `<canvas>`
    elements in tree order (`document.querySelectorAll('canvas')` order). A
    `CanvasContent` is either `Ops(String)` (the render tier's 2D-canvas op log —
    `ctx._ops` JSON, replayed at the canvas's `width` x `height`) or
    `Rgba { width, height, pixels: Vec<u8> }` (straight-alpha RGBA8, row-major from
    the top-left, `width * height * 4` bytes, as `getImageData` reads). Painted in
    each canvas's content box like Chrome 156: scaled by `object-fit`, placed by
    `object-position`, inside border and padding, clipped to the content box and its
    rounded corners, under the box's transform, upright in vertical writing modes.
    A canvas without an entry stays blank (background and border only).
  - **Live IDL state** (`idl_states`, turbo-dom#293 / #371).
    `IdlStates { indeterminate_inputs: HashSet<usize> }` carries the IDL flags the
    serialized HTML can't express: tree-order indices among
    `document.querySelectorAll('input')` of `<input>` elements whose IDL
    `indeterminate` flag a script set to `true`. A checkbox at one of those indices
    paints the Chrome indeterminate dash (equivalent to
    `turbo_dom_layout::INDETERMINATE_ATTR` on its styled box), checked or not; any
    other control ignores the flag, like Chrome.
- **Resource discovery** (works on the parsed document, never on source text):
  `image_urls`, `image_urls_in_css`, `stylesheet_hrefs`, and `delazy_tree` /
  `delazy_images` (fill lazy `<img>`s).
- **Capture resource planning:** `capture_resources(html, css, viewport, &metadata, &options)`
  returns sorted `pixel_urls` and deterministic `fragments_examined`. `ImageMetadata`
  maps resolver keys to `turbo_dom_layout::image::Intrinsic` headers; retain these
  independently of image bodies and set `options.image_metadata = Some(&metadata)`
  when painting only selected bodies. Layout uses the same inputs and viewport as
  screenshots. Unknown headers are always selected because their intrinsic sizes
  can change layout. Positioned descendants are visited independently of ancestor
  bounds; transforms, shadows and filters conservatively keep resources. Selection
  ignores clipping and occlusion, so the result may contain extra URLs. No image
  bytes or references are retained by the returned plan. Recompute after changing
  markup, CSS, viewport, metadata, fonts or live IDL inputs.
  Run the scalable offline correctness/timing harness with
  `cargo run -p turbo-dom-raster --example capture_resources -- 1000`.
- **Canvas:** `canvas_ops_png` / `canvas_ops_rgba` replay a 2D-canvas op log. Colours go
  through turbo-dom's `rtdom::color::parse_color`. `measure_text` measures a text run (width, height) with the bundled or system faces;
  `measure_canvas_text(text, font)` is canvas `measureText`'s width for a CSS `font`
  shorthand. Canvas text uses the bundled faces; `canvas_ops_png_with_fonts(width,
  height, ops_json, fonts)`, `canvas_ops_rgba_with_fonts(width, height, ops_json, fonts)`
  and `measure_canvas_text_with_fonts(text, font, fonts)` take a caller `&FontRegistry`
  (web fonts; the `coretext` path is skipped for an `@font-face` face).
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

Painting follows Chrome 156 for: per-corner elliptical `border-radius` on backgrounds,
gradients, borders (each side in its own width and colour), outer and inset box shadows,
and `overflow` clipping (turbo-dom#253, #192); background layers with size / position /
repeat (incl. `space` / `round`) / origin / clip, tiled from one decoded image (#248);
inline-box decorations per line fragment with `box-decoration-break` (#244; a cloned
box's radius, background image and inset shadow repeat on every fragment); vertical and
upright text (#234, #239); bidi-reordered runs (#238); list markers (#128). Broken or
unloaded `<img>`s paint their box without pixels (#243).

Background layers are one repeating-pattern fragment each (`FragmentContent::Tiles`),
painted as a pattern shader (PNG) or an SVG `<pattern>` that repeats one whole-pixel cell,
so any number of tiles fill the box; fractional tiles blend their edge pixels like Chrome
(#270), and a scaled or rotated box's tiles and images are drawn at device resolution
(#269). Gradients take length, double-position and repeating stops (#264). A `transform`
on an `<img>` itself applies (#272). Form controls paint Chrome 156's native appearance
from turbo-dom-layout's galley boxes (#255, #257). Glyphs are rasterized with exact-area
coverage, CoreText-like box-kernel outline growth and a luminance-dependent edge transfer
(#266, #331); the SVG screenshot's glyph paths take Skia's mask pre-blend through one
`feComponentTransfer` filter per text luminance class (#294). Chrome's grid-dependent
vertical geometry below about 20px is not modelled, so 12–16px text differs most.

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
- MSRV 1.89. Cyclomatic complexity ≤ 9 is **blocking** in 0.8.3.

## Testing

```bash
cargo test -p turbo-dom-raster                    # unit tests, tests/screenshot.rs, Chrome 156 controls
cargo run --release -p turbo-dom-raster --example retained_capture -- 5000 10
cargo test -p turbo-dom-raster --features avif
cargo test -p turbo-dom-raster --features jxl
cargo check -p turbo-dom-raster --features gpu-metal,coretext
cargo tarpaulin --engine llvm -p turbo-dom-raster [--features avif|jxl] --include-files 'crates/turbo-dom-raster/src/*'
```

The offline `retained_capture` harness accepts row count and repeat count, checks
pixel/byte equivalence and snapshot isolation, and prints timings plus layout-pass
counts. Timings are diagnostic; normal tests gate deterministic work and output.

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
