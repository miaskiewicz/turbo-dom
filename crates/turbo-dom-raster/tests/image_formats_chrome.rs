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
    // Classify a pixel into a colour class by whether each channel is bright (>160) or
    // dark (<96). The match form keeps cyclomatic complexity low (one match, no chained
    // boolean operators) while preserving the original six-way classification.
    match (r > 160, g > 160, b > 160, r < 96, g < 96, b < 96) {
        (true, true, true, _, _, _) => 'W',
        (_, true, true, true, _, _) => 'T',
        (true, _, _, _, true, true) => 'R',
        (_, true, _, true, _, true) => 'G',
        (_, _, true, true, true, _) => 'B',
        (true, true, _, _, _, true) => 'Y',
        _ => '?',
    }
}

/// Sources whose box this engine sizes differently from Chrome for reasons outside the
/// format support under test: SVGs without a definite size (Chrome stretches an
/// absolutely positioned one to its containing block / uses fractional sizes; the
/// screenshot lays SVGs out at `ceil(css size)`) and TIFF (Chrome's broken-image icon).
/// (The raw-utf8 SVG `data:` source, whose `src` holds a literal `<svg`, is compared since
/// the inline-`<svg>` conversion works on parsed elements: miaskiewicz/turbo-dom#25.)
const OTHER_SIZING: &[&str] = &["svg-viewbox-only", "svg-no-size", "svg-fractional", "tiff"];

fn expected_samples<'a>(id: &str, case: &'a serde_json::Value) -> &'a str {
    if (id.starts_with("avif") && !cfg!(feature = "avif"))
        || (id.starts_with("jxl") && !cfg!(feature = "jxl"))
    {
        "WWWW"
    } else {
        case["samples"].as_str().unwrap()
    }
}

fn compare_jxl(id: &str, case: &serde_json::Value, pm: &tiny_skia::Pixmap) -> Option<String> {
    let got: Vec<[u8; 3]> = case["points"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            let (x, y) = (p[0].as_u64().unwrap() as u32, p[1].as_u64().unwrap() as u32);
            let c = pm.pixel(x, y).expect("in bounds");
            [c.red(), c.green(), c.blue()]
        })
        .collect();
    let want: Vec<[u8; 3]> = serde_json::from_value(case["rgb"].clone()).unwrap();
    let near = got
        .iter()
        .zip(&want)
        .all(|(g, w)| g.iter().zip(w).all(|(a, b)| a.abs_diff(*b) <= 8));
    (!near).then(|| format!("{id}: {got:?}, Chrome {want:?}"))
}

fn compare_class(id: &str, case: &serde_json::Value, pm: &tiny_skia::Pixmap, want: &str) -> Option<String> {
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
    (got != want).then(|| format!("{id}: {got}, want {want} (Chrome {})", case["samples"]))
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
    // last rows run past 800px, to 978px (the page has no media/viewport units).
    let viewport = Viewport {
        width: 1280,
        height: 1000,
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
        let want = expected_samples(id, case);
        if id.starts_with("jxl") && cfg!(feature = "jxl") {
            compared += 1;
            if let Some(msg) = compare_jxl(id, case, &pm) {
                failures.push(msg);
            }
            continue;
        }
        compared += 1;
        if let Some(msg) = compare_class(id, case, &pm, want) {
            failures.push(msg);
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {compared} differ:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
