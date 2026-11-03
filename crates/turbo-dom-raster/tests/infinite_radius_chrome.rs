//! An infinite `border-radius` paints like Chrome 156 (R2 review #5): the radius is
//! clamped to a finite length before the overlap rule, so the box becomes a pill (one
//! huge corner: a quarter circle as tall as the box; huge / 10px: square corners) and
//! never NaN radii. Control: `tests/fixtures/infinite_radius_chrome156.json`
//! (`scripts/infinite-radius-chrome-control.mjs`), a 4px probe grid over each box where
//! Chrome's pixel is solid red (`r`) or white (`w`); its anti-aliased probes (`-`) are not
//! asserted. Each case is screenshot as PNG and as SVG (rasterized by resvg).
use std::collections::HashMap;

use turbo_dom_raster::{screenshot_png_with_opts, screenshot_svg_with_opts, Viewport};

const VIEWPORT: Viewport = Viewport {
    width: 300,
    height: 120,
};

/// `(width, rgba)` of a screenshot.
type Pixels = (u32, Vec<u8>);

fn png_pixels(html: &str) -> Pixels {
    let png = screenshot_png_with_opts(html, "", VIEWPORT, &HashMap::new(), false, false).unwrap();
    let pm = tiny_skia::Pixmap::decode_png(&png).unwrap();
    (pm.width(), pm.data().to_vec())
}

fn svg_pixels(html: &str) -> Pixels {
    use resvg::{tiny_skia as rsk, usvg};
    let svg = screenshot_svg_with_opts(html, "", VIEWPORT, &HashMap::new(), false, false).unwrap();
    assert!(!svg.contains("NaN"), "NaN in the SVG");
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    let mut pm = rsk::Pixmap::new(VIEWPORT.width, VIEWPORT.height).unwrap();
    pm.fill(rsk::Color::WHITE);
    resvg::render(&tree, rsk::Transform::identity(), &mut pm.as_mut());
    (pm.width(), pm.data().to_vec())
}

fn class(rgba: &[u8]) -> char {
    let [r, g, b] = [rgba[0], rgba[1], rgba[2]];
    if r > 200 && g < 60 && b < 60 {
        'r'
    } else if r > 230 && g > 230 && b > 230 {
        'w'
    } else {
        '-'
    }
}

fn differences(paint: fn(&str) -> Pixels) -> Vec<String> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/infinite_radius_chrome156.json"
    );
    let fixture: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert!(fixture["chrome"].as_str().unwrap().starts_with("156."));
    let mut failures = Vec::new();
    let mut compared = 0;
    for case in fixture["cases"].as_array().unwrap() {
        let id = case["id"].as_str().unwrap();
        let (width, rgba) = paint(case["html"].as_str().unwrap());
        let origin = [0, 1].map(|i| case["origin"][i].as_u64().unwrap() as u32);
        let step = case["step"].as_u64().unwrap() as u32;
        for (row, line) in case["rows"].as_array().unwrap().iter().enumerate() {
            for (col, want) in line.as_str().unwrap().chars().enumerate() {
                if want == '-' {
                    continue;
                }
                let x = origin[0] + col as u32 * step;
                let y = origin[1] + row as u32 * step;
                let i = ((y * width + x) * 4) as usize;
                let got = class(&rgba[i..i + 4]);
                compared += 1;
                if got != want {
                    failures.push(format!("{id} ({x},{y}): {got}, Chrome {want}"));
                }
            }
        }
    }
    assert!(compared > 2000, "{compared} probes");
    failures
}

#[test]
fn an_infinite_border_radius_paints_like_chrome_156_png() {
    let failures = differences(png_pixels);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn an_infinite_border_radius_paints_like_chrome_156_svg() {
    let failures = differences(svg_pixels);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
