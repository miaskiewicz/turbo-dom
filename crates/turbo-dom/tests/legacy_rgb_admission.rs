use sha2::{Digest, Sha256};
use turbo_dom::rtdom::css_shorthand::{longhand_value_css, Spelled};
fn oracle() -> serde_json::Value {
    let source = include_bytes!("fixtures/legacy-rgb-159/probe.mjs");
    let result: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/legacy-rgb-159/ORACLE.json")).unwrap();
    assert_eq!(result["browserVersion"], "159.0.8154.6");
    assert_eq!(
        result["sourceSha256"],
        format!("{:x}", Sha256::digest(source))
    );
    result
}
#[test]
fn canonical_legacy_rgb_channel_types_follow_actual_chrome159() {
    let math = oracle();
    assert_eq!(math["unresolvedMath"]["supports"], true);
    assert!(matches!(
        longhand_value_css("color", math["unresolvedMath"]["value"].as_str().unwrap()),
        Spelled::Valid(_)
    ));

    for control in oracle()["result"].as_array().unwrap() {
        let value = control["value"].as_str().unwrap();
        let accepted = matches!(longhand_value_css("color", value), Spelled::Valid(_));
        assert_eq!(
            accepted,
            control["literal"]["supports"].as_bool().unwrap(),
            "{value}"
        );
    }
}
#[cfg(feature = "computed-style-grammar")]
#[test]
fn literal_invalid_rgb_uses_lower_candidate_but_substituted_invalid_inherits() {
    use std::collections::BTreeMap;
    use turbo_dom::rtdom::computed_css::{
        candidate_set::CandidateSet,
        computed_values::{compute_values, ValueContext, ValueLimits},
        custom_cascade::CustomCascadeLimits,
        inline_style::{InlineLimits, InlineStyle},
        properties::InitialContext,
        selection::select_style,
        ComputationLimits, CssText,
    };
    let limits = ComputationLimits {
        max_nesting: 32,
        max_output_units: 4096,
        max_total_units: 8192,
    };
    let text = |s: &str| CssText::from_units(s.encode_utf16().collect());
    let context = ValueContext {
        initial: InitialContext {
            color: "black",
            font_family: "serif",
            font_size: "16px",
        },
        unit_bases: &[("rem", 16.0)],
        font_size_unit_bases: &[],
        is_root: false,
    };
    let value_limits = ValueLimits {
        computation: limits,
        retained_units: 8192,
    };
    let parent = compute_values(
        None,
        &BTreeMap::from([("color".to_owned(), Some(text("blue")))]),
        &context,
        value_limits,
    )
    .unwrap();
    let compute_source = |source: &str| {
        let inline = InlineStyle::compile(
            &text(source),
            InlineLimits {
                computation: limits,
                declarations: 8,
                properties: 16,
            },
        )
        .unwrap();
        let mut candidates = CandidateSet {
            ordinary: BTreeMap::new(),
            custom: BTreeMap::new(),
            declared_text_decoration: Vec::new(),
        };
        inline.append(&mut candidates, 0, 0);
        let selected = select_style(
            &BTreeMap::new(),
            &candidates.custom,
            &candidates.ordinary,
            CustomCascadeLimits {
                computation: limits,
                rollbacks: 8,
            },
        )
        .unwrap();
        let result =
            compute_values(Some(&parent), &selected.ordinary, &context, value_limits).unwrap();
        String::from_utf16(result.values["color"].as_units()).unwrap()
    };
    for control in oracle()["result"].as_array().unwrap() {
        let value = control["value"].as_str().unwrap();
        for (name, source) in [
            ("literal", format!("color:lime;color:{value}")),
            (
                "substituted",
                format!("--bad:{value};color:lime;color:var(--bad)"),
            ),
        ] {
            assert_eq!(
                compute_source(&source),
                control[name]["computed"].as_str().unwrap(),
                "{name}:{value}"
            );
        }
    }
    for control in oracle()["pending"].as_array().unwrap() {
        assert!(control["declared"].as_str().unwrap().contains("var("));
        assert_eq!(
            compute_source(control["source"].as_str().unwrap()),
            control["computed"].as_str().unwrap()
        );
    }
    let unused = oracle();
    assert_eq!(
        compute_source(unused["unusedFallback"]["css"].as_str().unwrap()),
        unused["unusedFallback"]["computed"].as_str().unwrap()
    );
}

#[cfg(feature = "computed-style-grammar")]
#[test]
fn unused_variable_fallback_keeps_raw_color_tokens_without_semantic_parsing() {
    use turbo_dom::rtdom::computed_css::{
        inline_style::{InlineLimits, InlineStyle},
        ComputationLimits, CssText,
    };
    let oracle = oracle();
    assert_eq!(oracle["unusedFallback"]["computed"], "rgb(0, 255, 0)");
    let source = oracle["unusedFallback"]["css"].as_str().unwrap();
    InlineStyle::compile(
        &CssText::from_units(source.encode_utf16().collect()),
        InlineLimits {
            computation: ComputationLimits {
                max_nesting: 32,
                max_output_units: 4096,
                max_total_units: 8192,
            },
            declarations: 8,
            properties: 16,
        },
    )
    .unwrap();
}

#[cfg(feature = "computed-style-grammar")]
#[test]
fn deferred_raw_ast_preserves_existing_heads_conditions_blocks_and_typed_limits() {
    use turbo_dom::rtdom::computed_css::{
        grammar::{computed_declaration_value, ConditionError, DeclarationAdmission},
        ComputationLimits, ComputedCssError,
    };
    let policy = ComputationLimits {
        max_nesting: 32,
        max_output_units: 4096,
        max_total_units: 8192,
    };
    for source in [
        "var(--ok, rgb(50%,calc(0),0%))",
        "var(--ok, [rgb(50%,calc(0),0%)] { raw })",
        "env(viewport-segment-width 0 1, rgb(50%,calc(0),0%))",
        "env(safe-area-inset-top)",
        "if(style(--x: 1): rgb(50%,calc(0),0%); else: lime)",
        "if((style(--x: 1) or media(width > 1px)):red;else:blue)",
        "var(--x, /* comment */ f(\"!;\", (raw)))",
    ] {
        assert_eq!(
            computed_declaration_value("color", source, policy).unwrap(),
            DeclarationAdmission::Valid(source.to_owned()),
            "{source}"
        );
    }
    for source in [
        "var(--x;red)",
        "var(--x,env())",
        "env(foo,env())",
        "f(env())",
        "env(,red)",
        "env(foo 1.5)",
        "if(else red)",
        "var(--x, red !)",
        "var(--x, red;blue)",
        "var(--x, ] )",
        "var(--x, \"bad\nstring\")",
    ] {
        assert_eq!(
            computed_declaration_value("color", source, policy).unwrap(),
            DeclarationAdmission::Invalid,
            "{source}"
        );
    }
    let source = "var(--x,red)";
    assert!(matches!(
        computed_declaration_value(
            "color",
            source,
            ComputationLimits {
                max_nesting: 0,
                ..policy
            }
        ),
        Err(ConditionError::Computation(ComputedCssError::NestingLimit))
    ));
    assert!(matches!(
        computed_declaration_value(
            "color",
            source,
            ComputationLimits {
                max_output_units: source.len() - 1,
                ..policy
            }
        ),
        Err(ConditionError::Computation(ComputedCssError::ResourceLimit))
    ));
}
