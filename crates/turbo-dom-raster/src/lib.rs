//! Synthetic screenshots for turbo-surf: turn an HTML **snapshot** into a PNG or
//! SVG image with no browser and no rendering surface of our own.
//!
//! The engine borrows turbo-dom-layout's native HTML/CSS **layout** (block/inline/
//! flex/table + real font shaping over bundled faces) to turn a raw HTML string
//! into a positioned [`Fragment`] display list, then paints that list two ways:
//! a raster [`paint_png`] (tiny-skia) and a vector [`paint_svg`]. Both walk the
//! same fragments in CSS stacking (paint) order via `Fragment::paint_order`, so
//! they agree.
//!
//! This is a *reasonably representative* render, not a pixel-faithful browser:
//! `position`/`z-index` are honored for block flow (out-of-flow boxes are placed
//! against their containing block and painted back-to-front per CSS 2.2 §9.9),
//! and `<img>`/`background-image` bytes the caller supplies are decoded + painted
//! (PNG/JPEG/GIF/WebP/BMP/ICO/CUR + SVG, AVIF under `avif`; unknown formats fall back to a
//! placeholder) — but
//! there is no JS-driven visual state beyond whatever produced the snapshot. It
//! runs only when asked — never on the fetch/extract hot path.
//!
//! Because the input is just an HTML string, any snapshot works: the initial
//! fetch, or any entry in a hydration trail.

mod avif;
mod canvas_ops;
mod glyph;
mod image_paint;
mod jxl;
mod paint_canvas;
#[cfg(all(target_os = "macos", feature = "coretext"))]
mod paint_canvas_coretext;
#[cfg(feature = "gpu-metal")]
mod paint_canvas_gpu;
mod paint_png;
mod paint_svg;
mod style_extract;
#[cfg(feature = "gpu-metal")]
mod webgl;

/// Execute a recorded WebGL call batch on the GPU; return the framebuffer as RGBA8 bytes
/// (len `width*height*4`, row-major, TOP-left origin). `None` on any failure (caller falls back
/// to its stub). See [`webgl`] for the JSON call protocol.
#[cfg(feature = "gpu-metal")]
pub use webgl::{webgl_metadata, webgl_readback};

/// Replay a recorded 2D-canvas draw list (the render tier's `ctx._ops` tuple log — see
/// [`canvas_ops`] for the format) into a `width × height` PNG. Default build rasterizes with
/// tiny-skia (software); under the `gpu-metal` feature it rasterizes on the real GPU (wgpu →
/// Metal) and, on any GPU failure, transparently falls back to tiny-skia — so enabling the
/// feature never regresses, it only swaps the pixel backend.
pub fn canvas_ops_png(width: u32, height: u32, ops_json: &str) -> Result<Vec<u8>, String> {
    let (w, h) = (width.max(1), height.max(1));
    let ops = canvas_ops::parse_ops(ops_json)?;
    #[cfg(feature = "gpu-metal")]
    {
        match paint_canvas_gpu::replay(w, h, &ops) {
            Ok(png) => Ok(png),
            Err(_) => paint_canvas::replay(w, h, &ops),
        }
    }
    #[cfg(not(feature = "gpu-metal"))]
    {
        paint_canvas::replay(w, h, &ops)
    }
}
/// Replay a 2D-canvas draw list into RAW straight-alpha RGBA8 (top-left origin, tightly packed) —
/// the byte layout `getImageData` returns. Software (tiny-skia): getImageData reads back exact
/// pixels with no PNG-encoder in the loop, so shapes/solids match a real browser byte-for-byte.
pub fn canvas_ops_rgba(width: u32, height: u32, ops_json: &str) -> Result<Vec<u8>, String> {
    let (w, h) = (width.max(1), height.max(1));
    let ops = canvas_ops::parse_ops(ops_json)?;
    paint_canvas::replay_rgba(w, h, &ops)
}
pub use style_extract::{
    delazy_images, delazy_tree, image_urls, image_urls_in_css, stylesheet_hrefs,
};

use std::collections::HashMap;

