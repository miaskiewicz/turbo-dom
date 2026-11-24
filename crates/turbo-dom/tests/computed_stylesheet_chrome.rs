//! Shared syntax/selector behavior against an explicitly frozen source-hashed oracle.
use serde_json::Value;
use sha2::{Digest, Sha256};
use turbo_dom::rtdom::{
    cascade::{computed_style, get_property_value},
    query::{match_for_cascade, parse_for_cascade_units, LiveNavigator},
};
use turbo_dom::Tree;

const PROBE: &str = include_str!("fixtures/computed-stylesheet-probe.mjs");
const CONTROL: &str = include_str!("fixtures/computed_stylesheet_chrome159.json");
fn units(value: &Value) -> Vec<u16> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|unit| u16::try_from(unit.as_u64().unwrap()).unwrap())
        .collect()
}

#[test]
fn stylesheet_tokens_and_unit_selectors_match_chrome159() {
    let control: Value = serde_json::from_str(CONTROL).unwrap();
    assert_eq!(control["browser"], "159.0.8154.6");
    assert_eq!(
        control["probeSha256"],
        format!("{:x}", Sha256::digest(PROBE))
    );
    for row in control["result"]["syntax"].as_array().unwrap() {
        let color = row["color"].as_str().unwrap();
        let css = row["css"].as_str().unwrap();
        let tree = Tree::parse(&format!("<style>{css}</style><div id=raw></div>"));
        let target = tree.query_selector("#raw").unwrap();
        let style = computed_style(&tree, target);
        assert_eq!(
            get_property_value(&style, "--token"),
            row["token"].as_str().unwrap()
        );
        assert_eq!(get_property_value(&style, "color"), color);
    }
    let mut tree = Tree::parse("<div id=raw></div><div id=repaired></div><div id=marker></div>");
    let records = [
        ("raw", vec![0xd800]),
        ("repaired", vec![0xfffd]),
        ("marker", vec![0xdb80, 0xdc00, 0xdb82, 0xdc00]),
    ];
    for (id, value) in &records {
        let node = tree.query_selector(&format!("#{id}")).unwrap();
        tree.set_attribute_utf16(node, "data-x", value);
        let mut class = vec![120];
        class.extend(value);
        tree.set_attribute_utf16(node, "class", &class);
    }
    for row in control["result"]["selectors"].as_array().unwrap() {
        let selectors = parse_for_cascade_units(&units(&row["selector"])).unwrap();
        let actual: Vec<&str> = records
            .iter()
            .filter_map(|(id, _)| {
                let node = tree.query_selector(&format!("#{id}")).unwrap();
                match_for_cascade(&LiveNavigator(&tree), node, &selectors.selectors()[0])
                    .then_some(*id)
            })
            .collect();
        let expected: Vec<&str> = row["query"]
            .as_array()
            .unwrap()
            .iter()
            .map(|id| id.as_str().unwrap())
            .collect();
        assert_eq!(actual, expected, "{}", row["id"]);
    }
}
