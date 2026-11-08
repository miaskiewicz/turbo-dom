// Chrome 156 multi-column geometry harness (#398 PR-B + PR-C1 + PR-C2 + PR-E).
//
// Loads the oracle captured by
// `crates/turbo-dom-layout/scripts/multicol-chrome-control.mjs` and runs
// turbo-dom-layout over each case's `html` to check that the CSS multi-column
// pass produces Chrome 156's container and item border boxes. PR-C1 activated
// the three cleanest equal-width sequential-fill cases (`basic-2col-equal`,
// `basic-3col-with-gap`, `column-width-driven`); PR-C2 activated the other
// five (`columns-shorthand`, `mixed-child-heights`, `overflow-at-column-height`,
// `nested-multicol`, `floats-inside-column`) by adding `column-fill: balance`,
// column fragmentation, overflow into extra strips beyond `used_count`, and
// a union-geometry pass for fragmented children. PR-E activates six more
// (`span-all-simple`, `span-all-balance`, `break-before-column`,
// `break-after-column`, `break-inside-avoid`, `break-inside-avoid-fits-anyway`)
// for `column-span: all` and the `break-{before,after,inside}` flow-honouring
// rules.

use serde_json::Value;
use turbo_dom_layout::{layout_boxes, test_support::fixture_fonts, Diagnostics};

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
    "span-all-simple",
    "span-all-balance",
    "break-before-column",
    "break-after-column",
    "break-inside-avoid",
    "break-inside-avoid-fits-anyway",
];

// Chrome rounds multi-column widths and gap positions on its 1/64 px LayoutUnit
// grid (≈0.015625 px). A naive float formula can differ by up to a handful of
// 1/64 px (≈0.08 px) from Chrome's reported value, so 0.1 px suffices for every
// case. Earlier revisions used 0.5 px "with room to spare", but 0.5 px is six
// times the actual 1/64 px grid budget and would silently hide a quarter-pixel
// column-origin bug.
const TOL: f64 = 0.1;

/// Cases PR-C1 asserts geometry for. The other five are in
/// `pr_c2_ignored_but_fixture_shape_checked` below.
const PR_C1_CASES: &[&str] = &[
    "basic-2col-equal",
    "basic-3col-with-gap",
    "column-width-driven",
];

fn fixture() -> Value {
    serde_json::from_str(FIXTURE).expect("multicol_chrome156.json parses")
}

fn find_case<'a>(cases: &'a [Value], id: &str) -> &'a Value {
    cases
        .iter()
        .find(|c| c["id"].as_str() == Some(id))
        .expect("case present in fixture")
}

fn boxes_for(html: &str) -> Vec<turbo_dom_layout::CidBox> {
    let fonts = fixture_fonts();
    let mut diags = Diagnostics::default();
    layout_boxes(html, "", 1280.0, 800.0, &fonts, &mut diags).expect("layout_boxes")
}

