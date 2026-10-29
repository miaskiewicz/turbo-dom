# turbo-dom-layout

The native **HTML/CSS layout engine** of turbo-dom. It covers:

- a style cascade over a turbo-dom `Tree`;
- box generation;
- block, inline, flex (taffy), grid and table layout into one galley of positioned
  `Fragment`s;
- fonts with shaping (rustybuzz) and per-character fallback;
- one shared image layer: sizing, PNG decode and SVG rasterization;
- CSSOM-View geometry over live snapshots.

It does no I/O and has no PDF or painting code.

## Where it sits

```
turbo-dom ──► turbo-dom-layout ──► turbo-dom-raster ──► turbo-surf (screenshots)
                    ├──► turbo-html2pdf (templating, pagination, PDF emit)
                    └──► turbo-surf (live CSSOM-View: getBoundingClientRect, elementFromPoint, …)
```

Selectors are parsed and matched by turbo-dom's engine (`parse_for_cascade` /
`match_for_cascade` through `style::Navigator`), so layout matches exactly what the DOM APIs
match. Origin: turbo-html2pdf-core 0.3.9's layout half plus turbo-surf's CSSOM-View modules
(turbo-dom 0.6.0 consolidation; [docs/CONSOLIDATION.md](../../docs/CONSOLIDATION.md),
`scripts/verify-copied-layout.sh` for the one-time move).

## Install

```toml
turbo-dom-layout = "0.6"                                            # bundled fonts on
turbo-dom-layout = { version = "0.6", default-features = false }   # caller fonts only
```

MSRV 1.89.

## Features

| feature | default | what it does |
|---|---|---|
| `bundled-fonts` | on | embeds Inter / Roboto, Liberation Serif / PT Serif, Fira Code / IBM Plex Mono (OFL, see `assets/fonts/NOTICE.md`) so documents lay out with zero caller fonts |
| `svg` | off | the PDF SVG pipeline: `image::probe` / `image::decode` route `image/svg+xml` through the shared rasterizer at 2x (implies `svg-rasterize`) |
| `svg-rasterize` | off | the shared SVG rasterizer alone (`svg::rasterize`, resvg `=0.45.1`, pinned: 0.46+ changes raster output) without routing `probe`/`decode` through it — what turbo-dom-raster enables |
| `perf` | off | hot-path counters (`perf` module, `hot!` macro) |
| `serde` | off | `serde::Serialize` for the `cssom_view` metrics records (turbo-surf returns them to page script) |
| `test-support` | off | `test_support`: font fixtures read from `assets/fonts` (`FIXTURE_FONTS_DIR`, `font_bytes`, `registry`, …), `style` pairs, test-only tree literals that build a `Tree` (`literal`), `FontFace::collection_fixture_faces`; for this crate's and downstream tests (the integration tests enable it via a self dev-dependency), never in production |

## API

```rust
use turbo_dom_layout::{layout_html, Diagnostics, FontRegistry, Fragment, RenderError};

fn lay_out(html: &str) -> Result<Fragment, RenderError> {
    let fonts = FontRegistry::new(); // bundled OFL faces (default `bundled-fonts`)
    let mut diags = Diagnostics::default();
    layout_html(html, "", 600.0, &fonts, &mut diags) // extra CSS, 600 px content width
}
```

- **HTML entries** (`html_layout`): `layout_html`, `layout_html_with_images`,
  `layout_html_tree`, `layout_html_tree_in_viewport`, `layout_boxes` / `layout_boxes_tree`
  (border boxes like `getBoundingClientRect()`), `parse_html_tree`, `style_html_tree`,
  `collect_style_css_tree`, `HtmlOptions`. They style the whole document as Chrome does:
  head `<style>`s, real `<html>`/`<body>` boxes, `body { margin: 8px }`, root-relative
  `rem`, and the canvas background.
- **Cascade** (`style`): `TreeView` (html2pdf mode, which can carry opaque `MarkerKind`
  markers from a `MarkerClassifier`), `Roots::{Detached, Document}`, `style_input` /
  `StyleInput`, `Cascade` (`for_document`, `with_fullscreen`), `build_cascade`,
  `ComputedStyle`, `StyleOptions`, `user_agent_css` / `fullscreen_user_agent_css`, and the
  root font helpers (`root_font_px`, `document_root_font_px`, `root_font_scope`).
- **Layout** (`layout`): `layout`, `layout_with_options`, `layout_with_images`,
  `layout_in_viewport`, `layout_recorded` / `layout_recorded_in_viewport` (`LaidOut` +
  `NodeSources`: box → source node, for PDF/UA tagging and links). Options are in
  `LayoutOptions`. The galley types are `Fragment`, `FragmentContent`, `ImagePlacement`,
  `ClipRect`, `BoxShadow`, `LinearGradient`, `Transform2D` and `BreakMeta`.
