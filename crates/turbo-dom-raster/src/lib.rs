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
mod canvas_content;
mod capture_resources;
pub use capture_resources::{capture_resources, CaptureResources, ImageMetadata};
mod canvas_ops;
mod capture_geometry;
#[cfg(test)]
mod form_native_chrome_tests;
mod glyph;
mod glyph_raster;
mod image_paint;
mod jxl;
mod masks;
mod paint_canvas;
#[cfg(all(target_os = "macos", feature = "coretext"))]
mod paint_canvas_coretext;
#[cfg(feature = "gpu-metal")]
mod paint_canvas_gpu;
mod paint_png;
mod paint_svg;
mod prepared;
pub use prepared::PreparedScreenshot;
mod rrect;
mod style_extract;
mod tile_cell;
#[cfg(test)]
mod vertical_upright_tests;
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
/// feature never regresses, it only swaps the pixel backend. Text draws with the bundled
/// faces ([`canvas_ops_png_with_fonts`] takes the caller's).
/// A canvas over Chrome's maximum area (268,435,456 pixels) is an `Err` (blank in Chrome).
pub fn canvas_ops_png(width: u32, height: u32, ops_json: &str) -> Result<Vec<u8>, String> {
    canvas_ops_png_with_fonts(width, height, ops_json, canvas_ops::bundled_fonts())
}

/// [`canvas_ops_png`] drawing text with `fonts`: the canvas `font` family list resolves
/// against the caller's registry (for example the page's `@font-face` faces from
/// [`FontFace::from_web_font_bytes`](turbo_dom_layout::FontFace::from_web_font_bytes) on top
/// of [`FontRegistry::new`]'s bundled set), so web fonts paint (miaskiewicz/turbo-dom#304).
pub fn canvas_ops_png_with_fonts(
    width: u32,
    height: u32,
    ops_json: &str,
    fonts: &FontRegistry,
) -> Result<Vec<u8>, String> {
    let (w, h) = (width.max(1), height.max(1));
    if canvas_too_large(w, h) {
        return Err(format!(
            "canvas {w}x{h} exceeds Chrome's maximum canvas area"
        ));
    }
    let ops = canvas_ops::parse_ops(ops_json)?;
    #[cfg(feature = "gpu-metal")]
    {
        match paint_canvas_gpu::replay(w, h, &ops, fonts) {
            Ok(png) => Ok(png),
            Err(_) => paint_canvas::replay(w, h, &ops, fonts),
        }
    }
    #[cfg(not(feature = "gpu-metal"))]
    {
        paint_canvas::replay(w, h, &ops, fonts)
    }
}
/// Replay a 2D-canvas draw list into RAW straight-alpha RGBA8 (top-left origin, tightly packed) —
/// the byte layout `getImageData` returns. Software (tiny-skia): getImageData reads back exact
/// pixels with no PNG-encoder in the loop, so shapes/solids match a real browser byte-for-byte.
/// A canvas over Chrome's maximum area is an `Err`, as for [`canvas_ops_png`]. Text draws with
/// the bundled faces ([`canvas_ops_rgba_with_fonts`] takes the caller's).
pub fn canvas_ops_rgba(width: u32, height: u32, ops_json: &str) -> Result<Vec<u8>, String> {
    canvas_ops_rgba_with_fonts(width, height, ops_json, canvas_ops::bundled_fonts())
}

