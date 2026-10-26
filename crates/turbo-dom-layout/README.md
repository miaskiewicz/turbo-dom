# turbo-dom-layout

The native **HTML/CSS layout engine** of turbo-dom: a style cascade over a
turbo-dom `Tree` (`style_input` over a `TreeView`, or the live-DOM input `live` for an
embedder-composed document), box generation, block / inline / flex / grid / table layout
into one continuous galley of positioned `Fragment`s, a font registry with shaping
(rustybuzz) and per-character fallback, the shared image layer (`image`: intrinsic sizing
of PNG / JPEG / GIF / WebP, plus SVG behind the `svg` feature; one PNG decode; one SVG
rasterizer), and CSSOM-View style layout snapshots (`layout_snapshot`). `layout_html`
drives a raw HTML string straight to a galley.

```rust
use turbo_dom_layout::{layout_html, Diagnostics, FontRegistry, Fragment, RenderError};

fn lay_out(html: &str) -> Result<Fragment, RenderError> {
    let fonts = FontRegistry::new(); // bundled OFL faces (default `bundled-fonts`)
    let mut diags = Diagnostics::default();
    layout_html(html, "", 600.0, &fonts, &mut diags) // 600 px content width
}
```

## Inputs and options

- `TreeView` — the html2pdf-mode view of a `Tree`: elements and text only (comments,
  PIs, doctypes and `<template>` content skipped), local attribute names, an optional
  marker classifier (`MarkerKind`: opaque inline/block markers such as html2pdf's `t:`
  directives) and optional stripping of non-visual subtrees.
- `Roots::Detached(&[Handle])` styles nodes as their own flow (no ancestors, default
  parent); `Roots::Document` styles the whole document from its root element, as a
  browser does (the HTML drive): `<html>`/`<head>`/`<body>` are real elements, `rem`
  resolves against the root font size (`root_font_scope`), and `Cascade::for_document`
  adds the document UA rules (`head { display: none }`, `body { margin: 8px }`) and the
  document's own `<style>` sheets. The HTML drive also propagates the root/body
  background colour to the canvas (the galley root's background;
  `html_layout::propagate_canvas_background`) — Chrome-controlled by
  `tests/html_document_chrome.rs` (`scripts/html-document-chrome-control.mjs`).
- `StyleOptions` / `LayoutOptions` / `HtmlOptions`: `accept_cmyk` (print colour),
  `atomic_internal_links` (`<a href="#x">` as its own box), `first_node_id`.
- `layout_recorded` returns the galley plus `NodeSources` (box id → source element /
  text node, table rows, the grid boxes of captioned tables, id range) — html2pdf derives
  PDF/UA roles and links from it.
- The string HTML entries (`layout_html`, `layout_boxes`) use no markers: `t:*` are
  ordinary unknown elements, as in a browser.