use turbo_dom_layout::text::{FontRegistry, SystemFontPolicy};
use turbo_dom_layout::turbo_dom::Tree;
use turbo_dom_layout::{
    layout_html_tree, parse_html_tree, Diagnostics, Fragment, FragmentContent, HtmlOptions,
    ImageCtx, Rgba,
};

/// Fetched image bytes keyed by their reference — the raw `<img src>` /
/// `background-image: url(...)` value exactly as [`image_urls`] returns it. The
/// caller fetches (the raster does no I/O) and hands the map to a `*_with_assets`
/// screenshot; both layout (intrinsic sizing) and paint read the same bytes.
pub type ImageAssets = HashMap<String, Vec<u8>>;

/// The empty asset map: no image bytes, so `<img>`/`background-image` boxes lay
/// out without pixels (as the `*_with_css` entries do).
fn no_assets() -> ImageAssets {
    HashMap::new()
}

/// The layout viewport a snapshot is rendered against. `width` drives CSS layout
/// (line wrapping, `%` widths); the image is `width × height` px and content
/// past the bottom edge is clipped, matching a browser viewport screenshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
}

impl Viewport {
    /// A common desktop default (overridable per call / per session).
    pub const DEFAULT: Viewport = Viewport {
        width: 1280,
        height: 800,
    };
}

impl Default for Viewport {
    fn default() -> Self {
        Viewport::DEFAULT
    }
}

/// Output encoding for a screenshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Png,
    Svg,
}

/// Lay `html` out at `viewport` and paint it into a PNG. Returns encoded bytes.
pub fn screenshot_png(html: &str, viewport: Viewport) -> Result<Vec<u8>, String> {
    screenshot_png_with_css(html, "", viewport)
}

/// Lay `html` out at `viewport` and paint it into a standalone SVG document.
pub fn screenshot_svg(html: &str, viewport: Viewport) -> Result<String, String> {
    screenshot_svg_with_css(html, "", viewport)
}

/// Like [`screenshot_png`] but with `external_css` (e.g. the concatenated bodies
/// of the page's `<link rel="stylesheet">` sheets, fetched by the caller — the
/// raster does no I/O) cascaded on top of the page's own inline styles.
pub fn screenshot_png_with_css(
    html: &str,
    external_css: &str,
    viewport: Viewport,
) -> Result<Vec<u8>, String> {
    screenshot_png_with_assets(html, external_css, viewport, &no_assets(), false)
}

/// Like [`screenshot_svg`] but with caller-fetched `external_css` (see
/// [`screenshot_png_with_css`]).
pub fn screenshot_svg_with_css(
    html: &str,
    external_css: &str,
    viewport: Viewport,
) -> Result<String, String> {
    screenshot_svg_with_assets(html, external_css, viewport, &no_assets(), false)
}

/// Upper bound on a `full_page` image's height (px) so a pathological page can't
/// allocate an unbounded canvas. ~24k px is far beyond any real page fold.
const MAX_FULL_PAGE_HEIGHT: u32 = 24_000;

/// The image height to paint: the viewport height (viewport-clipped), or — when
/// `full_page` — the laid-out content height (clamped), so nothing below the fold
/// is cut off. Width is always the viewport width.
fn paint_height(galley: &Fragment, viewport: Viewport, full_page: bool) -> u32 {
    if !full_page {
        return viewport.height;
    }
    let content = galley.bottom().ceil().max(1.0) as u32;
    content.min(MAX_FULL_PAGE_HEIGHT)
}

/// Like [`screenshot_png_with_css`] but also paints `<img>` / `background-image`
/// boxes from caller-fetched `images` (keyed as [`image_urls`] returns them; the
/// raster does no I/O). PNG + JPEG are drawn scaled into their layout box; an
/// image whose bytes are absent (or undecodable) lays out without pixels. With
/// `full_page`, the image height is the full content height (viewport width still
/// drives layout) instead of the viewport height.
pub fn screenshot_png_with_assets(
    html: &str,
    external_css: &str,
    viewport: Viewport,
    images: &ImageAssets,
    full_page: bool,
) -> Result<Vec<u8>, String> {
    screenshot_png_with_opts(html, external_css, viewport, images, full_page, false)
}

