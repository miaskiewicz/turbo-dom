#![cfg(feature = "computed-style-grammar")]
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use turbo_dom::rtdom::computed_css::{
    computed_values::ValueContext,
    conditions::{ConditionEnvironment, FeatureValue},
    custom_cascade::CustomCascadeLimits,
    facts::{AnswerDomain, MechanicalAnswer},
    inline_style::InlineLimits,
    model::{CompileLimits, CompiledStyles, SheetSource},
    prepared::{
        CascadeOrigin, PreparationEnvironment, PreparationLimits, PreparedStyles, SheetUse,
        StylePhase,
    },
    properties::InitialContext,
    resolve::{compute_style, ComputedStyle, ResolveLimits, TargetContext},
    rule_matches::{SelectorAnswers, TargetFacts},
    ComputationLimits, CssText,
};
use turbo_dom::rtdom::css_shorthand::LengthContext;
fn text(source: &str) -> CssText {
    CssText::from_units(source.encode_utf16().collect())
}
fn limits() -> ComputationLimits {
    ComputationLimits {
        max_nesting: 32,
        max_output_units: 16384,
        max_total_units: 32768,
    }
}
fn model(css: &str) -> PreparedStyles {
    let compiled = CompiledStyles::compile(
        &[SheetSource {
            id: 1,
            text: text(css),
        }],
        CompileLimits {
            sources: 4,
            source_units: 16384,
            rules: 128,
            source_rule_preludes: 128,
            selectors: 256,
            declarations: 256,
            properties: 2048,
            group_depth: 16,
            token_nesting: 32,
        },
    )
    .unwrap();
    let bases = [("em", 16.0), ("rem", 16.0)];
    let lengths = LengthContext { unit_bases: &bases };
    let features = BTreeMap::from([("width".to_owned(), FeatureValue::LengthPx(800.0))]);
    let selectors = BTreeMap::new();
    let conditions = ConditionEnvironment {
        media_type: "screen",
        features: &features,
        length_context: &lengths,
        selector_support: &selectors,
    };
    compiled
        .prepare(
            &[SheetUse {
                source_id: 1,
                origin: CascadeOrigin::Author,
                tree_scope: 0,
                implicit_scope_root: Some(1),
                scope_depth: 0,
                disabled: false,
                media: None,
            }],
            &PreparationEnvironment {
                conditions: &conditions,
                style_phase: Some(StylePhase::Steady),
            },
            PreparationLimits {
                uses: 4,
                rules: 128,
                layers: 128,
                computation: limits(),
            },
        )
        .unwrap()
}
fn answers(prepared: &PreparedStyles) -> SelectorAnswers {
    let domain = AnswerDomain::new(vec![2]).unwrap();
    prepared
        .metadata()
        .compiled
        .metadata()
        .selectors
        .iter()
        .enumerate()
        .flat_map(|(id, selector)| {
            selector
                .compiled
                .as_ref()
                .unwrap()
                .selectors()
                .iter()
                .enumerate()
                .map(move |(item, _)| (id, item))
        })
        .map(|(id, item)| {
            (
                (id, item, None),
                MechanicalAnswer::new(domain.clone(), vec![2]).unwrap(),
            )
        })
        .collect()
}
fn resolve(
    prepared: &PreparedStyles,
    answers: &SelectorAnswers,
    inline: Option<&CssText>,
    parent: Option<&ComputedStyle>,
) -> ComputedStyle {
    let empty = BTreeMap::new();
    let facts = TargetFacts {
        containers: None,
        path: &[2, 1],
        implicit_scope_root: 1,
        pseudo: None,
        selectors: answers,
        routes: None,
        boundaries: &empty,
    };
    let context = TargetContext {
        tree_scope: 0,
        scope_depth: 0,
        values: ValueContext {
            initial: InitialContext {
                color: "rgb(0, 0, 0)",
                font_family: "serif",
                font_size: "16px",
            },
            unit_bases: &[("rem", 16.0)],
            font_size_unit_bases: &[],
            is_root: false,
        },
    };
    compute_style(
        prepared,
        &facts,
        inline,
        parent,
        &context,
        ResolveLimits {
            matching_decisions: 4096,
            inline: InlineLimits {
                computation: limits(),
                declarations: 128,
                properties: 2048,
            },
            custom: CustomCascadeLimits {
                computation: limits(),
                rollbacks: 128,
            },
            values: turbo_dom::rtdom::computed_css::computed_values::ValueLimits {
                computation: limits(),
                retained_units: 32768,
            },
        },
    )
    .unwrap()
}
fn value(style: &ComputedStyle, name: &str) -> Option<String> {
    style
        .values
        .values
        .get(name)
        .map(|value| String::from_utf16(value.as_units()).unwrap())
}

