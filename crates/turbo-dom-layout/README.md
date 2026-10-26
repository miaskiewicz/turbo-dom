# turbo-dom-layout

The native **HTML/CSS layout engine** of turbo-dom: a style cascade over a
turbo-dom `Tree` (`style_input` over a `TreeView`; the legacy owned `Node` tree is
still accepted by `style_tree` until turbo-surf's copier is gone), box generation, block / inline / flex / grid / table layout into one
continuous galley of positioned `Fragment`s, a font registry with shaping
(rustybuzz) and per-character fallback, image intrinsic sizing (PNG / JPEG, plus
SVG behind the `svg` feature), and CSSOM-View style layout snapshots
(`layout_snapshot`). `layout_html` drives a raw HTML string straight to a galley.

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
  parent); `Roots::Shell` styles `<body>` content under match-only `<html>`/`<body>`
  shells (the HTML drive).
- `StyleOptions` / `LayoutOptions` / `HtmlOptions`: `accept_cmyk` (print colour),
  `atomic_internal_links` (`<a href="#x">` as its own box), `first_node_id`.
- `layout_recorded` returns the galley plus `NodeSources` (box id → source element /
  text node, table rows, id range) — html2pdf derives PDF/UA roles and links from it.
- The string HTML entries (`layout_html`, `layout_boxes`) use no markers: `t:*` are
  ordinary unknown elements, as in a browser.

Consumers:
- [`turbo-html2pdf-core`](https://github.com/miaskiewicz/turbo-html2pdf) — templating,
  pagination and PDF emit on top of this crate; it re-exports this crate's modules
  at its own paths (`turbo_html2pdf_core::style::…`, `::layout::…`, `::Node`, …).
- [`turbo-dom-raster`](../turbo-dom-raster) — paints the galley to PNG / SVG.

## Features

| feature | default | what it does |
|---|---|---|
| `bundled-fonts` | on | embeds Inter / Roboto, Liberation Serif / PT Serif, Fira Code / IBM Plex Mono (OFL, see `assets/fonts/NOTICE.md`) so documents lay out with zero caller fonts |
| `svg` | off | rasterizes `image/svg+xml` with resvg `=0.45.1` (pinned: 0.46+ changes raster output) |
| `perf` | off | hot-path counters (`perf` module, `hot!` macro) |
| `test-support` | off | fixtures for downstream tests (`FontFace::collection_fixture_faces`); never in production |

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
per-test move manifest is `docs/CONSOLIDATION-TESTS.md`.

## Tests

```bash
cargo test -p turbo-dom-layout                         # default (bundled-fonts)
cargo test -p turbo-dom-layout --features svg
cargo test -p turbo-dom-layout --features perf
TURBO_DOM_LAYOUT_EQUIV_DIRS=dir1:dir2 cargo test -p turbo-dom-layout --test tree_input_equivalence  # + corpora
cargo test -p turbo-dom-layout --no-default-features   # 44 known failures: tests that assume bundled fonts (turbo-html2pdf#28)
cargo tarpaulin --config crates/turbo-dom-layout/tarpaulin.toml
```

MSRV 1.89. License: MIT.