/// [`canvas_ops_rgba`] drawing text with `fonts` (see [`canvas_ops_png_with_fonts`]).
pub fn canvas_ops_rgba_with_fonts(
    width: u32,
    height: u32,
    ops_json: &str,
    fonts: &FontRegistry,
) -> Result<Vec<u8>, String> {
    let (w, h) = (width.max(1), height.max(1));
    if canvas_too_large(w, h) {
        return Err(format!(
            "canvas {w}x{h} exceeds Chrome's maximum canvas area"
        ));
    }
    let ops = canvas_ops::parse_ops(ops_json)?;
    paint_canvas::replay_rgba(w, h, &ops, fonts)
}
/// Chrome's maximum canvas area (268,435,456 pixels: 16384 x 16384, 32768 x 8192). A larger
/// canvas is blank in Chrome (`toDataURL()` is `data:,`); replaying it would allocate
/// gigabytes, so it is an `Err`.
fn canvas_too_large(width: u32, height: u32) -> bool {
    u64::from(width) * u64::from(height) > 268_435_456
}
pub use canvas_content::{CanvasContent, CanvasContents};
pub use masks::{screenshot_element_rects, ScreenshotMask, ScreenshotRect};
pub use style_extract::{
    delazy_images, delazy_tree, image_urls, image_urls_in_css, stylesheet_hrefs,
};

use std::collections::{HashMap, HashSet};

use turbo_dom_layout::text::{FontRegistry, SystemFontPolicy};
use turbo_dom_layout::turbo_dom::Tree;
use turbo_dom_layout::{
    layout_html_tree_capture_in_viewport_with, layout_html_tree_in_viewport_with, parse_html_tree,
    Attr, Diagnostics, Fragment, FragmentContent, HtmlOptions, ImageCtx, Rgba, StyledNode, Tag,
    INDETERMINATE_ATTR,
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
    let content = capture_geometry::content_bottom(galley)
        .max(viewport.height as f32)
        .ceil()
        .max(1.0) as u32;
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
    let mut options = ScreenshotOptions::default();
    (options.full_page, options.system_fonts) = (full_page, system_fonts);
    screenshot_png_with_options(html, external_css, viewport, images, &options)
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
    let mut options = ScreenshotOptions::default();
    (options.full_page, options.system_fonts) = (full_page, system_fonts);
    screenshot_svg_with_options(html, external_css, viewport, images, &options)
}

/// The switches and embedder-supplied inputs of a `*_with_options` screenshot beyond the
/// page, its stylesheets and its images. Start from [`ScreenshotOptions::default`] (every
/// switch off, nothing supplied) and set the fields you need; new fields are added with a
/// default that keeps today's render.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct ScreenshotOptions<'a> {
    /// Intrinsic headers retained independently of fetched pixel bytes.
    pub image_metadata: Option<&'a ImageMetadata>,
    /// CSS-pixel overlays painted after the document, in slice order. Empty by default.
    pub masks: &'a [ScreenshotMask],
    /// Paint the full content height instead of the viewport height (see
    /// [`screenshot_png_with_assets`]).
    pub full_page: bool,
    /// Leave unauthored canvas pixels transparent, retaining authored backgrounds.
    pub omit_background: bool,
    /// Output pixels per CSS pixel, independent of the layout viewport. Default: 1.
    /// Canvas dimensions round to nearest, with positive half-pixel ties up.
    /// The native painter must be able to represent the finite positive scale.
    pub device_scale: f64,
    /// Resolve fonts against the installed system fonts too (see
    /// [`screenshot_png_with_opts`]).
    pub system_fonts: bool,
    /// What the page drew into its `<canvas>` elements, painted in their boxes
    /// (miaskiewicz/turbo-dom#291; see [`CanvasContents`]). `None`: every canvas is blank.
    pub canvases: Option<&'a CanvasContents>,
    /// The faces the page lays out and paints with, in place of the bundled set (and of
    /// `system_fonts`): a caller that loads the page's `@font-face` sources (with
    /// [`FontFace::from_web_font_bytes`](turbo_dom_layout::FontFace::from_web_font_bytes), on
    /// top of [`FontRegistry::new`] or a clone of [`FontRegistry::system`]) gets them laid out
    /// with Chrome's web-font metrics and painted (miaskiewicz/turbo-dom#304). The raster does
    /// no I/O and does not read `@font-face` rules itself. `None`: the bundled (or system) set.
    pub fonts: Option<&'a FontRegistry>,
    /// The IDL-state flags a script set in the live DOM — the state the HTML markup does
    /// not carry — handed over for the snapshot to paint
    /// (miaskiewicz/turbo-dom#293, #371; see [`IdlStates`]). `None`: no IDL state is set,
    /// every control paints from its markup alone (today's default).
    pub idl_states: Option<&'a IdlStates>,
}

