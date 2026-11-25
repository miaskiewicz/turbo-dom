//! Post-substitution CSS-wide controls confirmed by the independent Chrome159 oracle.
use std::collections::BTreeMap;
use turbo_dom::rtdom::computed_css::{
    compute_custom_properties, ComputationLimits, ComputedCssError, CssText,
};

fn text(source: &str) -> CssText {
    CssText::from_units(source.encode_utf16().collect())
}
fn limits() -> ComputationLimits {
    ComputationLimits {
        max_nesting: 16,
        max_output_units: 100,
        max_total_units: 500,
    }
}

#[test]
fn resolved_custom_wide_keywords_are_applied_before_storage() {
    let parent = BTreeMap::from([("--x".into(), text("blue"))]);
    for keyword in ["inherit", "unset", "initial"] {
        let own = BTreeMap::from([("--x".into(), text(&format!("var(--missing,{keyword})")))]);
        let result = compute_custom_properties(&parent, &own, limits()).unwrap();
        let expected = (keyword != "initial").then(|| text("blue"));
        assert_eq!(result.get("--x"), expected.as_ref(), "{keyword}");
    }
}

#[test]
fn dependencies_observe_the_requesting_propertys_inherited_value() {
    let parent = BTreeMap::from([("--x".into(), text("blue")), ("--y".into(), text("green"))]);
    let own = BTreeMap::from([
        ("--x".into(), text("var(--missing,inherit)")),
        ("--y".into(), text("var(--x)")),
    ]);
    let result = compute_custom_properties(&parent, &own, limits()).unwrap();
    assert_eq!(result.get("--x"), Some(&text("blue")));
    assert_eq!(result.get("--y"), Some(&text("blue")));
}

#[test]
fn resolved_rollback_is_not_retained_as_a_literal_token() {
    for keyword in ["revert", "revert-layer"] {
        let own = BTreeMap::from([("--x".into(), text(&format!("var(--missing,{keyword})")))]);
        assert!(
            matches!(
                compute_custom_properties(&BTreeMap::new(), &own, limits()),
                Err(ComputedCssError::RequiresCascade)
            ),
            "{keyword}"
        );
    }
}

#[test]
fn quoted_wide_word_is_an_ordinary_custom_token() {
    let own = BTreeMap::from([("--x".into(), text("var(--missing,'inherit')"))]);
    let result = compute_custom_properties(&BTreeMap::new(), &own, limits()).unwrap();
    assert_eq!(result.get("--x"), Some(&text("'inherit'")));
}

#[test]
fn detailed_rollback_names_the_actual_dependency_and_keyword() {
    use turbo_dom::rtdom::computed_css::{
        compute_custom_properties_detailed, CustomComputation, RollbackKeyword,
    };
    for (raw, expected) in [
        ("revert", RollbackKeyword::Revert),
        ("var(--missing,revert-layer)", RollbackKeyword::RevertLayer),
    ] {
        let mut own = BTreeMap::from([("--a".into(), text("var(--b)")), ("--b".into(), text(raw))]);
        match compute_custom_properties_detailed(&BTreeMap::new(), &own, limits()).unwrap() {
            CustomComputation::RequiresCascade { property, keyword } => {
                assert_eq!(property, "--b");
                assert_eq!(keyword, expected);
            }
            CustomComputation::Computed(_) => panic!("rollback was silently retained"),
        }
        own.insert("--b".into(), text("green"));
        let CustomComputation::Computed(result) =
            compute_custom_properties_detailed(&BTreeMap::new(), &own, limits()).unwrap()
        else {
            panic!("fresh retry retained rollback state");
        };
        assert_eq!(result.get("--a"), Some(&text("green")));
    }
}

#[test]
fn cyclic_fallback_keyword_does_not_request_rollback() {
    use turbo_dom::rtdom::computed_css::{compute_custom_properties_detailed, CustomComputation};
    for keyword in ["revert", "revert-layer"] {
        let own = BTreeMap::from([("--x".into(), text(&format!("var(--x,{keyword})")))]);
        let CustomComputation::Computed(result) =
            compute_custom_properties_detailed(&BTreeMap::new(), &own, limits()).unwrap()
        else {
            panic!("cyclic fallback requested rollback");
        };
        assert!(result.get("--x").is_none());
    }
}
