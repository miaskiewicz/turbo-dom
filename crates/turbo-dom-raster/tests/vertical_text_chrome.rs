//! Vertical writing-mode text painted like Chrome 156 (miaskiewicz/turbo-dom#234).
//!
//! The control is `tests/fixtures/vertical_text_chrome156.json`
//! (`scripts/vertical-text-chrome-control.mjs`): red text in the fixture Liberation Serif
//! (the bundled face this crate's font registry resolves) in `vertical-rl` / `vertical-lr`
//! boxes, with each `[data-cid]` box and the bounding box of the screenshot's ink in it.
//! Each case is screenshot as PNG and as SVG (rasterized by resvg) and the ink in Chrome's
//! box, taken relative to the box `layout_boxes` places (the static layout the screenshot
//! paints), compared with Chrome's within 1px: the Latin glyphs are turned a quarter
//! clockwise, their baseline to the left of the line's centre, and the lines stack along
//! the horizontal block axis.
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
        "/tests/fixtures/vertical_text_chrome156.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).unwrap()
}

/// `(width, height, rgba)` of a screenshot.
type Pixels = (u32, u32, Vec<u8>);

/// The ink bounding box `[x0, y0, x1, y1]` within `rect`, as the control measures it:
/// pixels more than 96 below white in a channel.
fn ink((width, _, rgba): &Pixels, rect: [i64; 4]) -> Option<[i64; 4]> {
    let mut found: Option<[i64; 4]> = None;
    for y in rect[1]..rect[3] {
        for x in rect[0]..rect[2] {
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

/// Every box whose ink differs from Chrome's in the `paint`ed screenshots.
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
        for chrome in case["boxes"].as_array().unwrap() {
            let cid = chrome["cid"].as_str().unwrap();
            let laid = boxes.iter().find(|b| b.cid == cid).expect("box");
            let r = [0, 1, 2, 3].map(|i| chrome["rect"][i].as_f64().unwrap());
            for (got, want, field) in [(laid.width, r[2], "width"), (laid.height, r[3], "height")] {
                if (f64::from(got) - want).abs() > 0.01 {
                    failures.push(format!("{id} {cid}: {field} {got}, Chrome {want}"));
                }
            }
            // Compared relative to the box (the static layout may place it elsewhere).
            let shift = [
                f64::from(laid.x).round() as i64 - r[0].round() as i64,
                f64::from(laid.y).round() as i64 - r[1].round() as i64,
            ];
            let rect = [
                r[0].floor() as i64 + shift[0],
                r[1].floor() as i64 + shift[1],
                (r[0] + r[2]).ceil() as i64 + shift[0],
                (r[1] + r[3]).ceil() as i64 + shift[1],
            ];
            let got = ink(&pixels, rect).map(|b| {
                [
                    b[0] - shift[0],
                    b[1] - shift[1],
                    b[2] - shift[0],
                    b[3] - shift[1],
                ]
            });
            let want = chrome["ink"]["box"]
                .as_array()
                .map(|b| [0, 1, 2, 3].map(|i| b[i].as_i64().unwrap()));
            compared += 1;
            let close = match (got, want) {
                (Some(got), Some(want)) => got.iter().zip(want).all(|(g, w)| (g - w).abs() <= 1),
                (got, want) => got == want,
            };
            if !close {
                failures.push(format!("{id} {cid}: ink {got:?}, Chrome {want:?}"));
            }
        }
    }
    assert_eq!(compared, 6);
    failures
}

#[test]
fn vertical_text_paints_like_chrome_156_png() {
    let failures = differences(png_pixels);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn vertical_text_paints_like_chrome_156_svg() {
    let failures = differences(svg_pixels);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
