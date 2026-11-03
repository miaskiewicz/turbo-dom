//! miaskiewicz/turbo-dom#252 on the HTML (PDF / screenshot) path: the `media-*` cases of
//! the live-geometry Chrome 156 control (`tests/fixtures/live_geometry_chrome156.json`,
//! `scripts/live-geometry-chrome-control.mjs`) laid out through [`layout_boxes`] with the
//! repo's fixture fonts. `<canvas>`, `<video>`, `<audio controls>`, `<object>` and
//! `<embed>` are inline replaced boxes in horizontal and vertical writing modes; an
//! element Chrome renders no box for (`<audio>` without controls, `<embed>` without a
//! source, a canvas's fallback content) reads an all-zero rect there and has no box here.
//! The live path asserts the same cases exactly (`live::chrome_tests`).
use serde_json::Value;
use turbo_dom_layout::{layout_boxes, Diagnostics};

#[test]
fn media_elements_are_replaced_boxes_on_the_html_path_like_chrome_156() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/live_geometry_chrome156.json"
    );
    let fixture: Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("the Chrome fixture")).unwrap();
    let fonts = turbo_dom_layout::test_support::fixture_fonts();
    let mut failures = Vec::new();
    let mut compared = 0;
    for case in fixture["cases"].as_array().unwrap() {
        let id = case["id"].as_str().unwrap();
        if !id.starts_with("media-") {
            continue;
        }
        let html = case["html"].as_str().unwrap();
        let mut diags = Diagnostics::default();
        let got = layout_boxes(html, "", 1280.0, 800.0, &fonts, &mut diags).unwrap();
        for want in case["boxes"].as_array().unwrap() {
            compared += 1;
            let cid = want["cid"].as_str().unwrap();
            let field = |key: &str| want[key].as_f64().unwrap() as f32;
            let chrome = [field("x"), field("y"), field("width"), field("height")];
            let Some(b) = got.iter().find(|b| b.cid == cid) else {
                if chrome != [0.0; 4] {
                    failures.push(format!("{id}: no box for {cid}, Chrome {chrome:?}"));
                }
                continue;
            };
            let ours = [b.x, b.y, b.width, b.height];
            // The media box's size is exact; its position and its line's extent move
            // with the text beside it, which this path measures with its own advances
            // and line metrics (within 0.3px of Chrome's here; the live path is exact).
            let tolerance = |i: usize| if cid == "m" && i >= 2 { 0.01 } else { 0.3 };
            let off = (0..4).any(|i| (ours[i] - chrome[i]).abs() > tolerance(i));
            if off {
                failures.push(format!("{id}: {cid} = {ours:?}, Chrome {chrome:?}"));
            }
        }
    }
    assert!(compared > 0);
    assert!(
        failures.is_empty(),
        "{} boxes differ from Chrome 156:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
