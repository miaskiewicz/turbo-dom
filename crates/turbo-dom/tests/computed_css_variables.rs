use std::collections::BTreeMap;
use turbo_dom::rtdom::computed_css::{
    compute_custom_properties as compute_with_limits, declarations,
    substitute as substitute_with_limits, ComputationLimits, ComputedCssError, CssText,
    CustomProperties,
};
fn limits() -> ComputationLimits {
    ComputationLimits {
        max_nesting: 64,
        max_output_units: 4096,
        max_total_units: 16384,
    }
}
fn compute_custom_properties(
    parent: &BTreeMap<String, CssText>,
    own: &BTreeMap<String, CssText>,
) -> Result<CustomProperties, ComputedCssError> {
    compute_with_limits(parent, own, limits())
}
fn substitute(
    value: &CssText,
    variables: &CustomProperties,
) -> Result<Option<CssText>, ComputedCssError> {
    substitute_with_limits(value, variables, limits())
}

fn text(value: &str) -> CssText {
    CssText::from_units(value.encode_utf16().collect())
}
fn own(value: &str) -> BTreeMap<String, CssText> {
    declarations(&value.encode_utf16().collect::<Vec<_>>())
        .into_iter()
        .map(|d| (d.name, d.value))
        .collect()
}

#[test]
fn valid_empty_and_invalid_cycles_have_distinct_fallback_behavior() {
    let vars =
        compute_custom_properties(&BTreeMap::new(), &own("--empty:;--cycle:var(--cycle,red)"))
            .unwrap();
    assert_eq!(vars.get("--empty").unwrap().as_units(), []);
    assert!(vars.get("--cycle").is_none());
    assert_eq!(
        substitute(&text("var(--empty,red)"), &vars)
            .unwrap()
            .unwrap()
            .as_units(),
        []
    );
    assert_eq!(
        substitute(&text("var(--cycle,blue)"), &vars)
            .unwrap()
            .unwrap()
            .as_units(),
        text("blue").as_units()
    );
}
#[test]
fn quoted_functions_and_inherited_computed_values_are_not_reinterpreted() {
    let parent =
        compute_custom_properties(&BTreeMap::new(), &own("--a:red;--token:var(--a)")).unwrap();
    let vars = compute_custom_properties(
        &parent.values(),
        &own("--a:blue;--out:var(--token);--literal:'var(--missing)'"),
    )
    .unwrap();
    assert_eq!(
        vars.get("--out").unwrap().as_units(),
        text("red").as_units()
    );
    assert_eq!(
        vars.get("--literal").unwrap().as_units(),
        text("'var(--missing)'").as_units()
    );
}
#[test]
fn only_used_fallbacks_participate_in_the_chrome_cycle_path() {
    let vars = compute_custom_properties(
        &BTreeMap::new(),
        &own("--a:green;--b:var(--a,var(--b));--out:var(--b,blue)"),
    )
    .unwrap();
    assert_eq!(
        vars.get("--out").unwrap().as_units(),
        text("green").as_units()
    );
    let vars = compute_custom_properties(
        &BTreeMap::new(),
        &own("--a:var(--b,red);--b:var(--a,green);--out:var(--a,blue)"),
    )
    .unwrap();
    assert!(vars.get("--a").is_none());
    assert!(vars.get("--b").is_none());
    assert_eq!(
        vars.get("--out").unwrap().as_units(),
        text("blue").as_units()
    );
}
#[test]
fn substitution_preserves_raw_units_and_nested_function_delimiters() {
    let raw = CssText::from_units(vec![34, 0xd800, 34]);
    let vars = compute_custom_properties(
        &BTreeMap::new(),
        &BTreeMap::from([("--Token".into(), raw.clone())]),
    )
    .unwrap();
    assert_eq!(
        substitute(&text("var(--Token)"), &vars).unwrap().unwrap(),
        raw
    );
    assert_eq!(
        substitute(&text("calc(1px + var(--x,2em))"), &vars)
            .unwrap()
            .unwrap()
            .as_units(),
        text("calc(1px + 2em)").as_units()
    );
    assert_eq!(
        substitute(&text("var(--missing,)"), &vars)
            .unwrap()
            .unwrap()
            .as_units(),
        []
    );
    assert!(substitute(&text("var(--missing)"), &vars)
        .unwrap()
        .is_none());
}

