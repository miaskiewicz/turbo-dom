#![cfg(feature = "computed-style-grammar")]
use turbo_dom::rtdom::computed_css::{
    grammar::{
        check_grammar_limits, computed_declaration_value, ConditionError, GrammarLimitError,
    },
    ComputationLimits, ComputedCssError, CssText,
};
fn limits() -> ComputationLimits {
    ComputationLimits {
        max_nesting: 600,
        max_output_units: 32768,
        max_total_units: 0,
    }
}
fn text(value: &str) -> CssText {
    CssText::from_units(value.encode_utf16().collect())
}
fn wrapped(prefix: &str, count: usize, leaf: &str) -> CssText {
    text(&format!(
        "{}{leaf}{}",
        prefix.repeat(count),
        ")".repeat(count)
    ))
}
#[test]
fn env_fallback_uses_two_backend_edges_without_calling_the_backend() {
    assert_eq!(
        check_grammar_limits(&wrapped("env(foo,", 250, "red"), limits()),
        Ok(())
    );
    assert_eq!(
        check_grammar_limits(&wrapped("env(foo,", 251, "red"), limits()),
        Err(GrammarLimitError::NestingLimit)
    );
    assert_eq!(
        check_grammar_limits(&wrapped("env(foo,", 250, "red,blue"), limits()),
        Ok(())
    );
}
#[test]
fn variable_and_environment_prefixes_do_not_spend_a_tokenlist_edge_without_fallback() {
    for leaf in ["var(--x)", "env(foo)"] {
        assert_eq!(
            check_grammar_limits(&wrapped("env(foo,", 250, leaf), limits()),
            Ok(()),
            "{leaf}"
        );
    }
    for leaf in ["var(--x,red)", "env(foo,red)", "ENV(foo)"] {
        assert_eq!(
            check_grammar_limits(&wrapped("env(foo,", 250, leaf), limits()),
            Err(GrammarLimitError::NestingLimit),
            "{leaf}"
        );
    }
}
#[test]
fn escaped_env_names_count_but_quoted_comments_and_inner_commas_are_not_new_fallbacks() {
    assert_eq!(
        check_grammar_limits(&wrapped("\\65 nv(foo,", 251, "red"), limits()),
        Err(GrammarLimitError::NestingLimit)
    );
    assert_eq!(
        check_grammar_limits(&wrapped("ENV(foo,", 251, "red"), limits()),
        Ok(())
    );
    assert_eq!(
        check_grammar_limits(&wrapped("var(--x,", 500, "red"), limits()),
        Ok(())
    );
    assert_eq!(
        check_grammar_limits(
            &text(&format!(
                "\"{}\" /* {} */",
                "env(foo,".repeat(251),
                "env(foo,".repeat(251)
            )),
            limits()
        ),
        Ok(())
    );
    assert_eq!(
        check_grammar_limits(&wrapped("env(foo,", 249, "f(a,b)"), limits()),
        Ok(())
    );
}
#[test]
fn raw_caller_limits_remain_independent_from_semantic_backend_cost() {
    let value = wrapped("env(foo,", 3, "red");
    let mut policy = limits();
    policy.max_nesting = 4;
    assert_eq!(check_grammar_limits(&value, policy), Ok(()));
    policy.max_nesting = 3;
    assert_eq!(
        check_grammar_limits(&value, policy),
        Err(GrammarLimitError::Computation(
            ComputedCssError::NestingLimit
        ))
    );
    policy = limits();
    policy.max_output_units = 2;
    assert_eq!(
        check_grammar_limits(&wrapped("env(foo,", 251, "red"), policy),
        Err(GrammarLimitError::Computation(
            ComputedCssError::ResourceLimit
        ))
    );
}
#[test]
fn unsafe_weighted_values_are_refused_before_recursive_admission() {
    let value = wrapped("env(foo,", 251, "red");
    let value = String::from_utf16(value.as_units()).unwrap();
    assert!(matches!(
        computed_declaration_value("color", &value, limits()),
        Err(ConditionError::NestingLimit)
    ));
}

#[test]
fn malformed_prefixes_are_inspected_for_resources_but_not_admitted_as_css() {
    use turbo_dom::rtdom::computed_css::grammar::DeclarationAdmission;
    let source = "var(f(g(a,b)), x)";
    assert_eq!(check_grammar_limits(&text(source), limits()), Ok(()));
    assert_eq!(
        computed_declaration_value("color", source, limits()).unwrap(),
        DeclarationAdmission::Invalid
    );
}
