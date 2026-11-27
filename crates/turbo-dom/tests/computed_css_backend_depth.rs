#![cfg(feature = "computed-style-grammar")]
use turbo_dom::rtdom::computed_css::{
    declaration_model::compile_declarations,
    grammar::{
        check_grammar_limits, computed_declaration_value, declaration_value_checked,
        parse_media_condition_context, supports_condition_declaration, supports_declaration,
        ConditionError, GrammarLimitError, BACKEND_MAX_COMPONENT_DEPTH,
    },
    ComputationLimits, ComputedCssError, CssDeclaration, CssText,
};
fn limits() -> ComputationLimits {
    ComputationLimits {
        max_nesting: 600,
        max_output_units: 4096,
        max_total_units: 8192,
    }
}
fn nested(depth: usize) -> CssText {
    CssText::from_units(
        format!("{}red{}", "f(".repeat(depth), ")".repeat(depth))
            .encode_utf16()
            .collect(),
    )
}
#[test]
fn backend_off_by_one_is_checked_without_entering_recursive_backend() {
    assert_eq!(BACKEND_MAX_COMPONENT_DEPTH, 500);
    assert_eq!(check_grammar_limits(&nested(500), limits()), Ok(()));
    assert_eq!(
        check_grammar_limits(&nested(501), limits()),
        Err(GrammarLimitError::NestingLimit)
    );
}
#[test]
fn caller_stricter_equal_and_unit_policies_keep_original_error_identity() {
    let mut policy = limits();
    policy.max_nesting = 4;
    assert_eq!(check_grammar_limits(&nested(3), policy), Ok(()));
    assert_eq!(
        check_grammar_limits(&nested(4), policy),
        Err(GrammarLimitError::Computation(
            ComputedCssError::NestingLimit
        ))
    );
    policy.max_nesting = 501;
    assert_eq!(
        check_grammar_limits(&nested(501), policy),
        Err(GrammarLimitError::Computation(
            ComputedCssError::NestingLimit
        ))
    );
    policy = limits();
    policy.max_output_units = 2;
    assert_eq!(
        check_grammar_limits(&nested(501), policy),
        Err(GrammarLimitError::Computation(
            ComputedCssError::ResourceLimit
        ))
    );
}
#[test]
fn every_checked_entrypoint_rejects_excess_backend_depth_before_parsing() {
    let raw = String::from_utf16(nested(501).as_units()).unwrap();
    assert_eq!(
        declaration_value_checked("color", &raw, limits()),
        Err(ConditionError::NestingLimit)
    );
    assert!(matches!(
        computed_declaration_value("color", &raw, limits()),
        Err(ConditionError::NestingLimit)
    ));
    assert_eq!(
        supports_declaration("color", &raw, limits()),
        Err(ConditionError::NestingLimit)
    );
    assert_eq!(
        supports_condition_declaration("color", &raw, limits()),
        Err(ConditionError::NestingLimit)
    );
    assert!(matches!(
        parse_media_condition_context(&raw, limits()),
        Err(ConditionError::NestingLimit)
    ));
    let source = [CssDeclaration {
        name: "color".into(),
        value: nested(501),
        important: false,
    }];
    assert!(matches!(
        compile_declarations(&source, limits()),
        Err(ComputedCssError::NestingLimit)
    ));
}
#[test]
fn ordinary_checked_values_and_raw_unit_accounting_stay_compatible() {
    assert_eq!(
        declaration_value_checked("width", "2em", limits())
            .unwrap()
            .as_deref(),
        Some("2em")
    );
    assert_eq!(
        supports_condition_declaration("display", "grid", limits()),
        Ok(true)
    );
    let mut policy = limits();
    policy.max_output_units = 1;
    assert_eq!(
        check_grammar_limits(&CssText::from_units(vec![0xd800]), policy),
        Ok(())
    );
    assert_eq!(
        check_grammar_limits(&CssText::from_units(vec![0xd800, 0xdfff]), policy),
        Err(GrammarLimitError::Computation(
            ComputedCssError::ResourceLimit
        ))
    );
}

#[test]
fn container_axis_uses_exact_resource_only_preflight_error_types() {
    use turbo_dom::rtdom::computed_css::container::{parse_container_context, ContainerError};
    let none = CssText::from_units("none".encode_utf16().collect());
    let normal = CssText::from_units("normal".encode_utf16().collect());
    assert_eq!(
        parse_container_context(&none, &normal, &nested(501), limits()),
        Err(ContainerError::Computation(ComputedCssError::NestingLimit))
    );
    let mut policy = limits();
    policy.max_output_units = 8;
    assert_eq!(
        parse_container_context(
            &none,
            &normal,
            &CssText::from_units("horizontal-tb".encode_utf16().collect()),
            policy
        ),
        Err(ContainerError::Computation(ComputedCssError::ResourceLimit))
    );
}