#[test]
fn nested_custom_token_spacing_survives_substitution() {
    let vars =
        compute_custom_properties(&BTreeMap::new(), &own("--out:foo( var(--x, red) )")).unwrap();
    assert_eq!(
        vars.get("--out").unwrap().as_units(),
        text("foo( red )").as_units()
    );
}

#[test]
fn unresolved_cascade_and_excessive_nesting_are_explicit_errors() {
    for keyword in ["revert", "revert-layer"] {
        assert_eq!(
            compute_custom_properties(&BTreeMap::new(), &own(&format!("--a:{keyword}")))
                .unwrap_err(),
            ComputedCssError::RequiresCascade
        );
    }
    let vars = compute_custom_properties(&BTreeMap::new(), &BTreeMap::new()).unwrap();
    let deeply_nested = format!("{}red{}", "f(".repeat(65), ")".repeat(65));
    assert_eq!(
        substitute(&text(&deeply_nested), &vars),
        Err(ComputedCssError::NestingLimit)
    );
    for value in ["var(color)", "var(--)", "var(--missing red)"] {
        assert!(substitute(&text(value), &vars).unwrap().is_none());
    }
}

#[test]
fn css_wide_keywords_use_tokens_including_comments_and_escapes() {
    for value in ["initial/**/", r"in\69 tial"] {
        let vars = compute_custom_properties(
            &own("--a:red"),
            &own(&format!("--a:{value};--out:var(--a,blue)")),
        )
        .unwrap();
        assert!(vars.get("--a").is_none());
        assert_eq!(vars.get("--out").unwrap(), &text("blue"));
    }
}

#[test]
fn expansion_resource_failures_are_not_css_fallbacks() {
    let declarations =
        own("--a:xx;--b:var(--a)var(--a);--c:var(--b)var(--b);--d:var(--c,var(--missing,blue))");
    let constrained = ComputationLimits {
        max_output_units: 32,
        max_total_units: 9,
        ..limits()
    };
    assert_eq!(
        compute_with_limits(&BTreeMap::new(), &declarations, constrained).unwrap_err(),
        ComputedCssError::ResourceLimit
    );
    let doubling =
        own("--a:xxxxxxxxxxxxxxxx;--b:var(--a)var(--a);--c:var(--b)var(--b);--d:var(--c,blue)");
    assert_eq!(
        compute_with_limits(
            &BTreeMap::new(),
            &doubling,
            ComputationLimits {
                max_output_units: 32,
                ..limits()
            }
        )
        .unwrap_err(),
        ComputedCssError::ResourceLimit
    );
    let own = BTreeMap::from([("--a".into(), text("xx"))]);
    let vars = compute_with_limits(&BTreeMap::new(), &own, limits()).unwrap();
    let value = text("var(--a)var(--a)");
    assert_eq!(
        substitute_with_limits(
            &value,
            &vars,
            ComputationLimits {
                max_output_units: 4,
                ..limits()
            }
        )
        .unwrap()
        .unwrap(),
        text("xxxx")
    );
    let raw = CssText::from_units(vec![0xd800, 0xd83d, 0xde00]);
    let vars = CustomProperties::default();
    assert_eq!(
        substitute_with_limits(
            &raw,
            &vars,
            ComputationLimits {
                max_output_units: 3,
                ..limits()
            }
        )
        .unwrap(),
        Some(raw.clone())
    );
    assert_eq!(
        substitute_with_limits(
            &raw,
            &vars,
            ComputationLimits {
                max_output_units: 2,
                ..limits()
            }
        ),
        Err(ComputedCssError::ResourceLimit)
    );
    let inherited = BTreeMap::from([("--a".into(), raw)]);
    assert_eq!(
        compute_with_limits(
            &inherited,
            &BTreeMap::new(),
            ComputationLimits {
                max_total_units: 2,
                ..limits()
            }
        )
        .unwrap_err(),
        ComputedCssError::ResourceLimit
    );
}

