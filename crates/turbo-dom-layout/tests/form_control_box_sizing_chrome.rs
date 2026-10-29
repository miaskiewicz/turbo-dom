//! Form-control `box-sizing` (miaskiewicz/turbo-dom#193): Chrome 156's `html.css` gives
//! `button`, `select`, `meter`, `progress` and `input[type=button|submit|reset|color|
//! checkbox|radio|search]` `box-sizing: border-box`, so an authored width / height
//! includes their padding and border. Text inputs, `input[type=file]` and `textarea` stay
//! content-box. Expected boxes: `tests/fixtures/form_control_box_sizing_chrome156.json`,
//! `getBoundingClientRect()` from frozen Chrome 156.0.8078.47.
use serde_json::Value;
use turbo_dom_layout::test_support::fixture_fonts;
use turbo_dom_layout::{layout_boxes_tree, parse_html_tree, Diagnostics, HtmlOptions};

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/form_control_box_sizing_chrome156.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("the Chrome fixture")).unwrap()
}

fn differences(only: impl Fn(&str) -> bool) -> Vec<String> {
    let fixture = fixture();
    assert!(fixture["chrome"].as_str().unwrap().starts_with("156."));
    let fonts = fixture_fonts();
    let tree = parse_html_tree(fixture["html"].as_str().unwrap());
    let mut diags = Diagnostics::default();
    let got = layout_boxes_tree(
        &tree,
        "",
        1280.0,
        800.0,
        &fonts,
        &HtmlOptions::default(),
        &mut diags,
    );
    let mut failures = Vec::new();
    for want in fixture["boxes"].as_array().unwrap() {
        let cid = want["cid"].as_str().unwrap();
        if !only(cid) {
            continue;
        }
        let Some(b) = got.iter().find(|b| b.cid == cid) else {
            failures.push(format!("no box for data-cid={cid}"));
            continue;
        };
        for (field, value) in [
            ("x", b.x),
            ("y", b.y),
            ("width", b.width),
            ("height", b.height),
        ] {
            let chrome = want[field].as_f64().unwrap() as f32;
            if (value - chrome).abs() > 0.01 {
                failures.push(format!("{cid}.{field} = {value}, Chrome {chrome}"));
            }
        }
    }
    failures
}

fn assert_like_chrome(failures: &[String]) {
    assert!(
        failures.is_empty(),
        "{} differences from Chrome 156:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn button_select_meter_and_progress_are_border_box_like_chrome_156() {
    assert_like_chrome(&differences(|cid| {
        matches!(
            cid,
            "button" | "select" | "select-multiple" | "meter" | "progress"
        )
    }));
}

#[test]
fn button_like_checkable_color_and_search_inputs_are_border_box_like_chrome_156() {
    assert_like_chrome(&differences(|cid| {
        matches!(
            cid,
            "input-button"
                | "input-submit"
                | "input-reset"
                | "input-color"
                | "input-checkbox"
                | "input-radio"
                | "input-search"
        )
    }));
}

#[test]
fn every_form_control_box_matches_chrome_156_including_the_content_box_ones() {
    assert_like_chrome(&differences(|_| true));
}
