//! `text-orientation: mixed` per character like Chrome 156 (UAX #50 `Vertical_Orientation`,
//! R2 review #8): in a `vertical-rl` line of the fixture Liberation Serif, a character
//! Chrome sets upright (©, ®, ±, ×, ÷, §, ■, ▲, ‰, №, ™: Vo=U) advances by the face's
//! ascent + descent, one it turns (a, -, «, €, ‐: Vo=R) by its horizontal advance. In
//! sequences a variation selector or ZWJ follows the previous character and the
//! character after a ZWJ keeps its own orientation (`a ZWJ ©`: a turned `a`, an upright
//! `©`).
//! Control: `tests/fixtures/vertical_orientation_chrome156.json`
//! (`scripts/vertical-orientation-chrome-control.mjs`): each one-character span's box.
use serde_json::Value;
use turbo_dom_layout::test_support::fixture_fonts;
use turbo_dom_layout::{layout_boxes_tree, parse_html_tree, Diagnostics, HtmlOptions};

#[test]
fn characters_stand_upright_or_turn_like_chrome_156() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/vertical_orientation_chrome156.json"
    );
    let fixture: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert!(fixture["chrome"].as_str().unwrap().starts_with("156."));
    let tree = parse_html_tree(fixture["html"].as_str().unwrap());
    let mut diags = Diagnostics::default();
    let got = layout_boxes_tree(
        &tree,
        "",
        800.0,
        600.0,
        &fixture_fonts(),
        &HtmlOptions::default(),
        &mut diags,
    );
    let mut failures = Vec::new();
    let boxes = fixture["boxes"].as_array().unwrap();
    for want in boxes {
        let cid = want["cid"].as_str().unwrap();
        let text = want["text"].as_str().unwrap();
        let ours = got
            .iter()
            .find(|b| b.cid == cid)
            .map_or(f32::NAN, |b| b.height);
        // The advance along the line; Chrome's upright advance rounds the face's
        // ascent + descent to whole pixels.
        let chrome = want["rect"][3].as_f64().unwrap() as f32;
        if (ours - chrome).abs() > 1.0 {
            failures.push(format!("{text:?} ({cid}): advance {ours}, Chrome {chrome}"));
        }
    }
    assert_eq!(boxes.len(), 20);
    assert!(
        failures.is_empty(),
        "{} differences from Chrome 156:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
