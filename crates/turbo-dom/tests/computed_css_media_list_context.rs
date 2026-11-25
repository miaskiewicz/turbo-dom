#![cfg(feature = "computed-style-grammar")]
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use turbo_dom::rtdom::{
    computed_css::{
        conditions::{media_matches, ConditionEnvironment},
        grammar::{
            parse_media_condition, parse_media_condition_context, ConditionError, MediaType,
            Qualifier,
        },
        ComputationLimits, ComputedCssError, CssText,
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
fn forgiving_lists_preserve_every_source_hashed_chrome159_case() {
    let control: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/css_media_list_chrome159.json")).unwrap();
    assert_eq!(control["version"], "159.0.8154.6");
    assert_eq!(
        control["sourceSha256"],
        format!(
            "{:x}",
            Sha256::digest(include_bytes!("fixtures/css-media-list-oracle.mjs"))
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
#[test]
fn invalid_queries_become_not_all_without_splitting_nested_or_escaped_commas() {
    let parsed = parse_media_condition_context("print and, screen", limits()).unwrap();
    assert_eq!(parsed.media_queries.len(), 2);
    assert_eq!(parsed.media_queries[0].qualifier, Some(Qualifier::Not));
    assert_eq!(parsed.media_queries[0].media_type, MediaType::All);
    assert_eq!(parsed.media_queries[1].media_type, MediaType::Screen);
    for query in ["print, (future(1,2))", "future\\,screen, print"] {
        assert_eq!(
            parse_media_condition_context(query, limits())
                .unwrap()
                .media_queries
                .len(),
            2
        );
    }
    assert_eq!(
        parse_media_condition_context("screen,,print", limits())
            .unwrap()
            .media_queries
            .len(),
        3
    );
    assert!(parse_media_condition("print and, screen").is_err());
}
#[test]
fn forgiving_syntax_never_recovers_resource_or_nesting_failure() {
    let mut policy = limits();
    policy.max_output_units = 2;
    assert_eq!(
        parse_media_condition_context("print and, screen", policy).unwrap_err(),
        ConditionError::Computation(ComputedCssError::ResourceLimit)
    );
    policy = limits();
    policy.max_nesting = 2;
    assert_eq!(
        parse_media_condition_context("screen, (future(f(f(1))))", policy).unwrap_err(),
        ConditionError::Computation(ComputedCssError::NestingLimit)
    );
}