/// The IDL-state flags a live embedder (turbo-surf) carries into a static screenshot — the
/// state a script set at runtime that the serialized HTML does not express. Fields key
/// elements by their tree-order index among `document.querySelectorAll(...)` for the
/// matching selector (the same keying scheme as [`CanvasContents`]); an element with no
/// entry keeps its markup state (miaskiewicz/turbo-dom#371).
#[derive(Debug, Clone, Default)]
pub struct IdlStates {
    /// The tree-order indices of `<input>` elements (as
    /// `document.querySelectorAll('input')` enumerates them) whose IDL `indeterminate`
    /// flag a script has set to `true`: a checkbox at one of these indices paints the
    /// Chrome indeterminate dash, checked or not, exactly as
    /// [`turbo_dom_layout::INDETERMINATE_ATTR`] on its styled box (any other control
    /// ignores the flag, like Chrome).
    pub indeterminate_inputs: HashSet<usize>,
}

impl Default for ScreenshotOptions<'_> {
    fn default() -> Self {
        Self {
            image_metadata: None,
            masks: &[],
            full_page: false,
            omit_background: false,
            device_scale: 1.0,
            system_fonts: false,
            canvases: None,
            fonts: None,
            idl_states: None,
        }
    }
}

impl ScreenshotOptions<'_> {
    /// Lay out with the caller's [`fonts`](Self::fonts), else the bundled (or, with
    /// `system_fonts`, system) registry.
    fn with_fonts<T>(&self, draw: impl FnOnce(&FontRegistry) -> T) -> T {
        match self.fonts {
            Some(fonts) => draw(fonts),
            None => draw(&font_registry(self.system_fonts)),
        }
    }
}

/// Lay `html` out at `viewport` and paint it into a PNG, with caller-fetched
/// `external_css` and `images` (see [`screenshot_png_with_assets`]) and `options`.
pub fn screenshot_png_with_options(
    html: &str,
    external_css: &str,
    viewport: Viewport,
    images: &ImageAssets,
    options: &ScreenshotOptions,
) -> Result<Vec<u8>, String> {
    PreparedScreenshot::new(html, external_css, viewport, images, options).png()
}

/// [`screenshot_png_with_options`] as a standalone SVG document (images embed as base64
/// `data:` URIs).
pub fn screenshot_svg_with_options(
    html: &str,
    external_css: &str,
    viewport: Viewport,
    images: &ImageAssets,
    options: &ScreenshotOptions,
) -> Result<String, String> {
    PreparedScreenshot::new(html, external_css, viewport, images, options).svg()
}

pub(crate) fn screenshot_pixel_size(
    width: u32,
    height: u32,
    scale: f64,
) -> Result<(u32, u32), String> {
    if !scale.is_finite() || scale <= 0.0 {
        return Err("screenshot device scale must be finite and positive".into());
    }
    let native_scale = scale as f32;
    if !native_scale.is_finite() || native_scale == 0.0 {
        return Err("screenshot device scale is outside the native painter range".into());
    }
    let size = (
        screenshot_axis(width, scale),
        screenshot_axis(height, scale),
    );
    if size.0 == 0 || size.1 == 0 {
        return Err(format!("bad canvas {}x{}", size.0, size.1));
    }
    if canvas_too_large(size.0, size.1) {
        return Err("scaled screenshot exceeds the canvas size limit".into());
    }
    Ok(size)
}

