//! Percentage edges resolve against the flex containing block or grid area.
use serde_json::Value;
use sha2::{Digest, Sha256};
use turbo_dom_layout::{
    layout_boxes_tree, parse_html_tree, with_fractional_css_geometry, Diagnostics, FontRegistry,
    HtmlOptions,
};

const CASES: &str = include_str!("fixtures/percentage_frame_cases.json");
const CONTROL: &str = include_str!("fixtures/percentage_frame_chrome159.json");

#[test]
fn percentage_frames_match_chromium_159() {
    let cases: Value = serde_json::from_str(CASES).unwrap();
    let control: Value = serde_json::from_str(CONTROL).unwrap();
    assert_eq!(control["browser"], "159.0.8154.6");
    assert_eq!(control["offline"], true);
    assert_eq!(
        control["casesSha256"],
        format!("{:x}", Sha256::digest(CASES.as_bytes()))
    );
    assert_eq!(
        control["collectorSha256"],
        format!(
            "{:x}",
            Sha256::digest(include_bytes!(
                "../../../scripts/percentage-frame-chrome-control.mjs"
            ))
        )
    );
    let rows = control["rows"].as_array().unwrap();
    assert_eq!(cases.as_array().unwrap().len(), rows.len());
    let fonts = FontRegistry::new();
    let mut failures = Vec::new();
    for (input, expected) in cases.as_array().unwrap().iter().zip(rows) {
        assert_eq!(input["id"], expected["id"]);
        let mut tree = parse_html_tree(input["html"].as_str().unwrap());
        for selector in [".grid", ".item", ".inner", ".next"] {
            let node = tree.query_selector_all(selector)[0];
            tree.set_attribute(node, "data-cid", selector);
        }
        let boxes = with_fractional_css_geometry(|| {
            layout_boxes_tree(
                &tree,
                "",
                600.0,
                1000.0,
                &fonts,
                &HtmlOptions::default(),
                &mut Diagnostics::default(),
            )
        });
        for wanted in expected["boxes"].as_array().unwrap() {
            let key = wanted["key"].as_str().unwrap();
            let actual = boxes.iter().find(|item| item.cid == key).unwrap();
            for (axis, value) in [actual.x, actual.y, actual.width, actual.height]
                .into_iter()
                .enumerate()
            {
                let expected = wanted["rect"][axis].as_f64().unwrap();
                if (f64::from(value) - expected).abs() > 0.05 {
                    failures.push(format!(
                        "{} {key} axis{axis}: native {value}, Chromium {expected}",
                        input["id"]
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
