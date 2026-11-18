//! Flex row automatic minimums match Chromium.
use serde_json::Value;
use sha2::{Digest, Sha256};
use turbo_dom_layout::{
    layout_boxes_tree, parse_html_tree, with_fractional_css_geometry, Diagnostics, FontRegistry,
    HtmlOptions,
};

const CASES: &str = include_str!("fixtures/flex_row_min_cases.json");
const CONTROL: &str = include_str!("fixtures/flex_row_min_chrome159.json");

#[test]
fn row_automatic_minimums_match_chromium_159() {
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
                "../../../scripts/flex-row-min-chrome-control.mjs"
            ))
        )
    );
    let expected = control["rows"].as_array().unwrap();
    assert_eq!(cases.as_array().unwrap().len(), expected.len());
    let fonts = FontRegistry::new();
    let mut failures = Vec::new();
    for (case, expected) in cases.as_array().unwrap().iter().zip(expected) {
        let tree = parse_html_tree(case["html"].as_str().unwrap());
        let actual = with_fractional_css_geometry(|| {
            layout_boxes_tree(
                &tree,
                "",
                390.0,
                1000.0,
                &fonts,
                &HtmlOptions::default(),
                &mut Diagnostics::default(),
            )
        });
        for want in expected["boxes"].as_array().unwrap() {
            let key = want["key"].as_str().unwrap();
            let got = actual
                .iter()
                .find(|item| item.cid == key)
                .unwrap_or_else(|| panic!("{} missing {key}; native {actual:?}", case["id"]));
            for (axis, value) in [got.x, got.y, got.width, got.height]
                .into_iter()
                .enumerate()
            {
                let desired = want["rect"][axis].as_f64().unwrap();
                if (f64::from(value) - desired).abs() > 0.05 {
                    failures.push(format!(
                        "{} {key} axis{axis}: {value} vs Chromium {desired}",
                        case["id"]
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
