//! #518/#519 typed calc arithmetic and correct percentage bases.
use serde_json::Value;
use sha2::{Digest, Sha256};
use turbo_dom_layout::{
    layout_boxes_tree, parse_html_tree, Diagnostics, FontRegistry, HtmlOptions,
};

const CASES: &str = include_str!("fixtures/flex_calc_cases.json");
const CONTROL: &str = include_str!("fixtures/flex_calc_chrome159.json");

fn marked_tree(html: &str) -> turbo_dom::Tree {
    let mut tree = parse_html_tree(html);
    for selector in [".grid", ".item", ".card", ".badge", ".body"] {
        let nodes = tree.query_selector_all(selector).to_vec();
        for (index, node) in nodes.into_iter().enumerate() {
            tree.set_attribute(node, "data-cid", &format!("{selector}:{index}"));
        }
    }
    tree
}

#[test]
fn flex_calc_matches_chromium_159() {
    let cases: Value = serde_json::from_str(CASES).unwrap();
    let control: Value = serde_json::from_str(CONTROL).unwrap();
    assert_eq!(control["browser"], "159.0.8154.6");
    assert_eq!(control["offline"], true);
    for (name, bytes) in [
        ("casesSha256", CASES.as_bytes()),
        (
            "collectorSha256",
            include_bytes!("../../../scripts/flex-calc-chrome-control.mjs").as_slice(),
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
        let html = match input["fixture"].as_str().unwrap() {
            "mui-width-additive" => {
                include_str!("../../../tools/component-parity/fixtures/mui-width-additive.html")
            }
            "mui-width-expression" => {
                include_str!("../../../tools/component-parity/fixtures/mui-width-expression.html")
            }
            _ => panic!("unknown fixture"),
        };
        assert_eq!(
            control["fixtureHashes"][input["fixture"].as_str().unwrap()],
            format!("{:x}", Sha256::digest(html.as_bytes()))
        );
        let boxes = layout_boxes_tree(
            &marked_tree(html),
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
