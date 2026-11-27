//! Generated defaults share one canonical source without materializing styles.
use serde_json::Value;
use sha2::{Digest, Sha256};
use turbo_dom::rtdom::computed_css::property_data;

const SOURCE: &str = include_str!("../src/rtdom/computed_css/property_data.json");

#[test]
fn every_generated_property_row_matches_the_canonical_source() {
    let data: Value = serde_json::from_str(SOURCE).unwrap();
    let expected = data["rows"].as_array().unwrap();
    assert_eq!(
        property_data::SOURCE_SHA256,
        format!("{:x}", Sha256::digest(SOURCE))
    );
    assert_eq!(property_data::rows().len(), expected.len());
    assert_eq!(expected.len(), 105);
    for row in expected {
        let name = row["name"].as_str().unwrap();
        let entry = property_data::lookup(name).unwrap();
        assert_eq!(entry.initial, row["initial"].as_str().unwrap(), "{name}");
        assert_eq!(
            entry.inherited,
            row["inherited"].as_bool().unwrap(),
            "{name}"
        );
    }
    assert!(property_data::rows()
        .windows(2)
        .all(|pair| pair[0].0 < pair[1].0));
}

#[test]
fn symbolic_context_values_and_unmatched_properties_stay_explicit() {
    assert_eq!(
        property_data::lookup("word-spacing").unwrap().initial,
        "0px"
    );
    assert_eq!(
        property_data::lookup("outline-color").unwrap().initial,
        "currentcolor"
    );
    assert!(property_data::lookup("caret-color").unwrap().inherited);
    for name in [
        "width",
        "font-size",
        "color",
        "unknown-property",
        "WORD-SPACING",
        "__proto__",
    ] {
        assert_eq!(property_data::lookup(name), None, "{name}");
    }
}
