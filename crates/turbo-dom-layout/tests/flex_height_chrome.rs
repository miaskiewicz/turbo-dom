//! #520: descendants use the content height, without hiding incorrect overflow.
use serde_json::Value;
use sha2::{Digest, Sha256};
use turbo_dom_layout::{
    layout_boxes_tree, parse_html_tree, Diagnostics, FontRegistry, HtmlOptions,
};

const HTML: &str =
    include_str!("../../../tools/component-parity/fixtures/nested-flex-padding.html");
const CASES: &str = include_str!("fixtures/flex_height_cases.json");
const CONTROL: &str = include_str!("fixtures/flex_height_chrome159.json");

fn marked_tree() -> turbo_dom::Tree {
    let mut tree = parse_html_tree(HTML);
    for selector in [".grid", ".item", ".card", ".badge", ".body"] {
        let nodes = tree.query_selector_all(selector).to_vec();
        for (index, node) in nodes.into_iter().enumerate() {
            tree.set_attribute(node, "data-cid", &format!("{selector}:{index}"));
        }
    }
    tree
}

#[test]
fn nested_flex_content_heights_match_chromium_159() {
    let cases: Value = serde_json::from_str(CASES).unwrap();
    let control: Value = serde_json::from_str(CONTROL).unwrap();
    assert_eq!(control["browser"], "159.0.8154.6");
    assert_eq!(control["offline"], true);
    for (name, bytes) in [
        ("fixtureSha256", HTML.as_bytes()),
        ("casesSha256", CASES.as_bytes()),
        (
            "collectorSha256",
            include_bytes!("../../../scripts/flex-height-chrome-control.mjs").as_slice(),
        ),
    ] {
        assert_eq!(control[name], format!("{:x}", Sha256::digest(bytes)));
    }
    let rows = control["rows"].as_array().unwrap();
    assert_eq!(cases.as_array().unwrap().len(), rows.len());
    let fonts = FontRegistry::new();
    for (input, expected) in cases.as_array().unwrap().iter().zip(rows) {
        assert_eq!(input["id"], expected["id"]);
        assert_eq!(input["css"], expected["css"]);
        let boxes = layout_boxes_tree(
            &marked_tree(),
            input["css"].as_str().unwrap(),
            input["width"].as_f64().unwrap() as f32,
            1000.0,
            &fonts,
            &HtmlOptions::default(),
            &mut Diagnostics::default(),
        );
        for wanted in expected["boxes"].as_array().unwrap() {
            let key = wanted["key"].as_str().unwrap();
            let actual = boxes.iter().find(|item| item.cid == key).unwrap();
            for (axis, value) in [actual.x, actual.y, actual.width, actual.height]
                .into_iter()
                .enumerate()
            {
                let wanted = wanted["rect"][axis].as_f64().unwrap();
                assert!(
                    (f64::from(value) - wanted).abs() <= 1.0,
                    "{} {key} axis{axis}: native {value}, Chromium {wanted}",
                    input["id"]
                );
            }
        }
    }
}