- `layout_in_viewport` / `layout_html_tree_in_viewport` lay out in a viewport
  `(width, height)`: the initial containing block has the viewport's height, so a
  `position: fixed` box and an absolutely positioned box with no positioned ancestor
  resolve `bottom` / `%` insets and a `%` height against it (turbo-dom#81).
  `layout_boxes` (its `cb_height`) and turbo-dom-raster's screenshots use it; the other
  static entries leave that height unknown; `layout_recorded_in_viewport` is the
  recorded layout with that height (turbo-html2pdf's paged layout passes its page area,
  turbo-html2pdf#39).
- `live` — the live-DOM input ("surf mode"): an embedder that owns a live document
  (turbo-surf) supplies its composed children (`Fn(Handle) -> &[Handle]`), pre-matched
  rules per origin (`PreMatched`: declarations, targets with specificity, shadow-scope
  depth), CSSOM inline-style overrides and an inline-declaration parser. `live::capture`
  records the provenance under fixed budgets (50k nodes counted before node-type
  filtering, depth 256, 2 MB lossy-UTF-8 text, 2 MB qualified attributes, namespace
  rejection; exact error messages in `LiveError`); `live::snapshot` styles the composed
  tree, runs the embedder's post-cascade hook and lays it out with native text
  provenance. Qualified attribute names, no markers, sources on elements and text.
  Live layouts give table rows and row groups (`<tbody>`/`<thead>`/`<tfoot>`, an empty
  one a zero-height box at the bottom of the rows before it) border boxes with used
  metrics; an inline construct the native inline producer does not support fails only
  that element's reads (`LayoutSnapshot::unsupported_inline_geometry`), not the snapshot
  (P5b, turbo-surf#31/#36, turbo-dom#28; Chrome 156 control
  `scripts/live-geometry-chrome-control.mjs` in turbo-dom, asserted by
  `src/live/chrome_tests.rs`).
- Layout semantics shared by every path (Chrome 156-controlled, turbo-dom#28): a
  `width: auto` table is shrink-to-fit (`table-layout: fixed` needs a definite width);
  absolutely positioned and floated boxes are blockified (`inline-table` -> `table`;
  `inline-table` is an atomic inline-level table); a float met inside line content is
  placed at the top of its line, which wraps beside it; an out-of-flow box adds its own
  margins once; the anonymous block around an inline run inherits only inherited
  properties; a line holding only out-of-flow places is zero-height. CSSOM-View hit
  testing (`cssom_view::hit`) honours `clip: rect()` of absolutely positioned boxes, and
  so does the galley (turbo-dom#52): such a box keeps its box and records the region in
  `Fragment::clip` (`ClipRect`, from its border-box corner) for the painters; one whose
  clip keeps nothing at any size (the sr-only pattern) is left out of static renders
  (no paint, text or tag) — `layout_boxes` still lays it out and reports its border box
  and its descendants' like `getBoundingClientRect()` (turbo-dom#75); `clip` on any other
  box is ignored.
  Also (turbo-dom#48/#49/#50/#53/#56): absolutely positioned `width: auto` boxes shrink to
  fit; `inline-flex` / `inline-grid` are inline-level; auto-layout collapsed tables merge
  borders; percentage columns take their share; spanning cells spread over their columns.
  A table with a caption (`display: table-caption`, the UA style of `<caption>`) is a
  table wrapper box (turbo-dom#51): the table's fragment holds the captions
  (`caption-side: top` above, `bottom` below, in source order) and an anonymous grid
  box (`NodeSources::table_grids`) stretched to the widest of the grid and the captions'
  min-content, which carries the table's frame, background, `height` and rows.
  Also P5b-W5 (Chrome 156 controls in `scripts/html-document-chrome-control.mjs`): a last
  child's bottom margin is kept inside / collapsed through its parent on every policy
  (turbo-dom#41); a table's extra width goes to its auto columns, equally when empty
  (#55), and its extra height to its rows (#64); UA `table { box-sizing: border-box;
  border-spacing: 2px }` with `border-spacing` inherited and initially 0 (#62, #63);
  `border-box` widths count once in intrinsic and column widths (#65); only legacy
  `-webkit-*` alignment (UA `center`, `align` attributes) moves block children (#69); UA
  `th` is bold and centred (#70).
  Also P5b-W6 (same control): a last child's bottom margin collapses through an
  auto-height parent only when `min-height` / `max-height` leave its used height at its
  content height, else it is dropped (turbo-dom#75); an empty first child's margins
  collapse through its parent's top with the next child's (#76); `min-height` wins over
  `max-height` and `height` (#77); a percentage `height` / `min-height` / `max-height`
  resolves against a definite parent height on the static path too (#78); `display:
  flow-root` establishes a block formatting context and an empty formatting-context root is
  not self-collapsing (#88); column flex items take their basis from `height` and keep
  their `width` (#86); content-box flex item sizes include their padding and border when
  they reach taffy (#93).
  P5b-W7: a box's intrinsic widths count its children's
  horizontal margins (signed; `%` / `auto` count 0; turbo-dom#79); an `inline-block` is a
  block formatting context root, keeping its first / last child's vertical margins
  (#80); the initial containing block's height on the static path (#81, above); and the
  bugs found while fixing them: floats are shrink-to-fit (#83), out-of-flow children add
  no intrinsic width (#84), floats sit side by side in a max-content width (#85), `top` +
  `bottom` stretch an auto-height positioned box (#89), a positioned
  box resolves `%` widths against and is placed in its containing block's padding box (#100).
- Fullscreen (turbo-dom#82): while the tree's fullscreen stack (`Tree::fullscreen_stack`,
  reported by the binding) is non-empty, the cascade adds Chrome's `:not(:root):fullscreen`
  UA rules — the HTML drive itself (`Cascade::with_fullscreen`), a live-DOM embedder by
  appending `style::fullscreen_user_agent_css()` to `user_agent_css()` — and the element's
  box goes to the top layer: the end of the initial containing block's content, after its
  black `::backdrop`, above everything, free of its ancestors' offsets, transforms and
  clips. Control: `fullscreen-*` in `scripts/live-geometry-chrome-control.mjs`.
- `layout_snapshot::snapshot_{,html_}live_tree*` capture a snapshot of a `StyleInput`
  (identity paths index the styled forest).
- `image` — the ONE image layer shared by the PDF emitter and turbo-dom-raster:
  `ImageResolver::intrinsic` (default: `probe`, the header size probe for every format)
  sizes every image box; `PngFrame::decode(bytes, PngBudget)` is the one PNG decode (palette
  / low-bit / `tRNS` expanded, 16-bit kept), giving the PDF its native samples
  (`to_raster_image`: gray stays `DeviceGray`, 16-bit high byte) and the screenshot painter
  straight RGBA (`to_rgba8`, byte-identical to `image::to_rgba8`); under `svg-rasterize`,
  `svg::rasterize(bytes, scale, Oversize)` is the one SVG rasterizer with the intrinsic CSS
  size (`css_size`) separate from the oversampled pixel size (`pixel_size`): the PDF lays out
  and rasterizes at 2x clamping an axis to 8192 px, turbo-dom-raster at 1x rejecting above
  8192. JPEG stays a header read + `DCTDecode` passthrough for the PDF. The probe also
  reads the EXIF orientation (`Intrinsic::orientation`, an `Orientation`: JPEG `APP1`
  Exif, PNG `eXIf`; not WebP `EXIF`, which Chrome 156 ignores): image boxes lay out at the
  displayed size (`Intrinsic::displayed`, width/height swapped for orientations 5-8)
  unless `image-orientation: none`, and `ImagePlacement::orientation` tells the painters
  how to turn the stored pixels (`Orientation::apply_rgba` for raster; a rotated `cm` in
  the PDF). Chrome-controlled by `tests/image_orientation_chrome.rs`
  (`scripts/image-orientation-chrome-control.mjs`; fixtures from
  `scripts/make_orientation_fixtures.py`). Beyond PNG/JPEG/GIF/WebP the probe sizes BMP,
  ICO/CUR (`ico_entry`: the largest entry, as browsers show) and AVIF (the primary item's
  `ispe`, oriented by its `irot`/`imir` transform properties as Chrome displays them —
  `Orientation::then` composes them in association order, miaskiewicz/turbo-dom#45;
  `avif_payload` demuxes its AV1 streams for a decoder) and JPEG XL (bare codestream or
  container: its `SizeHeader` and `ImageMetadata` orientation, `src/image_jxl.rs`;
  miaskiewicz/turbo-dom#45 — Chrome 156 decodes it); the PDF does not embed
  them (decode emits nothing). `probe_natural` is the browser's `naturalWidth`/`Height`
  (displayed orientation; SVG under `svg-rasterize` via `svg::natural_size`, Chrome's
  300x150 default-object rules) and `decode_data_url` the Fetch-standard `data:` decoder
  (base64 and percent-encoded) — the API a live `<img>` load uses
  (miaskiewicz/turbo-surf#30); Chrome-controlled by `tests/image_formats_chrome.rs`
  (`scripts/image-formats-chrome-control.mjs`, `scripts/make_format_fixtures.py`). Both
  probes follow Blink's load decision for a cut-short body (miaskiewicz/turbo-dom#59): a
  JPEG is sized from the end of its first `SOS` header, a GIF from its first image
  descriptor's position and size, a WebP only with its whole declared RIFF payload, a PNG
  from its first `IDAT` header — before that the image is broken (no box size, no natural
  size). An SVG image (`svg::parse`, so every SVG path) needs an SVG-namespace `<svg>` root
  (any prefix; XML, so case-sensitive) and is never gzip-inflated. Chrome-controlled at every
  byte prefix by `tests/image_truncation_chrome.rs` (`scripts/image-truncation-chrome-control.mjs`,
  fixtures from `scripts/make_truncation_fixtures.py`). Chrome decides SVG by MIME type
  (`image/svg+xml` only, never sniffed); the probes take bytes only, so a host with the
  response type (turbo-surf) applies that rule itself.
- `FontRegistry::system(SystemFontPolicy)` — the process-wide installed-font registry, loaded
  once per policy (`Default` = `load_system_fonts`, `Html` = `load_system_fonts_for_html`);
  `system_cell` exposes the `OnceLock` for embedders that warm it themselves.
- `cssom_view` — CSSOM-View geometry over a live snapshot (bounding/client rects,
  offset/client/scroll metrics, hit testing, carets, Range boxes, visibility), moved
  from turbo-surf; the embedder supplies scroll state and each element's effective
  `zoom` (`cssom_view::scroll::SourceStyles`).

Consumers:
- [`turbo-html2pdf-core`](https://github.com/miaskiewicz/turbo-html2pdf) — templating,
  pagination and PDF emit on top of this crate; it re-exports this crate's modules
  at its own paths (`turbo_html2pdf_core::style::…`, `::layout::…`, `::Tag`, …), and its
  tests use this crate's `test-support` fixtures (no copy of `assets/fonts`).
- [`turbo-dom-raster`](../turbo-dom-raster) — paints the galley to PNG / SVG.
- [`turbo-surf`](https://github.com/miaskiewicz/turbo-surf) — its CSSOM-View layout
  (`getBoundingClientRect`, `elementFromPoint`, …) runs `live` + `cssom_view` over the
  page's own document.

## Features

| feature | default | what it does |
|---|---|---|
| `bundled-fonts` | on | embeds Inter / Roboto, Liberation Serif / PT Serif, Fira Code / IBM Plex Mono (OFL, see `assets/fonts/NOTICE.md`) so documents lay out with zero caller fonts |
| `svg` | off | the PDF SVG pipeline: `image::probe` / `image::decode` route `image/svg+xml` through the shared rasterizer at 2x (implies `svg-rasterize`) |
| `svg-rasterize` | off | the shared SVG rasterizer alone (`svg::rasterize`, resvg `=0.45.1`, pinned: 0.46+ changes raster output) without routing `probe`/`decode` through it — what turbo-dom-raster enables |
| `perf` | off | hot-path counters (`perf` module, `hot!` macro) |
| `serde` | off | `serde::Serialize` for the `cssom_view` metrics records (turbo-surf returns them to page script) |
| `test-support` | off | `test_support`: font fixtures read from `assets/fonts` (`FIXTURE_FONTS_DIR`, `font_bytes`, `registry`, …), `style` pairs, test-only tree literals that build a `Tree` (`literal`), `FontFace::collection_fixture_faces`; for this crate's and downstream tests (the integration tests enable it via a self dev-dependency), never in production |

## Provenance

Relocated **verbatim** from `turbo-html2pdf-core` 0.3.9
(miaskiewicz/turbo-html2pdf @ e2d7c4d) in the turbo-dom consolidation (step S1).
`scripts/verify-copied-layout.sh` re-derives the moved files from that commit with
`git archive` and diffs them against the S1 tree (`VERIFY_REF`, default the S1 merge
commit), allowing only the rewrites listed in `scripts/verify-copied-layout.rules`.
Step S2 then switched the cascade onto the turbo-dom `Tree` and replaced the
temporary `xref`/`pdf-ua`/`print-color` features with runtime options, and step S3b
replaced html2pdf's own selector parser and matcher with turbo-dom's selector engine
(`parse_for_cascade` / `match_for_cascade` over the scope `style::Navigator`; Chrome 156
control end to end in `tests/cascade_selectors_chrome.rs`, see turbo-dom's
`docs/selector-engine.md`); those edits are ordinary reviewed changes on top of S1. The
per-test move manifest is `docs/CONSOLIDATION-TESTS.md`. Step S4 added the live-DOM
input (`live`, replacing turbo-surf's tree copier) and moved turbo-surf's pure
CSSOM-View modules into `cssom_view` (turbo-surf @ 94c6150 `live_layout_*`, a move: surf
deleted its copies). Step S5 deleted the legacy owned-`Node` input (`node.rs`, `markup.rs`,
`style_tree*`; `Tag` / `Attr` / `MarkerKind` moved to `tag`), made `image` the one image
layer shared with turbo-dom-raster (size probe, PNG decode, SVG rasterizer — gated byte for
byte against the old decoders by `tests/image_layer.rs` and turbo-dom-raster's
`shared_layer_tests`), added the process-wide `FontRegistry::system` and the
`test-support` fixtures turbo-html2pdf-core's tests now use instead of a copy of
`assets/fonts`.

## Lints

`clippy::pedantic` through the workspace's `[workspace.lints.clippy]` (the turbo-dom
allow-list, plus `manual_midpoint`: `f32::midpoint` rounds through f64 and would move
geometry bits); item-level `#[allow]`s carry their reason inline. `cargo clippy -p
turbo-dom-layout --all-targets [--all-features | --no-default-features] -- -D warnings` is clean.

## Complexity

`cargo run --manifest-path <turbo-html2pdf>/tools/cc-check/Cargo.toml -- --max 5
crates/turbo-dom-layout/src` reports nothing outside `src/cssom_view/`. The surf-origin
`cssom_view` modules are allow-listed (plan §7.3) with their functions above 5 as moved:
`range::{visit 7, select 18}`, `scroll::{scroll_metrics 34, contained 9,
scroll_translation 12, propagated_body 6, viewport_metrics 16, expose 11}`,
`metrics::{element_geometry 16, offset_position 33}`, `hit::{contains 6,
elements_at_point 30, clipped 12}`, `inline::{check_style 6, geometry 10}`,
`composition::scopes 7`, `text::caret 16`, `visibility::check 18`.

## Tests

```bash
cargo test -p turbo-dom-layout                         # default (bundled-fonts)
cargo test -p turbo-dom-layout --features svg
cargo test -p turbo-dom-layout --features perf
cargo test -p turbo-dom-layout --features svg --test image_layer   # the shared image layer == the pre-S5 PDF decode
cargo test -p turbo-dom-layout --no-default-features   # 43 known failures: tests that assume bundled fonts (turbo-html2pdf#28)
cargo tarpaulin --config crates/turbo-dom-layout/tarpaulin.toml
```

MSRV 1.89. License: MIT.
