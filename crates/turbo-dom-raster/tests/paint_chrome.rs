//! Screenshot pixels against Chrome 156 (P5b-W5).
//!
//! The control is `tests/fixtures/paint_chrome156.json` (turbo-dom's
//! `scripts/paint-chrome-control.mjs`): pages over a `#00ffff` backdrop with Chrome's colour
//! class (R/G/B/Y/W/T = backdrop/`?`) at each case's probe points and the
//! `getBoundingClientRect()` of every `[data-cid]` element. Each case is screenshot here
//! (its `<img src="data:…">` bytes supplied as fetched assets keyed by the raw `src`) and
//! laid out through turbo-dom-layout's `layout_boxes` (the same static layout the
//! screenshot paints, plus the zero-area `clip: rect()` boxes it leaves out:
//! miaskiewicz/turbo-dom#75).
//! - `caption-*`: miaskiewicz/turbo-dom#51, table captions in the table wrapper box; the
//!   table's background and border paint the grid box only.
//! - `clip-*`: miaskiewicz/turbo-dom#52, an absolutely positioned box's `clip: rect()`
//!   clips it and its descendants (it keeps its border box); a zero-area clip paints
//!   nothing (its box still has its border box, #75); `clip` does not apply to other boxes.
use std::collections::HashMap;

use turbo_dom_layout::image::decode_data_url;
use turbo_dom_layout::{layout_boxes, Diagnostics, FontRegistry};
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
        "/tests/fixtures/paint_chrome156.json"
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

/// Every difference from Chrome of the cases `select` picks: probe colour classes of the
/// screenshot and the `layout_boxes` geometry.
fn differences(select: impl Fn(&str) -> bool) -> Vec<String> {
    let fixture = fixture();
    let fonts = FontRegistry::new();
    let mut failures = Vec::new();
    let mut compared = 0;
    for case in fixture["cases"].as_array().unwrap() {
        let id = case["id"].as_str().unwrap();
        if !select(id) {
            continue;
        }
        compared += 1;
        let html = case["html"].as_str().unwrap();
        let png =
            screenshot_png_with_opts(html, "", VIEWPORT, &assets(case), false, false).expect("png");
        let pm = tiny_skia::Pixmap::decode_png(&png).expect("decode");
        for probe in case["probes"].as_array().unwrap() {
            let (x, y) = (
                probe["x"].as_u64().unwrap() as u32,
                probe["y"].as_u64().unwrap() as u32,
            );
            let c = pm.pixel(x, y).expect("in bounds");
            let got = class([c.red(), c.green(), c.blue()]);
            let want = probe["class"].as_str().unwrap();
            if got.to_string() != want {
                failures.push(format!("{id} ({x},{y}): {got}, Chrome {want}"));
            }
        }
        let mut diags = Diagnostics::default();
        let boxes = layout_boxes(html, "", 1280.0, 800.0, &fonts, &mut diags).unwrap();
        for want in case["boxes"].as_array().unwrap() {
            let cid = want["cid"].as_str().unwrap();
            let Some(b) = boxes.iter().find(|b| b.cid == cid) else {
                failures.push(format!("{id}: no box for data-cid={cid}"));
                continue;
            };
            for (field, got) in [
                ("x", b.x),
                ("y", b.y),
                ("width", b.width),
                ("height", b.height),
            ] {
                let chrome = want[field].as_f64().unwrap() as f32;
                if (got - chrome).abs() > 0.01 {
                    failures.push(format!("{id}: {cid}.{field} = {got}, Chrome {chrome}"));
                }
            }
        }
    }
    assert!(compared > 0, "no case selected");
    failures
}

/// Caption cases that also need another fix: `caption-narrow`'s `width` is the table's
/// border box (miaskiewicz/turbo-dom#62, [`tables_are_border_box_sized_like_chrome_156`]).
fn pending_case(id: &str) -> bool {
    id == "caption-narrow"
}

#[test]
fn table_captions_paint_outside_the_grid_like_chrome_156() {
    let failures = differences(|id| id.starts_with("caption-") && !pending_case(id));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn tables_are_border_box_sized_like_chrome_156() {
    let failures = differences(pending_case);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn clip_rect_clips_the_box_and_its_descendants_like_chrome_156() {
    let failures = differences(|id| id.starts_with("clip-"));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// miaskiewicz/turbo-dom#81: the screenshot's initial containing block is its viewport.
#[test]
fn fixed_and_icb_boxes_anchor_to_the_viewport_like_chrome_156() {
    let failures = differences(|id| id.starts_with("icb-"));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The vector screenshot clips a `clip: rect()` box with a `<clipPath>` of its region and
/// leaves out a box whose region is empty (miaskiewicz/turbo-dom#52).
#[test]
fn svg_screenshot_clips_with_a_clip_path() {
    let fixture = fixture();
    let svg = |id: &str| {
        let case = fixture["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["id"] == id)
            .unwrap();
        let html = case["html"].as_str().unwrap();
        screenshot_svg_with_opts(html, "", VIEWPORT, &assets(case), false, false).expect("svg")
    };
    let partial = svg("clip-partial");
    assert_eq!(partial.matches("<clipPath").count(), 1, "{partial}");
    assert!(
        partial.contains(r#"<rect x="10.00" y="10.00" width="30.00" height="30.00"/></clipPath>"#)
    );
    assert!(partial.contains(r##"fill="#ff0000""##));
    let empty = svg("clip-zero-area-auto");
    assert!(
        !empty.contains("<clipPath") && !empty.contains(r##"fill="#ff0000""##),
        "{empty}"
    );
}
