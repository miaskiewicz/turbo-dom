use super::scroll::*;
use crate::layout_snapshot::LayoutSnapshot;
use crate::live::{source_id, testing};
use serde_json::Value;
use sha2::{Digest, Sha256};

const PROBE: &str = include_str!("../../tests/fixtures/body_overflow_propagation_probe.js");
const CONTROL: &str = include_str!("../../tests/fixtures/body_overflow_propagation_chrome159.json");

fn verify_case(case: &Value) {
    let name = case["name"].as_str().unwrap();
    let doctype = if case["quirks"] == true {
        ""
    } else {
        "<!doctype html>"
    };
    let html = format!(
        "{doctype}<body><div style='height:100px'></div><button id='target'>Target</button>"
    );
    let css = format!("html{{scrollbar-width:none}}body{{margin:8px;height:40px;overflow:hidden;scrollbar-width:none}}button{{display:block;height:30px;padding:0;border:0}}{}", case["css"].as_str().unwrap());
    let (tree, snapshot, styles) = testing::layout_with_fonts(
        &html,
        &css,
        (400.0, 300.0),
        Some(1.0),
        |_| {},
        &crate::test_support::fixture_fonts(),
    );
    let body = source_id(testing::DOCUMENT, tree.query_selector("body").unwrap());
    let want = &case["read"];
    let own = want["bodyScroll"] == 90;
    if want["bodyHeight"] == 40 {
        assert_eq!(propagated_body(&styles, body), !own, "{name}: propagation");
        assert_eq!(
            element_scroller(&styles, body),
            own,
            "{name}: own scrollport"
        );
    }
    let metrics = scroll_metrics(&snapshot, &styles, body).unwrap();
    assert_eq!(
        i64::from(metrics.height),
        want["bodyScrollHeight"].as_i64().unwrap(),
        "{name}: extent"
    );
    if own {
        assert_eq!(metrics.max_y, 90.0, "{name}: range");
        assert_eq!(
            expose(100.0, 30.0, 0.0, 40.0, alignment(3, 0).unwrap()),
            90.0,
            "{name}: nearest exposure"
        );
    }
    verify_translation(case, &tree, &snapshot, &styles, body);
}

fn verify_translation(
    case: &Value,
    tree: &turbo_dom::Tree,
    snapshot: &LayoutSnapshot,
    styles: &SourceStyles,
    body: crate::layout_snapshot::SourceId,
) {
    let target = source_id(testing::DOCUMENT, tree.get_element_by_id("target").unwrap());
    let offset = case["read"]["bodyScroll"].as_f64().unwrap();
    let offsets = std::collections::BTreeMap::from([(body, [0.0, offset])]);
    let shift = scroll_translation(snapshot, styles, target, &offsets).unwrap();
    assert_eq!(
        shift,
        [0.0, offset as f32],
        "{}: body translation",
        case["name"]
    );
}

#[test]
fn body_propagation_matches_independent_chrome159_controls() {
    let control: Value = serde_json::from_str(CONTROL).unwrap();
    assert!(control["browser"].as_str().unwrap().starts_with("159."));
    assert_eq!(
        control["probeSha256"],
        format!("{:x}", Sha256::digest(PROBE.as_bytes()))
    );
    assert_eq!(control["result"].as_array().unwrap().len(), 57);
    for case in control["result"].as_array().unwrap() {
        verify_case(case);
    }
}

#[test]
fn propagation_requires_a_direct_html_document_parent() {
    let (tree, _, mut styles) = testing::layout_with_fonts(
        "<!doctype html><body style='overflow:hidden'></body>",
        "",
        (400.0, 300.0),
        Some(1.0),
        |_| {},
        &crate::test_support::fixture_fonts(),
    );
    let body = source_id(testing::DOCUMENT, tree.query_selector("body").unwrap());
    let root = styles[&body].parent.unwrap();
    assert!(propagated_body(&styles, body));
    styles.get_mut(&root).unwrap().tag = "svg".into();
    assert!(!propagated_body(&styles, body));
    styles.get_mut(&root).unwrap().tag = "HTML".into();
    assert!(propagated_body(&styles, body));
    styles.get_mut(&body).unwrap().parent = None;
    assert!(!propagated_body(&styles, body));
}