#[test]
fn caller_limits_preserve_empty_values_and_exact_boundaries() {
    let empty = ComputationLimits {
        max_nesting: 1,
        max_output_units: 0,
        max_total_units: 0,
    };
    let vars = compute_with_limits(&BTreeMap::new(), &own("--a:"), empty).unwrap();
    assert_eq!(vars.get("--a").unwrap(), &text(""));
    assert_eq!(
        substitute_with_limits(&text(""), &vars, empty).unwrap(),
        Some(text(""))
    );
    assert_eq!(
        substitute_with_limits(&text("x"), &vars, empty),
        Err(ComputedCssError::ResourceLimit)
    );
    assert_eq!(
        substitute_with_limits(
            &text(""),
            &vars,
            ComputationLimits {
                max_nesting: 0,
                ..empty
            }
        ),
        Err(ComputedCssError::NestingLimit)
    );
    let exact = ComputationLimits {
        max_nesting: 4,
        max_output_units: 3,
        max_total_units: 3,
    };
    let vars = compute_with_limits(&BTreeMap::new(), &own("--a:xxx"), exact).unwrap();
    assert_eq!(vars.get("--a").unwrap(), &text("xxx"));
    assert_eq!(
        substitute_with_limits(&text("f(x)"), &vars, exact),
        Err(ComputedCssError::ResourceLimit)
    );
}

#[test]
fn ordinary_and_empty_dashed_names_do_not_become_custom_properties() {
    let vars =
        compute_custom_properties(&BTreeMap::new(), &own("color:red;--:red;--valid:blue")).unwrap();
    assert!(vars.get("color").is_none());
    assert!(vars.get("--").is_none());
    assert_eq!(vars.get("--valid").unwrap(), &text("blue"));
    assert!(substitute(&text("var(--,blue)"), &vars).unwrap().is_none());
}

#[test]
fn nesting_limit_spans_dependency_and_structural_token_recursion() {
    let properties = own("--a:f(f(var(--b)));--b:f(f(var(--c)));--c:green");
    let constrained = ComputationLimits {
        max_nesting: 6,
        ..limits()
    };
    assert_eq!(
        compute_with_limits(&BTreeMap::new(), &properties, constrained).unwrap_err(),
        ComputedCssError::NestingLimit
    );
    let vars = compute_with_limits(&BTreeMap::new(), &properties, limits()).unwrap();
    assert_eq!(vars.get("--a").unwrap(), &text("f(f(f(f(green))))"));
}

#[test]
fn malformed_tokens_are_invalid_even_inside_unused_nested_fallbacks() {
    let vars = compute_custom_properties(&BTreeMap::new(), &own("--a:green")).unwrap();
    for value in [
        "'bad\nstring",
        "url(a\"b)",
        ")",
        "]",
        "}",
        "var(--a, f('bad\nstring))",
        "var(--a, f(url(a\"b)))",
        "var(--a,var())",
        "var(--a,var(color))",
        "var(--a,f(]))",
    ] {
        assert!(
            substitute(&text(value), &vars).unwrap().is_none(),
            "{value:?}"
        );
    }
}

#[test]
fn typed_computation_failures_do_not_select_variable_fallbacks() {
    for (source, policy, expected) in [
        (
            "--a:var(--b,blue);--b:revert",
            limits(),
            ComputedCssError::RequiresCascade,
        ),
        (
            "--a:var(--b,blue);--b:f(f(green))",
            ComputationLimits {
                max_nesting: 3,
                ..limits()
            },
            ComputedCssError::NestingLimit,
        ),
    ] {
        assert_eq!(
            compute_with_limits(&BTreeMap::new(), &own(source), policy).unwrap_err(),
            expected
        );
    }
    let vars = compute_custom_properties(&BTreeMap::new(), &own("--a:xx")).unwrap();
    assert_eq!(
        substitute_with_limits(
            &text("var(--a,red)"),
            &vars,
            ComputationLimits {
                max_output_units: 1,
                ..limits()
            }
        ),
        Err(ComputedCssError::ResourceLimit)
    );
    let vars =
        compute_custom_properties(&BTreeMap::new(), &own("--a:var(--b,blue);--b:var(color)"))
            .unwrap();
    assert_eq!(vars.get("--a").unwrap(), &text("blue"));
    assert!(vars.get("--b").is_none());
}
