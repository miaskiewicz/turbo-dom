use turbo_dom::rtdom::computed_css::{
    has_variable_references, ComputationLimits, ComputedCssError, CssText,
};

fn limits() -> ComputationLimits {
    ComputationLimits {
        max_nesting: 16,
        max_output_units: 1000,
        max_total_units: 1000,
    }
}

#[test]
fn reference_probe_uses_canonical_css_tokens() {
    for (source, expected) in [
        ("var(--paint)", true),
        (r"v\61r(--paint)", true),
        ("VAR(--paint)", true),
        ("[var(--paint)]", true),
        ("{var(--paint)}", true),
        ("(var(--paint))", true),
        ("rgb(calc(var(--channel)) 0 0)", true),
        ("'var(--paint)'", false),
        ("/*var(--paint)*/ red", false),
        ("url('var(--paint)')", false),
        ("url(var(--paint))", false),
        ("var(bad-name)", true),
        ("red", false),
    ] {
        assert_eq!(
            has_variable_references(
                &CssText::from_units(source.encode_utf16().collect()),
                limits()
            ),
            Ok(expected),
            "{source}"
        );
    }
}

#[test]
fn reference_probe_preserves_typed_nesting_failure() {
    let source = format!("{}red{}", "f(".repeat(17), ")".repeat(17));
    assert_eq!(
        has_variable_references(
            &CssText::from_units(source.encode_utf16().collect()),
            limits()
        ),
        Err(ComputedCssError::NestingLimit)
    );
}

#[test]
fn borrowed_computed_snapshot_remains_immutable_during_child_computation() {
    use std::collections::BTreeMap;
    use turbo_dom::rtdom::computed_css::compute_custom_properties;
    let text = |source: &str| CssText::from_units(source.encode_utf16().collect());
    let parent = compute_custom_properties(
        &BTreeMap::new(),
        &BTreeMap::from([("--color".into(), text("#22cc44"))]),
        limits(),
    )
    .unwrap();
    let child = compute_custom_properties(
        parent.as_map(),
        &BTreeMap::from([("--copy".into(), text("var(--color)"))]),
        limits(),
    )
    .unwrap();
    assert_eq!(child.get("--copy"), Some(&text("#22cc44")));
    assert_eq!(parent.get("--color"), Some(&text("#22cc44")));
}

#[test]
fn component_limits_walk_unused_blocks_without_deciding_var_grammar() {
    use turbo_dom::rtdom::computed_css::check_component_limits;
    let text = |source: &str| CssText::from_units(source.encode_utf16().collect());
    for source in [
        "var()",
        "var(not-custom)",
        "var(--defined,var(bad))",
        "[{}]",
        "'bad\nstring'",
    ] {
        assert_eq!(
            check_component_limits(&text(source), limits()),
            Ok(()),
            "{source}"
        );
    }
    let source = format!("var(--defined,{}red{})", "f(".repeat(17), ")".repeat(17));
    assert_eq!(
        check_component_limits(&text(&source), limits()),
        Err(ComputedCssError::NestingLimit)
    );
    let mut quota = limits();
    quota.max_output_units = 3;
    assert_eq!(check_component_limits(&text("red"), quota), Ok(()));
    assert_eq!(
        check_component_limits(&text("blue"), quota),
        Err(ComputedCssError::ResourceLimit)
    );
    quota.max_nesting = 0;
    assert_eq!(
        check_component_limits(&text(""), quota),
        Err(ComputedCssError::NestingLimit)
    );
    quota.max_nesting = 1;
    assert_eq!(check_component_limits(&text("red"), quota), Ok(()));
    quota.max_output_units = 100;
    assert_eq!(
        check_component_limits(&text("f(red)"), quota),
        Err(ComputedCssError::NestingLimit)
    );
}

#[test]
fn named_function_probe_does_not_confuse_environment_and_variable_functions() {
    use turbo_dom::rtdom::computed_css::has_function_reference;
    let text = |source: &str| CssText::from_units(source.encode_utf16().collect());
    assert_eq!(
        has_function_reference(&text("calc(env(safe-area-inset-top))"), "env", limits()),
        Ok(true)
    );
    assert_eq!(
        has_function_reference(&text("var(--env)"), "env", limits()),
        Ok(false)
    );
}

#[test]
fn declaration_serialization_escapes_names_and_preserves_raw_values() {
    use turbo_dom::rtdom::computed_css::CssDeclaration;
    let value = CssText::from_units(vec![
        0xd800,
        u16::from(b'/'),
        u16::from(b'*'),
        u16::from(b'x'),
        u16::from(b'*'),
        u16::from(b'/'),
    ]);
    let declaration = CssDeclaration {
        name: "--p:a".into(),
        value: value.clone(),
        important: true,
    };
    let mut expected: Vec<u16> = "--p\\:a:".encode_utf16().collect();
    expected.extend(value.as_units());
    expected.extend(" !important;".encode_utf16());
    assert_eq!(declaration.serialized().as_units(), expected);
    let declaration = CssDeclaration {
        name: "fill".into(),
        value: CssText::from_units("red".encode_utf16().collect()),
        important: false,
    };
    assert_eq!(
        declaration.serialized().as_units(),
        "fill:red;".encode_utf16().collect::<Vec<_>>()
    );
}

#[test]
fn structural_selector_certificate_is_conservative_through_logical_arguments() {
    use turbo_dom::rtdom::query::parse_for_cascade;
    for (source, expected) in [
        ("rect[style*=fill]", true),
        (":root", true),
        (":scope", true),
        ("style:not(:empty)+rect", true),
        (":is(:where(:root))", true),
        ("rect:hover", false),
        ("rect::before", false),
        (":is(:hover)", false),
        ("rect:first-child", false),
    ] {
        let selector = parse_for_cascade(source).unwrap();
        assert_eq!(
            selector.selectors()[0].certifies_structural_matching(),
            expected,
            "{source}"
        );
    }
}

#[test]
fn strict_value_syntax_is_distinct_from_limits_and_computed_invalidity() {
    use turbo_dom::rtdom::computed_css::valid_custom_value_syntax;
    let text = |source: &str| CssText::from_units(source.encode_utf16().collect());
    for (source, valid) in [
        ("var(--missing)", true),
        ("var(--cycle)", true),
        ("var(bad)", false),
        ("var(--defined,var(bad))", false),
        ("'bad\nstring'", false),
        ("", true),
    ] {
        assert_eq!(
            valid_custom_value_syntax(&text(source), limits()),
            Ok(valid),
            "{source}"
        );
    }
    let mut policy = limits();
    policy.max_nesting = 0;
    assert_eq!(
        valid_custom_value_syntax(&text("red"), policy),
        Err(ComputedCssError::NestingLimit)
    );
    policy.max_nesting = 16;
    policy.max_output_units = 2;
    assert_eq!(
        valid_custom_value_syntax(&text("red"), policy),
        Err(ComputedCssError::ResourceLimit)
    );
}
