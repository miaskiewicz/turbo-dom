#![cfg(feature = "computed-style-grammar")]
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use turbo_dom::rtdom::{
    computed_css::{
        conditions::{media_matches, ConditionEnvironment},
        grammar::{parse_media_condition, MediaType, Qualifier},
        ComputationLimits, CssText,
    },
    css_shorthand::LengthContext,
};
fn limits() -> ComputationLimits {
    ComputationLimits {
        max_nesting: 64,
        max_output_units: 4096,
        max_total_units: 0,
    }
}
#[test]
fn malformed_eof_is_not_an_authored_empty_media_attribute() {
    let parsed = parse_media_condition("screen and").unwrap();
    assert_eq!(parsed.media_queries.len(), 1);
    let query = &parsed.media_queries[0];
    assert_eq!(query.qualifier, Some(Qualifier::Not));
    assert_eq!(query.media_type, MediaType::All);
    assert!(query.condition.is_none());
    for source in ["", "   ", "/**/"] {
        assert!(parse_media_condition(source)
            .unwrap()
            .media_queries
            .is_empty());
    }
}
#[test]
fn empty_and_malformed_eof_queries_follow_source_hashed_chrome159_truth() {
    let control: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/css_media_empty_chrome159.json")).unwrap();
    assert_eq!(control["version"], "159.0.8154.6");
    assert_eq!(
        control["sourceSha256"],
        format!(
            "{:x}",
            Sha256::digest(include_bytes!("fixtures/css-media-empty-oracle.mjs"))
        )
    );
    let features = BTreeMap::new();
    let selectors = BTreeMap::new();
    let lengths = LengthContext { unit_bases: &[] };
    let environment = ConditionEnvironment {
        media_type: "screen",
        features: &features,
        length_context: &lengths,
        selector_support: &selectors,
    };
    for source in [
        "",
        "   ",
        "/**/",
        "screen and",
        "screen and (width:)",
        "screen, print and",
        "print, screen and",
    ] {
        let row = control["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["source"] == source)
            .unwrap();
        let expected = row["matches"].as_bool().unwrap();
        assert_eq!(
            media_matches(
                &CssText::from_units(source.encode_utf16().collect()),
                &environment,
                limits()
            ),
            Ok(expected),
            "{source}"
        );
    }
}