- **Text** (`text`): `FontRegistry` (`new`, `FontRegistry::system(SystemFontPolicy)` for the
  process-wide installed fonts), `FontFace`, `layout_text`, `TextStyle`, `LineBox`, `Align`,
  `WhiteSpace`. `cmap` provides the reverse cmap for text extraction.
- **Images** (`image`): `ImageResolver` / `NoImages`, `probe` (header size and EXIF
  orientation for PNG / JPEG / GIF / WebP / BMP / ICO / AVIF / JPEG XL, and SVG under
  `svg`), `probe_natural` (`naturalWidth/Height`), `PngFrame` (the one PNG decode),
  `decode_data_url`. Truncated bodies follow Blink's load decision.
- **SVG** (`svg`, under `svg-rasterize`): `svg::rasterize(bytes, scale, Oversize)` and
  `natural_size`.
- **Live documents** (`live`, "surf mode"): `live::capture` / `live::snapshot` over an
  embedder's composed tree. They take pre-matched rules (`PreMatched`) and inline
  overrides, with fixed budgets; failures are `LiveError`.
  `layout_snapshot::{LayoutSnapshot, snapshot_live_tree, snapshot_html_live_tree_with_text, …}`
  records source provenance; `unsupported_inline_geometry` fails a single element instead
  of the whole snapshot.
- **CSSOM-View** (`cssom_view`): `metrics`, `hit` (`elements_at_point`), `scroll`,
  `visibility` (`checkVisibility`), `inline`, `text` (`caret`), `range` (Range rects), and
  the `Composition` scope graph. The embedder supplies scroll state and effective `zoom`.
- Diagnostics: `Diagnostics`, `RenderError`, `CompileError`, `Lint`. Markup types: `Tag`,
  `Attr`. The crate re-exports `turbo_dom`.

## Gates

- Line coverage: **99.86%** measured, CI floor **99.8** (blocking). Run locally with
  `cargo tarpaulin --config crates/turbo-dom-layout/tarpaulin.toml`.
- CI `layout` job:
  - clippy pedantic `-D warnings`: default, all shipped features, no default features;
  - tests: default, `svg`, `perf`, `serde,test-support`;
  - `--no-default-features` against an exact list of 43 known failures (tests that assume
    the bundled fonts; `scripts/baseline-red/`, turbo-html2pdf#28);
  - wasm32 and musl checks.
- MSRV 1.89 check.
- Pull requests run on Linux; macOS is added in the release run.
- Cyclomatic complexity ≤ 15 is **report-only** until the post-release refactor. It
  replaces the old cc ≤ 5 allow-list.

## Testing

```bash
cargo test -p turbo-dom-layout                       # default (bundled-fonts)
cargo test -p turbo-dom-layout --features svg        # + perf, serde,test-support
python3 scripts/check-baseline-red.py scripts/baseline-red/turbo-dom-layout-no-default-features.tsv -- -p turbo-dom-layout --no-default-features
```

- **Fixture fonts.** Text-geometry tests lay out with
  `test_support::fixture_fonts()` (`assets/fonts`: Liberation Serif / Inter / Fira Code as
  `serif` / `sans-serif` / `monospace`), never with host fonts. Their expected values come
  from Chrome 156 with the same files loaded through `@font-face`
  (`scripts/fixture-font-chrome-controls.mjs`). Only the tests of the host-font policy
  itself (`native_serif_policy`, `original_text_units`) read installed fonts, and they are
  macOS-only.
- **Chrome 156 controls.** Fixtures in `tests/fixtures/*_chrome15x.json` are recorded by the
  repository's `scripts/*-chrome-control.mjs`:
  - `html-document` (document styling and table / block layout);
  - `live-geometry` (live tables, fullscreen);
  - `cssom-view` (scroll / visibility / caret / Range);
  - `image-formats`, `image-orientation`, `image-truncation`;
  - `cascade-selectors`.

  The tests that assert them include `tests/html_document_chrome.rs`,
  `tests/cascade_selectors_chrome.rs`, `tests/image_*_chrome.rs` and
  `src/live/chrome_tests.rs`.
- `test-support` also gives downstream crates these fixtures, `ComputedStyle` pairs and
  tree literals. turbo-html2pdf's tests use it instead of copying `assets/fonts`.

## Links

[turbo-dom](../turbo-dom/README.md) · [turbo-dom-raster](../turbo-dom-raster/README.md) ·
[docs/selector-engine.md](../../docs/selector-engine.md) ·
[docs/CONSOLIDATION-TESTS.md](../../docs/CONSOLIDATION-TESTS.md) ·
[CHANGELOG](../../CHANGELOG.md) (the behaviour fixes, each with its issue and control) ·
[assets/fonts/NOTICE.md](assets/fonts/NOTICE.md)

License: MIT (bundled fonts: OFL 1.1, see NOTICE).