/// Like [`screenshot_svg_with_css`] but with caller-fetched `images` (see
/// [`screenshot_png_with_assets`]); images embed as base64 `data:` URIs. With
/// `full_page`, the height is the full content height.
pub fn screenshot_svg_with_assets(
    html: &str,
    external_css: &str,
    viewport: Viewport,
    images: &ImageAssets,
    full_page: bool,
) -> Result<String, String> {
    screenshot_svg_with_opts(html, external_css, viewport, images, full_page, false)
}

/// [`screenshot_png_with_assets`] plus `system_fonts`: when true, the page's
/// fonts resolve against the machine's installed system fonts (matching a browser
/// on the same host) instead of only the bundled fallback faces. Opt-in — it
/// loads the OS font set (cached across calls).
pub fn screenshot_png_with_opts(
    html: &str,
    external_css: &str,
    viewport: Viewport,
    images: &ImageAssets,
    full_page: bool,
    system_fonts: bool,
) -> Result<Vec<u8>, String> {
    let mut decoded = image_paint::decode_all(images);
    // Recover inline `data:` image masks/backgrounds the fetch step skips (nothing
    // to fetch) — e.g. Wikipedia's TOC-toggle chevron mask-image.
    image_paint::add_data_uri_images(&mut decoded, external_css, html);
    let tree = parse_document(html, &mut decoded);
    let galley = lay_out(
        &tree,
        external_css,
        viewport.width,
        viewport.height,
        &decoded,
        system_fonts,
    );
    let bg = canvas_background(&galley);
    let height = paint_height(&galley, viewport, full_page);
    paint_png::paint(&galley, viewport.width, height, bg, &decoded)
}

/// [`screenshot_svg_with_assets`] plus `system_fonts` (see
/// [`screenshot_png_with_opts`]).
pub fn screenshot_svg_with_opts(
    html: &str,
    external_css: &str,
    viewport: Viewport,
    images: &ImageAssets,
    full_page: bool,
    system_fonts: bool,
) -> Result<String, String> {
    let mut decoded = image_paint::decode_all(images);
    image_paint::add_data_uri_images(&mut decoded, external_css, html);
    let tree = parse_document(html, &mut decoded);
    let galley = lay_out(
        &tree,
        external_css,
        viewport.width,
        viewport.height,
        &decoded,
        system_fonts,
    );
    let bg = canvas_background(&galley);
    let height = paint_height(&galley, viewport, full_page);
    Ok(paint_svg::paint(
        &galley,
        viewport.width,
        height,
        bg,
        &decoded,
    ))
}

/// The font registry for a render: the bundled fallback set, plus (opt-in) every
/// installed system font. The system-font set is turbo-dom-layout's process-wide
/// registry ([`FontRegistry::system`], loaded once per process and shared with every
/// other embedder) cloned per call (the parsed faces are `Arc`-shared, so the clone is
/// cheap).
fn font_registry(system_fonts: bool) -> FontRegistry {
    if !system_fonts {
        return FontRegistry::new();
    }
    FontRegistry::system(SystemFontPolicy::Default).clone()
}

/// Measure the advance width (and an approximate line height) of `text` under a CSS
/// `font-family` list at `size_px`, using real font metrics. With `system_fonts`, the OS
/// fonts are loaded so distinct named families (Arial vs Georgia vs Courier) resolve to
/// distinct faces and yield distinct widths — what a font-detection probe reads. Returns
/// `(width_px, height_px)`. A face always resolves: the registry carries the bundled faces
/// (this crate enables turbo-dom-layout's `bundled-fonts`) and `FontRegistry::select`
/// falls back to the first one when no family matches.
#[must_use]
pub fn measure_text(text: &str, family_css: &str, size_px: f32, system_fonts: bool) -> (f32, f32) {
    let families: Vec<String> = family_css
        .split(',')
        .map(|s| {
            s.trim()
                .trim_matches(|c| c == '\'' || c == '"')
                .trim()
                .to_string()
        })
        .filter(|s| !s.is_empty())
        .collect();
    let refs: Vec<&str> = families.iter().map(std::string::String::as_str).collect();
    let reg = font_registry(system_fonts);
    let face = reg
        .select(&refs, 400, false)
        .expect("the bundled faces are registered");
    (face.measure(text, size_px, 0.0), size_px * 1.2)
}

