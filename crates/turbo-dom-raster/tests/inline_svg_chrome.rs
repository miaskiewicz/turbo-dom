//! Inline `<svg>` screenshots against Chrome 156 (miaskiewicz/turbo-dom#25 item 1).
//!
//! The control is `tests/fixtures/inline_svg_chrome156.json` (turbo-dom's
//! `scripts/inline-svg-chrome-control.mjs`): pages of absolutely positioned boxes over a
//! `#00ffff` backdrop, with Chrome's colour class (R/G/B/Y/W/T = backdrop) at the four
//! quadrant centres of each `[data-cid]` box. The screenshot paints the document's parsed
//! `<svg>` elements — a `<svg` inside an attribute value (a raw-UTF-8 SVG `data:` URL, a
//! `title`) or a `<script>` string is not one — and serializes each as namespace-well-formed
//! XML for the rasterizer (`xlink:href` without a declaration, HTML character references,
//! the parser's camelCase SVG names). Each case's `<img src="data:…">` bytes are supplied as
//! fetched assets keyed by the raw `src` (the screenshot API's contract).
use std::collections::HashMap;

use turbo_dom_layout::image::decode_data_url;
use turbo_dom_raster::{screenshot_png_with_opts, screenshot_svg_with_opts, ImageAssets, Viewport};

fn class([r, g, b]: [u8; 3]) -> char {
    let hi = |v: u8| v > 160;
    let lo = |v: u8| v < 96;
    if hi(r) && hi(g) && hi(b) {
        'W'
    } else if lo(r) && hi(g) && hi(b) {
        'T'
    } else if hi(r) && lo(g) && lo(b) {
        'R'
    } else if lo(r) && hi(g) && lo(b) {
        'G'
    } else if lo(r) && lo(g) && hi(b) {
        'B'
    } else if hi(r) && hi(g) && lo(b) {
        'Y'
    } else {
        '?'
    }
}

fn fixture() -> serde_json::Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/inline_svg_chrome156.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).unwrap()
}

fn assets(case: &serde_json::Value) -> ImageAssets {
    let mut assets: ImageAssets = HashMap::new();
    for src in case["assets"].as_array().unwrap() {
        let src = src.as_str().unwrap();
        assets.insert(src.to_string(), decode_data_url(src).unwrap().bytes);
    }
    assets
}

const VIEWPORT: Viewport = Viewport {
    width: 1280,
    height: 800,
};

#[test]
fn inline_svg_pixels_match_chrome_156() {
    let fixture = fixture();
    let mut failures = Vec::new();
    for case in fixture["cases"].as_array().unwrap() {
        let html = case["html"].as_str().unwrap();
        let png =
            screenshot_png_with_opts(html, "", VIEWPORT, &assets(case), false, false).expect("png");
        let pm = tiny_skia::Pixmap::decode_png(&png).expect("decode");
        for b in case["boxes"].as_array().unwrap() {
            let got: String = b["points"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| {
                    let (x, y) = (p[0].as_u64().unwrap() as u32, p[1].as_u64().unwrap() as u32);
                    let c = pm.pixel(x, y).expect("in bounds");
                    class([c.red(), c.green(), c.blue()])
                })
                .collect();
            let want = b["samples"].as_str().unwrap();
            if got != want {
                failures.push(format!("{} {}: {got}, Chrome {want}", case["id"], b["cid"]));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The vector screenshot embeds the same inline SVG rasters: one `<image>` per painted
/// inline `<svg>`/`<img>`, and none for a `<svg` that is only source text.
#[test]
fn svg_screenshot_embeds_one_image_per_parsed_svg_element() {
    let fixture = fixture();
    for case in fixture["cases"].as_array().unwrap() {
        let html = case["html"].as_str().unwrap();
        let svg =
            screenshot_svg_with_opts(html, "", VIEWPORT, &assets(case), false, false).expect("svg");
        let images = svg.matches("<image").count();
        let want = match case["id"].as_str().unwrap() {
            "script-strings" | "attribute-values" => 0,
            "data-url-and-inline" => 2,
            _ => 1,
        };
        assert_eq!(images, want, "{}", case["id"]);
    }
}
