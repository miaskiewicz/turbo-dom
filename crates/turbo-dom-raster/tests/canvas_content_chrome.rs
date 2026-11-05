//! `<canvas>` content in page screenshots against Chrome 156 (miaskiewicz/turbo-dom#291).
//!
//! The control is `tests/fixtures/canvas_content_chrome156.json` (turbo-dom's
//! `scripts/canvas-content-chrome-control.mjs`): pages over a `#00ffff` backdrop whose
//! `<canvas>` elements Chrome draws into — an op log in the render tier's `ctx._ops` tuple
//! format, or a straight-alpha RGBA bitmap of four quadrant colours — with the screenshot's
//! RGB at each probe point. Each case is screenshot here with the same content supplied
//! through [`ScreenshotOptions::canvases`], keyed by the canvas's index in tree order, and
//! both the PNG and the (resvg-rasterized) SVG screenshot must match Chrome: the bitmap at the
//! canvas's `width` / `height` size, scaled into the content box by `object-fit` /
//! `object-position`, inside border and padding, clipped to the content box and its rounded
//! corners, turned by the box's transform, upright in a vertical writing mode.
use turbo_dom_layout::image::decode_data_url;
use turbo_dom_raster::{
    screenshot_png_with_options, screenshot_svg_with_options, CanvasContent, CanvasContents,
    ImageAssets, ScreenshotOptions, Viewport,
};

const VIEWPORT: Viewport = Viewport {
    width: 400,
    height: 300,
};

/// A probe channel may differ from Chrome's by this much (rounding of translucent pixels).
const TOLERANCE: u8 = 2;

fn fixture() -> serde_json::Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/canvas_content_chrome156.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).unwrap()
}

/// A `size` bitmap of the four `quadrants` colours (top-left, top-right, bottom-left,
/// bottom-right), as the control writes it with `putImageData`.
fn quadrant_bitmap(spec: &serde_json::Value) -> CanvasContent {
    let size: [u32; 2] = serde_json::from_value(spec["size"].clone()).unwrap();
    let quadrants: Vec<[u8; 4]> = serde_json::from_value(spec["quadrants"].clone()).unwrap();
    let [w, h] = size;
    let mut pixels = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let q = usize::from(y >= h / 2) * 2 + usize::from(x >= w / 2);
            pixels.extend_from_slice(&quadrants[q]);
        }
    }
    CanvasContent::Rgba {
        width: w,
        height: h,
        pixels,
    }
}

/// The case's canvas contents, keyed by tree-order index.
fn canvases(case: &serde_json::Value) -> CanvasContents {
    case["canvases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|content| {
            let index = content["index"].as_u64().unwrap() as usize;
            let content = match content.get("ops") {
                Some(ops) => CanvasContent::Ops(ops.to_string()),
                None => quadrant_bitmap(&content["bitmap"]),
            };
            (index, content)
        })
        .collect()
}

/// Every probe of every case whose RGB differs from Chrome's, in the PNG and the SVG
/// screenshot.
fn differences() -> Vec<String> {
    let fixture = fixture();
    let mut failures = Vec::new();
    for case in fixture["cases"].as_array().unwrap() {
        let id = case["id"].as_str().unwrap();
        let html = case["html"].as_str().unwrap();
        let contents = canvases(case);
        let mut options = ScreenshotOptions::default();
        options.canvases = Some(&contents);
        let png =
            screenshot_png_with_options(html, "", VIEWPORT, &ImageAssets::new(), &options).unwrap();
        let png = tiny_skia::Pixmap::decode_png(&png).unwrap();
        let svg =
            screenshot_svg_with_options(html, "", VIEWPORT, &ImageAssets::new(), &options).unwrap();
        let svg = rasterize_svg(&svg);
        for probe in case["probes"].as_array().unwrap() {
            let (x, y) = (
                probe["x"].as_u64().unwrap() as u32,
                probe["y"].as_u64().unwrap() as u32,
            );
            let chrome: [u8; 3] = serde_json::from_value(probe["rgb"].clone()).unwrap();
            let p = png.pixel(x, y).unwrap();
            let s = svg.pixel(x, y).unwrap().demultiply();
            for (kind, got) in [
                ("png", [p.red(), p.green(), p.blue()]),
                ("svg", [s.red(), s.green(), s.blue()]),
            ] {
                if got
                    .iter()
                    .zip(chrome)
                    .any(|(&g, c)| g.abs_diff(c) > TOLERANCE)
                {
                    failures.push(format!("{id} {kind} ({x},{y}): {got:?}, Chrome {chrome:?}"));
                }
            }
        }
    }
    failures
}

#[test]
fn canvas_content_paints_in_its_box_like_chrome_156() {
    let failures = differences();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The SVG screenshot rasterized by resvg (as `paint_chrome.rs` does: this resvg build has
/// no raster-image decoders, so each embedded PNG `data:` image becomes an inline `<svg>` of
/// its pixels with the same box).
fn rasterize_svg(svg: &str) -> resvg::tiny_skia::Pixmap {
    use resvg::{tiny_skia as rsk, usvg};
    use std::fmt::Write;
    const PNG: &str = "href=\"data:image/png;base64,";
    let mut svg = svg.to_string();
    while let Some(at) = svg.find(PNG) {
        let start = at + "href=\"".len();
        let end = start + svg[start..].find('"').unwrap();
        let png = decode_data_url(&svg[start..end]).unwrap().bytes;
        let img = image::load_from_memory(&png).unwrap().to_rgba8();
        let mut pixels = format!(
            "viewBox='0 0 {0} {1}' shape-rendering='crispEdges'>",
            img.width(),
            img.height()
        );
        // Identical rows share their rects (one rect per run, as tall as the rows).
        let row = |y: u32| {
            (0..img.width())
                .map(|x| img.get_pixel(x, y).0)
                .collect::<Vec<_>>()
        };
        let mut y = 0;
        while y < img.height() {
            let rows = (y..img.height()).take_while(|&r| row(r) == row(y)).count() as u32;
            let mut x = 0;
            while x < img.width() {
                let p = img.get_pixel(x, y).0;
                let run = (x..img.width())
                    .take_while(|&e| img.get_pixel(e, y).0 == p)
                    .count() as u32;
                let _ = write!(
                    pixels,
                    "<rect x='{x}' y='{y}' width='{run}' height='{rows}' fill='rgb({},{},{})' fill-opacity='{}'/>",
                    p[0],
                    p[1],
                    p[2],
                    f32::from(p[3]) / 255.0
                );
                x += run;
            }
            y += rows;
        }
        pixels.push_str("</svg>");
        let open = svg[..at].rfind("<image ").unwrap();
        let close = end + svg[end..].find("/>").unwrap() + 2;
        let attrs = svg[open + "<image ".len()..at].to_string();
        svg.replace_range(open..close, &format!("<svg {attrs}{pixels}"));
    }
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).expect("valid svg");
    let mut pm = rsk::Pixmap::new(VIEWPORT.width, VIEWPORT.height).unwrap();
    resvg::render(&tree, rsk::Transform::identity(), &mut pm.as_mut());
    pm
}