// Chrome's device-pixel canvas size rounds a double-precision CSS-size/DPR
// product to the nearest pixel, with positive half-pixel ties rounding up.
fn screenshot_axis(css: u32, scale: f64) -> u32 {
    (f64::from(css) * scale).round() as u32
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

/// Canvas `measureText(text).width` under a canvas `font` shorthand (`"italic bold 16px
/// Inter, serif"`), as Chrome 156 measures it: the shorthand's size, weight, style and
/// family list resolve a face from the bundled set the canvas painter draws with (700 and
/// 400 are distinct faces), and the text is measured word by word with Blink's 16.16 glyph
/// advances ([`FontFace::measure_canvas`](turbo_dom_layout::FontFace::measure_canvas)).
#[must_use]
pub fn measure_canvas_text(text: &str, font: &str) -> f64 {
    measure_canvas_text_with_fonts(text, font, canvas_ops::bundled_fonts())
}

/// [`measure_canvas_text`] against the caller's `fonts`, the registry
/// [`canvas_ops_png_with_fonts`] draws with.
#[must_use]
pub fn measure_canvas_text_with_fonts(text: &str, font: &str, fonts: &FontRegistry) -> f64 {
    let (size_px, families) = canvas_ops::parse_font(font);
    let (weight, italic) = canvas_ops::parse_font_style(font);
    canvas_ops::resolve_face(fonts, &families, weight, italic).measure_canvas(text, size_px)
}

/// The colour to fill the whole image with before painting: the canvas background
/// turbo-dom-layout's HTML drive propagated from the root element (or `<body>`) onto
/// the galley root, composited over the white initial canvas; white when the page
/// sets none.
fn screenshot_canvas_background(galley: &Fragment, omit_background: bool) -> Rgba {
    if !omit_background {
        return canvas_background(galley);
    }
    match &galley.content {
        FragmentContent::Box {
            background: Some(bg),
            ..
        } => *bg,
        _ => Rgba {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        },
    }
}

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
/// (miaskiewicz/turbo-dom#27: no caller bytes needed). The embedder's canvas contents join
/// `decoded` under their canvases' names (miaskiewicz/turbo-dom#291), counted before the
/// inline `<svg>`s (and any canvas in one) are replaced.
fn parse_document(
    html: &str,
    decoded: &mut image_paint::DecodedAssets,
    options: &ScreenshotOptions,
) -> Tree {
    prepare_document(parse_html_tree(html), decoded, options, None)
}

type SvgReplacement<'a> = &'a mut dyn FnMut(
    &Tree,
    turbo_dom_layout::turbo_dom::Handle,
    turbo_dom_layout::turbo_dom::Handle,
);

fn prepare_document(
    mut tree: Tree,
    decoded: &mut image_paint::DecodedAssets,
    options: &ScreenshotOptions,
    replaced: Option<SvgReplacement<'_>>,
) -> Tree {
    style_extract::delazy_tree(&mut tree);
    image_paint::add_img_data_urls(decoded, &tree);
    if let Some(canvases) = options.canvases {
        canvas_content::add_canvases(decoded, &tree, canvases);
    }
    let inline_svgs = match replaced {
        Some(replaced) => image_paint::inline_svg_to_img_with(&mut tree, replaced),
        None => image_paint::inline_svg_to_img(&mut tree),
    };
    image_paint::add_inline_svg_assets(decoded, &inline_svgs);
    tree
}

#[cfg(test)]
thread_local! {
    static LAYOUT_PASSES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Drive the borrowed layout engine: lay the parsed document out at viewport width
/// `viewport` width over `fonts`. The page's own `<style>` sheets (head and body) are
/// collected by the layout's HTML drive (which never lays out
/// `<style>`/`<script>`/`<noscript>`/`<template>`); the caller's fetched `<link>` sheets
/// precede them.
fn lay_out(
    tree: &Tree,
    external_css: &str,
    viewport: Viewport,
    images: &image_paint::DecodedAssets,
    fonts: &FontRegistry,
    screenshot_options: &ScreenshotOptions,
) -> Fragment {
    #[cfg(test)]
    LAYOUT_PASSES.with(|passes| passes.set(passes.get() + 1));
    let Viewport { width, height } = viewport;
    // Tell the cascade the viewport HEIGHT so `@media (min-height/max-height:…)`
    // conditions evaluate correctly (else every height feature is ignored and its
    // rules always apply — Google's tall search box is `display:none` below
    // `max-height:575px`, so it always vanished → blank render).
    turbo_dom_layout::set_media_viewport_height(height as f32);
    let mut diags = Diagnostics::default();
    let resolver = image_paint::AssetResolver(images);
    // No page-height basis in a viewport render (and no caps: `uncapped_images`).
    let image_ctx = ImageCtx {
        resolver: &resolver,
        body_height: None,
    };
    // Images size by CSS alone, like Chrome (no paged-media caps).
    let options = HtmlOptions {
        linked_css: external_css,
        uncapped_images: true,
        ..HtmlOptions::default()
    };
    // The initial containing block is the viewport: `position: fixed` boxes and
    // absolutely positioned ones it contains anchor `bottom` / `%` insets to its
    // height (miaskiewicz/turbo-dom#81). The post-cascade hook carries the live
    // IDL state (today: the `indeterminate` checkbox flag) a screenshot has no
    // script to set onto the matching styled boxes (miaskiewicz/turbo-dom#371).
    let layout = if screenshot_options.full_page {
        layout_html_tree_capture_in_viewport_with
    } else {
        layout_html_tree_in_viewport_with
    };
    layout(
        tree,
        "",
        (width as f32, height as f32),
        fonts,
        &image_ctx,
        &options,
        |styled: &mut [StyledNode]| apply_idl_states(styled, screenshot_options.idl_states),
        &mut diags,
    )
}

/// Push [`INDETERMINATE_ATTR`] onto each `<input>` whose tree-order index among the
/// document's `<input>` elements (as `document.querySelectorAll('input')` enumerates them)
/// is in [`IdlStates::indeterminate_inputs`]. `None` or an empty set is a no-op, so the
/// styled document (and the render) is byte-for-byte unchanged when no IDL state is set.
fn apply_idl_states(styled: &mut [StyledNode], idl_states: Option<&IdlStates>) {
    let Some(states) = idl_states else { return };
    if states.indeterminate_inputs.is_empty() {
        return;
    }
    let mut index = 0usize;
    mark_indeterminate_inputs(styled, &states.indeterminate_inputs, &mut index);
}

fn mark_indeterminate_inputs(
    nodes: &mut [StyledNode],
    indices: &HashSet<usize>,
    index: &mut usize,
) {
    for node in nodes {
        let StyledNode::Element(el) = node else {
            continue;
        };
        let is_input = matches!(&el.tag, Tag::Html(name) if name == "input");
        if is_input {
            if indices.contains(index) {
                el.attrs.push(Attr {
                    name: INDETERMINATE_ATTR.into(),
                    value: String::new(),
                });
            }
            *index += 1;
        }
        mark_indeterminate_inputs(&mut el.children, indices, index);
    }
}

#[cfg(test)]
mod canvas_area_tests {
    use super::{canvas_ops_png, canvas_ops_rgba, canvas_too_large};

    #[test]
    fn chromes_max_canvas_area_is_268_435_456_pixels() {
        // Chrome 156: 16384 x 16384 and 32768 x 8192 draw; 16385 x 16384 is a blank canvas.
        assert!(!canvas_too_large(16384, 16384));
        assert!(!canvas_too_large(32768, 8192));
        assert!(canvas_too_large(16385, 16384));
        assert!(canvas_too_large(u32::MAX, u32::MAX));
    }

    #[test]
    fn an_over_area_canvas_is_refused_before_allocating() {
        // Unbounded, a 30000 x 30000 fill peaked at 7.2 GB.
        let fill = r##"[["fillRect",[0,0,30000,30000],"#f00",null,null,null,null,1,null]]"##;
        assert!(canvas_ops_png(30000, 30000, fill).is_err());
        assert!(canvas_ops_rgba(16385, 16384, "[]").is_err());
        assert_eq!(canvas_ops_rgba(2, 1, "[]").unwrap().len(), 8);
    }
}
