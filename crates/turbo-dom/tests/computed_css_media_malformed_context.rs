#![cfg(feature = "computed-style-grammar")]
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use turbo_dom::rtdom::{
    computed_css::{
        conditions::{media_matches, ConditionEnvironment},
        grammar::{
            parse_media_condition, parse_media_condition_context, MediaCondition, MediaType,
            Qualifier,
        },
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
fn every_affected_strict_syntax_error_is_a_chrome159_nonmatching_context_query() {
    let control: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/css_media_malformed_chrome159.json")).unwrap();
    assert_eq!(control["version"], "159.0.8154.6");
    assert_eq!(
        control["sourceSha256"],
        format!(
            "{:x}",
            Sha256::digest(include_bytes!("fixtures/css-media-malformed-oracle.mjs"))
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
    for row in control["results"].as_array().unwrap() {
        let source = row["source"].as_str().unwrap();
        assert_eq!(row["matches"], false);
        assert!(parse_media_condition(source).is_err(), "strict: {source}");
        let parsed = parse_media_condition_context(source, limits()).unwrap();
        let index = usize::from(source.starts_with("future(foo),"));
        assert_eq!(parsed.media_queries.len(), index + 1, "{source}");
        let invalid = &parsed.media_queries[index];
        assert_eq!(invalid.qualifier, Some(Qualifier::Not), "{source}");
        assert_eq!(invalid.media_type, MediaType::All, "{source}");
        assert!(invalid.condition.is_none(), "{source}");
        if index == 1 {
            assert!(matches!(
                parsed.media_queries[0].condition,
                Some(MediaCondition::Unknown(_))
            ));
        }
        assert_eq!(
            media_matches(
                &CssText::from_units(source.encode_utf16().collect()),
                &environment,
                limits()
            ),
            Ok(false),
            "context: {source}"
        );
    }
}