#[test]
fn generated_single_profile_values_match_unchanged_measured_registry_without_enabling_all_defaults()
{
    use turbo_dom::rtdom::computed_css::{properties, property_registry};
    let source = property_registry::REGISTRY_JSON;
    assert_eq!(
        format!("{:x}", Sha256::digest(source.as_bytes())),
        property_registry::SOURCE_SHA256
    );
    let registry: serde_json::Value = serde_json::from_str(source).unwrap();
    for (name, profile) in registry["properties"].as_object().unwrap() {
        let expected = profile
            .get("initial")
            .and_then(|values| values.as_array())
            .filter(|values| values.len() == 1)
            .map(|values| values[0].as_str().unwrap());
        assert_eq!(
            property_registry::initial_single_value(name),
            expected,
            "{name}"
        );
    }
    assert_eq!(
        property_registry::initial_single_value("unknown-unmatched-property"),
        None
    );
    assert_eq!(
        properties::context_free_initial("background-blend-mode"),
        None
    );
    assert_eq!(
        properties::context_free_initial("unknown-unmatched-property"),
        None
    );
    for name in [
        "background-attachment",
        "background-clip",
        "background-origin",
        "background-size",
        "text-decoration-thickness",
    ] {
        let actual = properties::context_free_initial(name).unwrap();
        assert_eq!(
            String::from_utf16(actual.as_units()).unwrap(),
            property_registry::initial_single_value(name).unwrap()
        );
    }
}
#[test]
fn background_and_decoration_shorthands_compute_without_missing_initial_capabilities_against_actual_chrome159(
) {
    let cases = include_str!("fixtures/npm-cssom-compat-159/cases.json");
    let fixture = include_str!("fixtures/npm-cssom-compat-159/fixture.mjs");
    let oracle: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/npm-cssom-compat-159/CHROME159.json")).unwrap();
    assert_eq!(oracle["browser"], "159.0.8154.6");
    assert_eq!(
        oracle["casesSha256"],
        format!("{:x}", Sha256::digest(cases.as_bytes()))
    );
    assert_eq!(
        oracle["fixtureSha256"],
        format!("{:x}", Sha256::digest(fixture.as_bytes()))
    );
    let cases: serde_json::Value = serde_json::from_str(cases).unwrap();
    for (id, checks) in [
        ("background-shorthand", &["background-color"][..]),
        (
            "text-decoration-declared",
            &[
                "text-decoration-line",
                "text-decoration-style",
                "text-decoration-color",
            ][..],
        ),
    ] {
        let case = cases
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["id"] == id)
            .unwrap();
        let control = oracle["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["id"] == id)
            .unwrap();
        let source = case["inline"].as_str().unwrap();
        // The same authored declaration is compiled as a stylesheet and inline.
        // Both routes share the canonical admission/expansion/default boundary.
        for inline in [false, true] {
            let prepared = model(&format!(".x{{{}}}", if inline { "" } else { source }));
            let answers = answers(&prepared);
            let inline_text = inline.then(|| text(source));
            let result = resolve(&prepared, &answers, inline_text.as_ref(), None);
            for property in checks {
                let expected = control["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|row| row["name"] == *property)
                    .unwrap()["computed"]["getPropertyValue"]["value"]
                    .as_str()
                    .unwrap();
                assert_eq!(
                    value(&result, property).unwrap(),
                    expected,
                    "{id}:{property}"
                );
            }
            assert_eq!(value(&result, "font-weight"), None);
            assert_eq!(value(&result, "unknown-unmatched-property"), None);
        }
    }
}
