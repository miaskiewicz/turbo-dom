//! List markers painted like Chrome 156 (miaskiewicz/turbo-dom#128).
//!
//! The control is turbo-dom-layout's `tests/fixtures/list_markers_chrome156.json`
//! (`scripts/list-marker-chrome-control.mjs`, copied here as
//! `tests/fixtures/list_markers_chrome156.json`): pages in the fixture Liberation Serif
//! (the bundled face this crate's font registry resolves) with, per `::marker`, Chrome's
//! marker box and the bounding box of the screenshot's ink within 1px of it (not past the
//! box's end, where the item's text starts). Each case is screenshot as PNG and as SVG
//! (rasterized by resvg) and the ink in Chrome's marker region, taken relative to the list
//! item's border box (`layout_boxes`, the static layout the screenshot paints), compared
//! with Chrome's: the left / right edges within 1px (anti-aliasing), the top / bottom
//! within 2px — the static text path places a line's glyphs up to 1.5px below Chrome's
//! (unrounded font ascent, its own line height) and the marker sits on that line's
//! baseline. So the marker text, the disc / circle / square shapes and their colour are
//! painted where Chrome paints them.
use std::collections::HashMap;

use turbo_dom_layout::{layout_boxes, Diagnostics, FontRegistry};
use turbo_dom_raster::{screenshot_png_with_opts, screenshot_svg_with_opts, Viewport};

const VIEWPORT: Viewport = Viewport {
    width: 800,
    height: 600,
};

fn fixture() -> serde_json::Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/list_markers_chrome156.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).unwrap()
}

/// `(width, height, rgba)` of a screenshot.
type Pixels = (u32, u32, Vec<u8>);

/// The ink bounding box `[x0, y0, x1, y1]` within `rect` grown by 1px except at its end
/// (where the item's text starts), as the control measures it: pixels more than 96 below
/// white in a channel.
fn ink((width, height, rgba): &Pixels, rect: [f64; 4]) -> Option<[i64; 4]> {
    let x0 = (rect[0].floor() as i64 - 1).max(0);
    let y0 = (rect[1].floor() as i64 - 1).max(0);
    let x1 = ((rect[0] + rect[2]).ceil() as i64).min(i64::from(*width));
    let y1 = ((rect[1] + rect[3]).ceil() as i64 + 1).min(i64::from(*height));
    let mut found: Option<[i64; 4]> = None;
    for y in y0..y1 {
        for x in x0..x1 {
            let i = ((y * i64::from(*width) + x) * 4) as usize;
            if !rgba[i..i + 3].iter().any(|&v| 255 - i64::from(v) > 96) {
                continue;
            }
            found = Some(found.map_or([x, y, x + 1, y + 1], |b| {
                [b[0].min(x), b[1].min(y), b[2].max(x + 1), b[3].max(y + 1)]
            }));
        }
    }
    found
}

fn png_pixels(html: &str) -> Pixels {
    let png = screenshot_png_with_opts(html, "", VIEWPORT, &HashMap::new(), false, false).unwrap();
    let pm = tiny_skia::Pixmap::decode_png(&png).unwrap();
    (pm.width(), pm.height(), pm.data().to_vec())
}

fn svg_pixels(html: &str) -> Pixels {
    use resvg::{tiny_skia as rsk, usvg};
    let svg = screenshot_svg_with_opts(html, "", VIEWPORT, &HashMap::new(), false, false).unwrap();
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    let mut pm = rsk::Pixmap::new(VIEWPORT.width, VIEWPORT.height).unwrap();
    pm.fill(rsk::Color::WHITE);
    resvg::render(&tree, rsk::Transform::identity(), &mut pm.as_mut());
    (pm.width(), pm.height(), pm.data().to_vec())
}

/// Every marker whose ink differs from Chrome's in the `paint`ed screenshots.
fn differences(paint: fn(&str) -> Pixels) -> Vec<String> {
    let fixture = fixture();
    assert!(fixture["chrome"].as_str().unwrap().starts_with("156."));
    let mut failures = Vec::new();
    let mut compared = 0;
    let fonts = FontRegistry::new();
    for case in fixture["cases"].as_array().unwrap() {
        let id = case["id"].as_str().unwrap();
        let html = case["html"].as_str().unwrap();
        let pixels = paint(html);
        let mut diags = Diagnostics::default();
        let boxes = layout_boxes(html, "", 800.0, 600.0, &fonts, &mut diags).unwrap();
        for marker in case["markers"].as_array().unwrap() {
            let cid = marker["cid"].as_str().unwrap();
            // The screenshot's static layout stacks lines at its own text metrics, so each
            // marker is compared relative to its list item's border box.
            let item = boxes.iter().find(|b| b.cid == cid).expect("list item box");
            let chrome = case["boxes"]
                .as_array()
                .unwrap()
                .iter()
                .find(|b| b["cid"] == cid)
                .expect("Chrome's list item box");
            let shift = [
                f64::from(item.x) - chrome["rect"][0].as_f64().unwrap(),
                f64::from(item.y) - chrome["rect"][1].as_f64().unwrap(),
            ];
            let r = &marker["rect"];
            let rect = [0, 1, 2, 3].map(|i| r[i].as_f64().unwrap() + shift.get(i).unwrap_or(&0.0));
            let want = marker["ink"]["box"]
                .as_array()
                .map(|b| [0, 1, 2, 3].map(|i| b[i].as_i64().unwrap()));
            let back = shift.map(|v| v.round() as i64);
            let got = ink(&pixels, rect).map(|b| {
                [
                    b[0] - back[0],
                    b[1] - back[1],
                    b[2] - back[0],
                    b[3] - back[1],
                ]
            });
            compared += 1;
            let close = match (got, want) {
                (Some(got), Some(want)) => got
                    .iter()
                    .zip(want)
                    .zip([1, 2, 1, 2])
                    .all(|((g, w), tolerance)| (g - w).abs() <= tolerance),
                (got, want) => got == want,
            };
            if !close {
                failures.push(format!("{id} {cid}: ink {got:?}, Chrome {want:?}"));
            }
        }
    }
    assert_eq!(compared, 84);
    failures
}

#[test]
fn list_markers_paint_like_chrome_156_png() {
    let failures = differences(png_pixels);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn list_markers_paint_like_chrome_156_svg() {
    let failures = differences(svg_pixels);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
