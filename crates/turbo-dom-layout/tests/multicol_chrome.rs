// Chrome 156 multi-column geometry harness (#398 PR-B).
//
// Loads the oracle captured by
// `crates/turbo-dom-layout/scripts/multicol-chrome-control.mjs` and asserts its
// shape. The actual layout comparison (running turbo-dom-layout's multicol pass
// against each case's recorded container + items + columns) lands in #398 PR-C,
// which replaces the TODO below with a real `assert_matches_chrome` call per case.
// Keeping the harness in place now means PR-C is a one-line activation.

use serde_json::Value;

const FIXTURE: &str = include_str!("fixtures/multicol_chrome156.json");

const EXPECTED_IDS: &[&str] = &[
    "basic-2col-equal",
    "basic-3col-with-gap",
    "column-width-driven",
    "columns-shorthand",
    "mixed-child-heights",
    "overflow-at-column-height",
    "nested-multicol",
    "floats-inside-column",
];

#[test]
fn multicol_fixture_loads_and_covers_the_eight_cases() {
    let fixture: Value = serde_json::from_str(FIXTURE).expect("multicol_chrome156.json parses");
    let generator = fixture["generator"].as_str().expect("generator string");
    assert!(
        generator.ends_with("multicol-chrome-control.mjs"),
        "fixture generator: {generator}",
    );
    let chrome = fixture["chrome"].as_str().expect("chrome version string");
    assert!(chrome.starts_with("156."), "Chrome version: {chrome}");
    let viewport = fixture["viewport"].as_array().expect("viewport array");
    assert_eq!(viewport.len(), 2);
    assert_eq!(viewport[0].as_u64(), Some(1280));
    assert_eq!(viewport[1].as_u64(), Some(800));
    let cases = fixture["cases"].as_array().expect("cases array");
    let ids: Vec<&str> = cases.iter().map(|c| c["id"].as_str().unwrap()).collect();
    assert_eq!(ids, EXPECTED_IDS);
    for case in cases {
        let id = case["id"].as_str().unwrap();
        let container = &case["container"];
        assert_eq!(container["cid"].as_str(), Some("c"), "case {id}: container cid");
        assert!(
            !case["html"].as_str().unwrap().is_empty(),
            "case {id}: html present",
        );
        let items = case["items"].as_array().expect("items array");
        assert!(!items.is_empty(), "case {id}: at least one flowed item");
        let columns = case["columns"].as_array().expect("columns array");
        assert!(!columns.is_empty(), "case {id}: at least one derived column strip");
        let client_width = container["clientWidth"].as_f64().expect("clientWidth");
        for col in columns {
            let x = col["x"].as_f64().expect("col x");
            let w = col["width"].as_f64().expect("col width");
            assert!(x >= 0.0, "case {id}: column x >= 0");
            assert!(w > 0.0, "case {id}: column width > 0");
            // Chrome may generate overflow column strips beyond the content box
            // (e.g. the `overflow-at-column-height` case); strips live at the
            // content-box top, so just bound their x by a generous limit.
            assert!(
                x < client_width * 4.0 + 1.0,
                "case {id}: column strip x ({x}) within sane bound",
            );
            let _ = client_width;
        }
    }
    // TODO(#398 PR-C): once the multicol layout pass lands, replace the shape
    // checks above with a call that runs turbo-dom-layout over each case's
    // `html` and asserts the resulting container/items/columns match Chrome's
    // numbers from this fixture.
}
