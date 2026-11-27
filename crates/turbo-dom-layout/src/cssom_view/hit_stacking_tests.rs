use super::{hit, metrics};
use crate::live::{source_id, testing};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

fn top_hit(subject_style: &str, expected: &str) {
    top_hit_rects(subject_style, expected, &[[0.0, 0.0, 100.0, 100.0]; 3]);
}

fn top_hit_rects(subject_style: &str, expected: &str, rects: &[[f32; 4]]) {
    top_hit_css(subject_style, expected, rects, "");
}

fn top_hit_css(subject_style: &str, expected: &str, rects: &[[f32; 4]], extra_css: &str) {
    let html =
        "<!doctype html><body><div id='subject'><div id='inner'></div></div><div id='cover'></div>";
    let css = format!(
        "body{{margin:0}}#subject{{position:relative;width:100px;height:100px;{subject_style}}}#inner{{position:absolute;inset:0;z-index:100}}#cover{{position:absolute;left:0;top:0;width:100px;height:100px;z-index:1}}{extra_css}"
    );
    let (tree, snapshot, styles) = testing::layout_with_fonts(
        html,
        &css,
        (300.0, 150.0),
        Some(1.0),
        |_| {},
        &crate::test_support::fixture_fonts(),
    );
    for (id, expected_rect) in ["subject", "inner", "cover"].into_iter().zip(rects) {
        let source = source_id(testing::DOCUMENT, tree.get_element_by_id(id).unwrap());
        let rect = metrics::element_geometry(&snapshot, source).unwrap().bounds;
        assert_eq!([rect.x, rect.y, rect.width, rect.height], *expected_rect);
    }
    let expected = source_id(testing::DOCUMENT, tree.get_element_by_id(expected).unwrap());
    let hits = hit::elements_at_point(&snapshot, &styles, &BTreeMap::new(), [50.0, 50.0]).unwrap();
    assert_eq!(hits.first(), Some(&expected), "{subject_style}");
}

#[test]
fn non_positioned_flex_and_grid_items_keep_their_own_z_context() {
    for style in ["display:flex", "display:grid"] {
        top_hit_css(
            style,
            "inner",
            &[[0.0, 0.0, 100.0, 100.0]; 3],
            "#inner{position:static;width:100px;height:100px}",
        );
    }
}

#[test]
fn positioned_auto_ancestor_does_not_trap_positive_z_descendant() {
    top_hit("", "inner");
}

#[test]
fn actual_stacking_context_keeps_positive_z_descendant_inside() {
    for style in [
        "z-index:0",
        "opacity:.5",
        "transform:translateX(0px)",
        "position:fixed",
        "position:sticky",
        "isolation:isolate",
        "contain:paint",
        "will-change:opacity",
    ] {
        top_hit(style, "cover");
    }
}

#[test]
fn ordinary_static_z_and_opaque_value_do_not_create_contexts() {
    for style in ["opacity:1", "transform:none"] {
        top_hit(style, "inner");
    }
}

#[test]
fn public_native_hit_matches_source_pinned_chrome159_static_inverses() {
    let probe = include_str!("../../tests/fixtures/hit_stacking_probe.js");
    let control: serde_json::Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/hit_stacking_chrome159.json"
    ))
    .unwrap();
    assert!(control["browser"].as_str().unwrap().starts_with("159."));
    assert_eq!(
        control["probeSha256"],
        format!("{:x}", Sha256::digest(probe.as_bytes()))
    );
    let cases = control["result"].as_array().unwrap();
    assert_eq!(cases.len(), 16);
    for case in cases.iter().filter(|case| case["effect"].is_null()) {
        let rects: Vec<[f32; 4]> = serde_json::from_value(case["rects"].clone()).unwrap();
        top_hit_rects(
            case["subjectStyle"].as_str().unwrap(),
            case["hit"].as_str().unwrap(),
            &rects,
        );
    }
}
