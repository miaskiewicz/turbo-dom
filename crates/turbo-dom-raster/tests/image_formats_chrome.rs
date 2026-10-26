//! Screenshots of the image formats Chrome 156 decodes (miaskiewicz/turbo-surf#30).
//!
//! The control is turbo-dom-layout's `tests/fixtures/image_formats_chrome156.json`
//! (turbo-dom's `scripts/image-formats-chrome-control.mjs`): one page of `<img
//! src="data:…">` in AVIF, BMP, ICO, CUR, SVG and TIFF over a `#00ffff` backdrop, with
//! Chrome's colour class (R/G/B/Y/W/T = backdrop) at the four quadrant centres of each
//! box. The page is screenshotted here with every source supplied as a fetched asset
//! (turbo-surf passes them so) and the samples compared. AVIF pixels need the `avif`
//! feature (without it an AVIF is laid out at its size and painted as the placeholder).
use std::collections::HashMap;

use turbo_dom_layout::image::decode_data_url;
use turbo_dom_raster::{screenshot_png_with_opts, ImageAssets, Viewport};

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

/// Sources whose box this engine sizes differently from Chrome for reasons outside the
/// format support under test: SVGs without a definite size (Chrome stretches an
/// absolutely positioned one to its containing block / uses fractional sizes; the
/// screenshot lays SVGs out at `ceil(css size)`) and TIFF (Chrome's broken-image icon).
/// (The raw-utf8 SVG `data:` source, whose `src` holds a literal `<svg`, is compared since
/// the inline-`<svg>` conversion works on parsed elements: miaskiewicz/turbo-dom#25.)
const OTHER_SIZING: &[&str] = &["svg-viewbox-only", "svg-no-size", "svg-fractional", "tiff"];

/// AVIF `irot`/`imir` cases that do not paint like Chrome yet (turbo-dom#45): the quarter
/// turns lay out unturned; with the `avif` decoder every transform paints unturned. Each
/// must still differ, so a fixed one is removed.
const PENDING: &[&str] = &["avif-irot90", "avif-irot270", "avif-irot90-imir0"];
const PENDING_DECODED: &[&str] = &["avif-irot180", "avif-imir0", "avif-imir1"];

fn pending(id: &str) -> bool {
    PENDING.contains(&id) || (cfg!(feature = "avif") && PENDING_DECODED.contains(&id))
}

#[test]
fn decoded_pixels_match_chrome_156() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../turbo-dom-layout/tests/fixtures/image_formats_chrome156.json"
    );
    let fixture: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).unwrap();
    let html = fixture["html"].as_str().unwrap();
    let mut assets: ImageAssets = HashMap::new();
    for case in fixture["cases"].as_array().unwrap() {
        let src = case["src"].as_str().unwrap();
        assets.insert(
            src.replace('"', "&quot;").replace("&quot;", "\""),
            decode_data_url(src).unwrap().bytes,
        );
    }
    // Taller than the control's 800px viewport: its screenshot is the full page, whose
    // last rows run past 800px (the page has no media/viewport units).
    let viewport = Viewport {
        width: 1280,
        height: 900,
    };
    let png = screenshot_png_with_opts(html, "", viewport, &assets, false, false).expect("png");
    let pm = tiny_skia::Pixmap::decode_png(&png).expect("decode");
    let mut failures = Vec::new();
    let mut compared = 0;
    for case in fixture["cases"].as_array().unwrap() {
        let id = case["id"].as_str().unwrap();
        if OTHER_SIZING.contains(&id) {
            continue;
        }
        // Without the `avif` feature an AVIF is laid out at its size but not decoded:
        // the box paints as the light-grey placeholder (class W; the backdrop is T).
        let want = if id.starts_with("avif") && !cfg!(feature = "avif") {
            "WWWW"
        } else {
            case["samples"].as_str().unwrap()
        };
        let got: String = case["points"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| {
                let (x, y) = (p[0].as_u64().unwrap() as u32, p[1].as_u64().unwrap() as u32);
                let c = pm.pixel(x, y).expect("in bounds");
                class([c.red(), c.green(), c.blue()])
            })
            .collect();
        compared += 1;
        if (got != want) != pending(id) {
            failures.push(format!(
                "{id}: {got}, want {want} (Chrome {})",
                case["samples"]
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {compared} differ:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
