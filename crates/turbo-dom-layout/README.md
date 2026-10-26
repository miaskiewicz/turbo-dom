# turbo-dom-layout

The native **HTML/CSS layout engine** of turbo-dom: a style cascade over an owned
node tree, box generation, block / inline / flex / grid / table layout into one
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
| `xref`, `pdf-ua`, `print-color` | off | TEMPORARY PDF-side gates whose cfg'd fields live in the layout code; forwarded by turbo-html2pdf-core's features of the same name. Replaced by runtime layout options in the consolidation's S2 step — do not enable them from other crates |
| `test-support` | off | fixtures for downstream tests (`FontFace::collection_fixture_faces`); never in production |

## Provenance

Relocated **verbatim** from `turbo-html2pdf-core` 0.3.9
(miaskiewicz/turbo-html2pdf @ e2d7c4d) in the turbo-dom consolidation (step S1).
`scripts/verify-copied-layout.sh` re-derives the moved files from that commit with
`git archive` and diffs them, allowing only the rewrites listed in
`scripts/verify-copied-layout.rules`. The per-test move manifest is
`docs/CONSOLIDATION-TESTS.md`.

## Tests

```bash
cargo test -p turbo-dom-layout                         # default (bundled-fonts)
cargo test -p turbo-dom-layout --features svg
cargo test -p turbo-dom-layout --features perf
cargo test -p turbo-dom-layout --features xref,pdf-ua,print-color
cargo test -p turbo-dom-layout --no-default-features   # 44 known failures: tests that assume bundled fonts (turbo-html2pdf#28)
cargo tarpaulin --config crates/turbo-dom-layout/tarpaulin.toml
```

MSRV 1.89. License: MIT.