/// The colour to fill the whole image with before painting: the canvas background
/// turbo-dom-layout's HTML drive propagated from the root element (or `<body>`) onto
/// the galley root, composited over the white initial canvas; white when the page
/// sets none.
fn canvas_background(galley: &Fragment) -> Rgba {
    let white = Rgba {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };
    match &galley.content {
        FragmentContent::Box {
            background: Some(bg),
            ..
        } => over_white(*bg),
        _ => white,
    }
}

/// `c` composited over opaque white (source-over, straight alpha).
fn over_white(c: Rgba) -> Rgba {
    let a = u32::from(c.a);
    let mix = |v: u8| ((u32::from(v) * a + 255 * (255 - a) + 127) / 255) as u8;
    Rgba {
        r: mix(c.r),
        g: mix(c.g),
        b: mix(c.b),
        a: 255,
    }
}

/// Dispatch by [`Format`]; SVG bytes are UTF-8 of the document.
pub fn screenshot(html: &str, viewport: Viewport, format: Format) -> Result<Vec<u8>, String> {
    match format {
        Format::Png => screenshot_png(html, viewport),
        Format::Svg => screenshot_svg(html, viewport).map(String::into_bytes),
    }
}

/// Parse the snapshot for layout into a turbo-dom tree (scripting on: turbo-surf rasters
/// the hydrated, JS-enabled DOM, so `<noscript>` fallbacks stay inert raw text), de-lazy its
/// `<img>` elements (fill in `src`-less lazy images — data-src/srcset/data-*-url — so their
/// boxes render; [`image_urls`] returns the same URLs, so the caller's fetched bytes resolve
/// against these boxes; miaskiewicz/turbo-dom#27: elements, never `<img` text), then replace the parsed inline `<svg>`
/// elements with `<img>`s whose rasterized SVGs join `decoded` — the engine has no
/// inline-SVG painter (miaskiewicz/turbo-dom#25: elements of the tree, never `<svg` text).
/// The document's `<img src="data:…">` images are decoded from their URLs into `decoded`
/// (miaskiewicz/turbo-dom#27: no caller bytes needed).
fn parse_document(html: &str, decoded: &mut image_paint::DecodedAssets) -> Tree {
    let mut tree = parse_html_tree(html);
    style_extract::delazy_tree(&mut tree);
    image_paint::add_img_data_urls(decoded, &tree);
    let inline_svgs = image_paint::inline_svg_to_img(&mut tree);
    image_paint::add_inline_svg_assets(decoded, &inline_svgs);
    tree
}

/// Drive the borrowed layout engine: lay the parsed document out at viewport width
/// `width` over the bundled font set. The page's own `<style>` sheets (head and body) are
/// collected by the layout's HTML drive (which never lays out
/// `<style>`/`<script>`/`<noscript>`/`<template>`); the caller's fetched `<link>` sheets
/// precede them.
fn lay_out(
    tree: &Tree,
    external_css: &str,
    width: u32,
    height: u32,
    images: &image_paint::DecodedAssets,
    system_fonts: bool,
) -> Fragment {
    // Tell the cascade the viewport HEIGHT so `@media (min-height/max-height:…)`
    // conditions evaluate correctly (else every height feature is ignored and its
    // rules always apply — Google's tall search box is `display:none` below
    // `max-height:575px`, so it always vanished → blank render).
    turbo_dom_layout::set_media_viewport_height(height as f32);
    let mut diags = Diagnostics::default();
    let resolver = image_paint::AssetResolver(images);
    // No page-height basis in a viewport render — only the 100%-width cap applies
    // (images shrink to fit their column, keeping aspect ratio).
    let image_ctx = ImageCtx {
        resolver: &resolver,
        body_height: None,
    };
    let options = HtmlOptions {
        linked_css: external_css,
        ..HtmlOptions::default()
    };
    layout_html_tree(
        tree,
        "",
        width as f32,
        &font_registry(system_fonts),
        &image_ctx,
        &options,
        &mut diags,
    )
}