fn assert_matches_chrome(id: &str, case: &Value) {
    let html = case["html"].as_str().expect("html");
    let got = boxes_for(html);
    let mut failures: Vec<String> = Vec::new();
    // Container border box.
    let want_c = &case["container"];
    let got_c = got
        .iter()
        .find(|b| b.cid == "c")
        .unwrap_or_else(|| panic!("{id}: container cid=c missing from layout_boxes output"));
    for (field, got_v) in [
        ("x", got_c.x),
        ("y", got_c.y),
        ("width", got_c.width),
        ("height", got_c.height),
    ] {
        let want_v = want_c[field].as_f64().expect("container field");
        if (f64::from(got_v) - want_v).abs() > TOL {
            failures.push(format!(
                "{id}: container.{field} = {got_v}, Chrome {want_v}"
            ));
        }
    }
    // Each flowed item's border box.
    for item in case["items"].as_array().expect("items") {
        let cid = item["cid"].as_str().expect("cid");
        let Some(b) = got.iter().find(|b| b.cid == cid) else {
            failures.push(format!(
                "{id}: item cid={cid} missing from layout_boxes output"
            ));
            continue;
        };
        for (field, got_v) in [
            ("x", b.x),
            ("y", b.y),
            ("width", b.width),
            ("height", b.height),
        ] {
            let want_v = item[field].as_f64().expect("item field");
            if (f64::from(got_v) - want_v).abs() > TOL {
                failures.push(format!("{id}: {cid}.{field} = {got_v}, Chrome {want_v}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn multicol_fixture_loads_and_covers_the_eight_cases() {
    let fixture = fixture();
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
        assert_eq!(
            container["cid"].as_str(),
            Some("c"),
            "case {id}: container cid"
        );
        assert!(
            !case["html"].as_str().unwrap().is_empty(),
            "case {id}: html present",
        );
        let items = case["items"].as_array().expect("items array");
        assert!(!items.is_empty(), "case {id}: at least one flowed item");
        let columns = case["columns"].as_array().expect("columns array");
        assert!(
            !columns.is_empty(),
            "case {id}: at least one derived column strip"
        );
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
}

// --------------------------------------------------------------------------
// PR-C1: equal-width sequential-fill, 3 clean cases.
// --------------------------------------------------------------------------

#[test]
fn basic_2col_equal_matches_chrome_156() {
    let fixture = fixture();
    let case = find_case(fixture["cases"].as_array().unwrap(), "basic-2col-equal");
    assert_matches_chrome("basic-2col-equal", case);
}

#[test]
fn basic_3col_with_gap_matches_chrome_156() {
    let fixture = fixture();
    let case = find_case(fixture["cases"].as_array().unwrap(), "basic-3col-with-gap");
    assert_matches_chrome("basic-3col-with-gap", case);
}

#[test]
fn column_width_driven_matches_chrome_156() {
    let fixture = fixture();
    let case = find_case(fixture["cases"].as_array().unwrap(), "column-width-driven");
    assert_matches_chrome("column-width-driven", case);
}

/// Covers `PR_C1_CASES` so a stray fifth case cannot sneak in without updating the list.
#[test]
fn pr_c1_activates_three_cases() {
    assert_eq!(
        PR_C1_CASES,
        &[
            "basic-2col-equal",
            "basic-3col-with-gap",
            "column-width-driven"
        ],
    );
}

// --------------------------------------------------------------------------
// PR-C2: column-fill: balance + column fragmentation + overflow strips.
// --------------------------------------------------------------------------

#[test]
fn columns_shorthand_matches_chrome_156() {
    // PR-C1's docstring tagged i2 / i5 as a "column-span capture anomaly" —
    // the capture is actually `getBoundingClientRect()` returning the union of
    // two column fragments a block produces when `column-fill: balance`
    // (default) cuts it at the column boundary, which PR-C2's fragmenter
    // models directly. The remaining fixture rounding (Chrome's 1/64 px
    // `LayoutUnit` grid on the balanced column height, `content_h /
    // used_count`) stays within the harness's 1/64 px (≈0.1 px) tolerance.
    let fixture = fixture();
    let case = find_case(fixture["cases"].as_array().unwrap(), "columns-shorthand");
    assert_matches_chrome("columns-shorthand", case);
}

#[test]
fn mixed_child_heights_matches_chrome_156() {
    let fixture = fixture();
    let case = find_case(fixture["cases"].as_array().unwrap(), "mixed-child-heights");
    assert_matches_chrome("mixed-child-heights", case);
}

#[test]
fn overflow_at_column_height_matches_chrome_156() {
    let fixture = fixture();
    let case = find_case(
        fixture["cases"].as_array().unwrap(),
        "overflow-at-column-height",
    );
    assert_matches_chrome("overflow-at-column-height", case);
}

#[test]
fn nested_multicol_matches_chrome_156() {
    let fixture = fixture();
    let case = find_case(fixture["cases"].as_array().unwrap(), "nested-multicol");
    assert_matches_chrome("nested-multicol", case);
}

#[test]
fn floats_inside_column_matches_chrome_156() {
    let fixture = fixture();
    let case = find_case(fixture["cases"].as_array().unwrap(), "floats-inside-column");
    assert_matches_chrome("floats-inside-column", case);
}

// --------------------------------------------------------------------------
// PR-E: `column-span: all` + `break-{before,after,inside}` flow honouring.
// --------------------------------------------------------------------------

#[test]
fn span_all_simple_matches_chrome_156() {
    let fixture = fixture();
    let case = find_case(fixture["cases"].as_array().unwrap(), "span-all-simple");
    assert_matches_chrome("span-all-simple", case);
}

#[test]
fn span_all_balance_matches_chrome_156() {
    let fixture = fixture();
    let case = find_case(fixture["cases"].as_array().unwrap(), "span-all-balance");
    assert_matches_chrome("span-all-balance", case);
}

#[test]
fn break_before_column_matches_chrome_156() {
    let fixture = fixture();
    let case = find_case(fixture["cases"].as_array().unwrap(), "break-before-column");
    assert_matches_chrome("break-before-column", case);
}

#[test]
fn break_after_column_matches_chrome_156() {
    let fixture = fixture();
    let case = find_case(fixture["cases"].as_array().unwrap(), "break-after-column");
    assert_matches_chrome("break-after-column", case);
}

#[test]
fn break_inside_avoid_matches_chrome_156() {
    let fixture = fixture();
    let case = find_case(fixture["cases"].as_array().unwrap(), "break-inside-avoid");
    assert_matches_chrome("break-inside-avoid", case);
}

#[test]
fn break_inside_avoid_fits_anyway_matches_chrome_156() {
    let fixture = fixture();
    let case = find_case(
        fixture["cases"].as_array().unwrap(),
        "break-inside-avoid-fits-anyway",
    );
    assert_matches_chrome("break-inside-avoid-fits-anyway", case);
}
