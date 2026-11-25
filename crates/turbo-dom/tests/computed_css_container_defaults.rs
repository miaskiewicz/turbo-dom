#![cfg(feature = "computed-style-grammar")]
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use turbo_dom::rtdom::computed_css::{
    computed_values::{compute_values, ComputedValues, ValueContext, ValueLimits},
    properties::{self, InitialContext},
    property_data, ComputationLimits, CssText,
};
fn text(value: &str) -> CssText {
    CssText::from_units(value.encode_utf16().collect())
}
fn context() -> ValueContext<'static> {
    ValueContext {
        initial: InitialContext {
            color: "rgb(0, 0, 0)",
            font_family: "serif",
            font_size: "16px",
        },
        unit_bases: &[("rem", 16.0)],
        font_size_unit_bases: &[],
        is_root: false,
    }
}
fn limits() -> ValueLimits {
    ValueLimits {
        computation: ComputationLimits {
            max_nesting: 32,
            max_output_units: 4096,
            max_total_units: 8192,
        },
        retained_units: 4096,
    }
}
fn row_values(value: &serde_json::Value) -> BTreeMap<String, Option<CssText>> {
    value
        .as_object()
        .unwrap()
        .iter()
        .map(|(name, value)| (name.clone(), Some(text(value.as_str().unwrap()))))
        .collect()
}
fn read(values: &ComputedValues, name: &str) -> String {
    let fallback;
    let value = if let Some(value) = values.values.get(name) {
        value
    } else {
        fallback = properties::initial(name, &context().initial).unwrap();
        &fallback
    };
    String::from_utf16(value.as_units()).unwrap()
}
#[test]
fn canonical_context_initials_and_wide_keywords_match_chrome159() {
    let control: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/css_container_defaults_chrome159.json"
    ))
    .unwrap();
    assert_eq!(control["version"], "159.0.8154.6");
    assert_eq!(
        control["sourceSha256"],
        format!(
            "{:x}",
            Sha256::digest(include_bytes!("fixtures/css-container-defaults-oracle.mjs"))
        )
    );
    let mut computed = BTreeMap::<String, ComputedValues>::new();
    for row in control["results"].as_array().unwrap() {
        let parent = row["parent"].as_str().map(|id| &computed[id]);
        let values = compute_values(
            parent,
            &row_values(&row["declarations"]),
            &context(),
            limits(),
        )
        .unwrap();
        for (name, expected) in row["computed"].as_object().unwrap() {
            assert_eq!(
                read(&values, name),
                expected.as_str().unwrap(),
                "{}:{name}",
                row["id"]
            );
        }
        computed.insert(row["id"].as_str().unwrap().to_owned(), values);
    }
}
#[test]
fn new_context_initials_do_not_expand_native_legacy_default_metadata() {
    assert!(property_data::lookup("container-name").is_none());
    assert!(property_data::lookup("container-type").is_none());
    assert!(property_data::lookup("writing-mode").unwrap().inherited);
    assert!(!properties::inherited("container-name"));
    assert!(!properties::inherited("container-type"));
}
