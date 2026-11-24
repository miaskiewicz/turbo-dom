use super::{hit, metrics, scroll};
use crate::live::{source_id, testing};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

const PROBE: &str = include_str!("../../tests/fixtures/hit_clip_ownership_probe.js");
const CONTROL: &str = include_str!("../../tests/fixtures/hit_clip_ownership_chrome159.json");

fn controls() -> Value {
    let data: Value = serde_json::from_str(CONTROL).unwrap();
    assert!(data["browser"].as_str().unwrap().starts_with("159."));
    assert_eq!(
        data["probeSha256"],
        format!("{:x}", Sha256::digest(PROBE.as_bytes()))
    );
    assert_eq!(data["result"].as_array().unwrap().len(), 35);
    data
}

fn verify(case: &Value) {
    let name = case["name"].as_str().unwrap();
    let html = format!("<!doctype html><body>{}", case["body"].as_str().unwrap());
    let (tree, snapshot, styles) = testing::layout_with_fonts(
        &html,
        case["css"].as_str().unwrap(),
        (1280.0, 800.0),
        Some(1.0),
        |_| {},
        &crate::test_support::fixture_fonts(),
    );
    let control = source_id(
        testing::DOCUMENT,
        tree.get_element_by_id("control").unwrap(),
    );
    let body = source_id(testing::DOCUMENT, tree.query_selector("body").unwrap());
    let scroll = case["read"]["bodyScroll"].as_f64().unwrap();
    let offsets = BTreeMap::from([(body, [0.0, scroll])]);
    let point = [
        case["point"][0].as_f64().unwrap(),
        case["point"][1].as_f64().unwrap(),
    ];
    if case["read"]["controlHit"] == true {
        let rect = metrics::element_geometry(&snapshot, control)
            .unwrap()
            .bounds;
        let shift = scroll::scroll_translation(&snapshot, &styles, control, &offsets).unwrap();
        let got = [
            rect.x - shift[0],
            rect.y - shift[1],
            rect.width,
            rect.height,
        ];
        let want = ["x", "y", "width", "height"]
            .map(|key| case["read"]["rect"][key].as_f64().unwrap() as f32);
        assert_eq!(got, want, "{name}: public geometry");
    }
    let hits = hit::elements_at_point(&snapshot, &styles, &offsets, point).unwrap();
    assert_eq!(
        hits.first() == Some(&control),
        case["read"]["controlHit"].as_bool().unwrap(),
        "{name}: public hit"
    );
}

#[test]
fn propagated_body_does_not_clip_public_hit() {
    let data = controls();
    verify(&data["result"][0]);
}
#[test]
fn fixed_descendant_escapes_ordinary_overflow_for_public_hit() {
    let data = controls();
    verify(&data["result"][4]);
}
#[test]
fn clip_ownership_matches_supported_chrome159_controls() {
    let data = controls();
    let cases = data["result"].as_array().unwrap();
    assert_eq!(
        cases
            .iter()
            .filter(|case| case["unsupported"] == true)
            .count(),
        1
    );
    for case in cases.iter().filter(|case| case["unsupported"] != true) {
        verify(case);
    }
}

#[test]
fn viewport_overflow_uses_the_viewport_instead_of_the_root_border_box() {
    let (tree, snapshot, styles) = testing::layout_with_fonts(
        "<!doctype html><body style='height:40px;overflow:hidden'></body>",
        "",
        (400.0, 300.0),
        Some(1.0),
        |_| {},
        &crate::test_support::fixture_fonts(),
    );
    let body = source_id(testing::DOCUMENT, tree.query_selector("body").unwrap());
    let root = styles[&body].parent.unwrap();
    let offsets = BTreeMap::new();
    for ancestor in [root, body] {
        assert!(!hit::clipped(&snapshot, &styles, &[ancestor], &offsets, [100.0, 200.0]).unwrap());
        assert!(hit::clipped(&snapshot, &styles, &[ancestor], &offsets, [-1.0, 200.0]).unwrap());
        assert!(hit::clipped(&snapshot, &styles, &[ancestor], &offsets, [400.0, 200.0]).unwrap());
    }
}

#[test]
fn stale_foreign_containment_provenance_is_an_error() {
    let (tree, snapshot, mut styles) = testing::layout_with_fonts(
        "<!doctype html><body></body>",
        "",
        (400.0, 300.0),
        Some(1.0),
        |_| {},
        &crate::test_support::fixture_fonts(),
    );
    let root = source_id(testing::DOCUMENT, tree.query_selector("html").unwrap());
    let foreign = crate::layout_snapshot::SourceId(u64::MAX - 7);
    styles.insert(
        foreign,
        scroll::SourceStyle {
            style: crate::ComputedStyle::from_pairs([
                ("display", "inline"),
                ("contain", "paint"),
                ("overflow", "clip"),
            ]),
            parent: Some(root),
            tag: "span".into(),
            zoom: 1.0,
        },
    );
    assert_eq!(
        hit::clipped(
            &snapshot,
            &styles,
            &[foreign],
            &BTreeMap::new(),
            [100.0, 200.0]
        )
        .unwrap_err(),
        "native hit containment source missing"
    );
}
