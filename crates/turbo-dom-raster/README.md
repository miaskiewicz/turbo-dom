# turbo-dom-raster

Synthetic screenshots with no browser: turn an HTML **snapshot** into a PNG or SVG.
[`turbo-dom-layout`](../turbo-dom-layout) lays the HTML out into a positioned
`Fragment` display list; this crate paints it two ways — a raster painter
(tiny-skia) and a vector painter (SVG) — walking the same fragments in CSS paint
order, so the two agree. Caller-supplied `<img>` / `background-image` bytes
(PNG / JPEG / GIF / WebP / SVG) are decoded and painted; it does no I/O itself.

It also replays a recorded 2D-canvas op log (`canvas_ops_png` / `canvas_ops_rgba`)
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

turbo-dom-layout is used with its default features (`bundled-fonts`); its `svg`
feature is never enabled from here (SVG images are rasterized by this crate's own
resvg path).

## Provenance

Relocated **verbatim** from `turbo-surf-raster` 0.5.5 (miaskiewicz/turbo-surf @
b1de4eb, including `webgl.rs`) in the turbo-dom consolidation (step S1); the only
code change is the dependency `turbo-html2pdf-core` -> `turbo-dom-layout` (same API,
same names). `scripts/verify-copied-layout.sh` checks the copy.

```bash
cargo test -p turbo-dom-raster        # 40 unit tests + tests/screenshot.rs (24)
cargo check -p turbo-dom-raster --features gpu-metal,coretext
```

MSRV 1.89. License: MIT.
