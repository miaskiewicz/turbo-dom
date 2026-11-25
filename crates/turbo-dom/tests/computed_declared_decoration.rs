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
    model_with_origins(css, &[CascadeOrigin::Author])
}
fn model_with_origins(css: &str, origins: &[CascadeOrigin]) -> PreparedStyles {
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
            &origins
                .iter()
                .map(|origin| SheetUse {
                    source_id: 1,
                    origin: *origin,
                    tree_scope: 0,
                    scope_depth: 0,
                    implicit_scope_root: Some(1),
                    disabled: false,
                    media: None,
                })
                .collect::<Vec<_>>(),
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
fn resolve_bounded(
    prepared: &PreparedStyles,
    answers: &SelectorAnswers,
    inline: Option<&CssText>,
    parent: Option<&ComputedStyle>,
    retained_units: usize,
) -> Result<ComputedStyle, turbo_dom::rtdom::computed_css::resolve::ResolveError> {
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
                retained_units,
            },
        },
    )
}
fn resolve(
    prepared: &PreparedStyles,
    answers: &SelectorAnswers,
    inline: Option<&CssText>,
    parent: Option<&ComputedStyle>,
) -> ComputedStyle {
    resolve_bounded(prepared, answers, inline, parent, 32768).unwrap()
}
fn value(style: &ComputedStyle, name: &str) -> Option<String> {
    style
        .values
        .values
        .get(name)
        .map(|value| String::from_utf16(value.as_units()).unwrap())
}
fn marker(style: &ComputedStyle) -> Option<String> {
    style
        .declared_text_decoration
        .as_ref()
        .map(|marker| String::from_utf16(marker.value.as_units()).unwrap())
}
#[test]
fn declared_markers_match_existing_frozen_native_jsdom_shorthand_controls() {
    let control: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/declared-decoration-jsdom27/control.json"
    ))
    .unwrap();
    assert!(control["jsdom"].as_str().unwrap().starts_with("27."));
    assert_eq!(
        control["probeSha256"],
        format!(
            "{:x}",
            Sha256::digest(include_bytes!(
                "fixtures/declared-decoration-jsdom27/probe.js"
            ))
        )
    );
    for (id, source) in [
        ("strike", "line-through"),
        ("explicit-color", "underline red"),
        ("explicit-style", "underline dotted blue"),
    ] {
        let expected = control["result"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["name"] == id)
            .unwrap()["value"]
            .as_str()
            .unwrap();
        for inline in [false, true] {
            let declaration = format!("text-decoration:{source}");
            let prepared = model(&format!(".x{{{}}}", if inline { "" } else { &declaration }));
            let answers = answers(&prepared);
            let inline = inline.then(|| text(&declaration));
            let output = resolve(&prepared, &answers, inline.as_ref(), None);
            assert_eq!(marker(&output).unwrap(), expected);
            let provenance = output.declared_text_decoration.unwrap();
            assert_eq!(provenance.origin, CascadeOrigin::Author);
            assert_eq!(provenance.inline, inline.is_some());
        }
    }
}
#[test]
fn longhand_only_has_no_marker_and_longhand_override_does_not_rewrite_declared_shorthand() {
    let prepared = model(".x{text-decoration-line:underline;text-decoration-color:blue}");
    assert_eq!(
        marker(&resolve(&prepared, &answers(&prepared), None, None)),
        None
    );
    let prepared = model(".x{text-decoration:underline;text-decoration-color:blue}");
    let output = resolve(&prepared, &answers(&prepared), None, None);
    assert_eq!(marker(&output).as_deref(), Some("underline"));
    assert_eq!(
        value(&output, "text-decoration-color").as_deref(),
        Some("rgb(0, 0, 255)")
    );
    let prepared = model(".x{text-decoration:underline #161925}");
    let output = resolve(&prepared, &answers(&prepared), None, None);
    assert_eq!(
        marker(&output).as_deref(),
        Some("underline rgb(22, 25, 37)")
    );
}
#[test]
fn marker_winners_reuse_importance_layer_inline_order_and_origin_rank() {
    for (css,expected) in [
        (".x{text-decoration:underline}.x{text-decoration:line-through}","line-through"),
        (".x{text-decoration:underline!important}.x{text-decoration:line-through}","underline"),
        ("@layer first,last;@layer first{.x{text-decoration:underline}}@layer last{.x{text-decoration:line-through}}","line-through"),
        ("@layer first,last;@layer first{.x{text-decoration:underline!important}}@layer last{.x{text-decoration:line-through!important}}","underline"),
    ] {
        let prepared=model(css);assert_eq!(marker(&resolve(&prepared,&answers(&prepared),None,None)).as_deref(),Some(expected),"{css}");
    }
    for (css, expected, important) in [
        (".x{text-decoration:underline}", "line-through", false),
        (".x{text-decoration:underline!important}", "underline", true),
    ] {
        let prepared = model(css);
        let output = resolve(
            &prepared,
            &answers(&prepared),
            Some(&text("text-decoration:line-through")),
            None,
        );
        assert_eq!(marker(&output).as_deref(), Some(expected));
        assert_eq!(
            output.declared_text_decoration.as_ref().unwrap().important,
            important
        );
    }
    for (suffix, origin) in [
        ("", CascadeOrigin::Author),
        ("!important", CascadeOrigin::UserAgent),
    ] {
        let prepared = model_with_origins(
            &format!(".x{{text-decoration:underline{suffix}}}"),
            &[
                CascadeOrigin::UserAgent,
                CascadeOrigin::User,
                CascadeOrigin::Author,
            ],
        );
        let output = resolve(&prepared, &answers(&prepared), None, None);
        assert_eq!(output.declared_text_decoration.unwrap().origin, origin);
    }
}
#[test]
fn pending_and_csswide_marker_tokens_remain_declared_while_longhands_compute_independently() {
    let prepared = model(".x{--dec:underline;text-decoration:var(--dec)}");
    let output = resolve(&prepared, &answers(&prepared), None, None);
    assert_eq!(marker(&output).as_deref(), Some("var(--dec)"));
    assert_eq!(
        value(&output, "text-decoration-line").as_deref(),
        Some("underline")
    );
    for keyword in ["inherit", "unset", "initial", "revert", "revert-layer"] {
        let prepared = model(&format!(".x{{text-decoration:{keyword}}}"));
        let output = resolve(&prepared, &answers(&prepared), None, None);
        assert_eq!(marker(&output).as_deref(), Some(keyword));
    }
}
#[test]
fn pending_marker_retains_raw_utf16_and_only_admitted_shorthands_get_annotations() {
    use turbo_dom::rtdom::computed_css::{declaration_model::compile_declarations, declarations};
    let mut units = "text-decoration:var(--missing, '"
        .encode_utf16()
        .collect::<Vec<_>>();
    units.push(0xd800);
    units.extend("')".encode_utf16());
    let parsed = declarations(&units);
    let compiled = compile_declarations(&parsed, limits()).unwrap();
    assert_eq!(
        compiled[0].declared_text_decoration.as_ref().unwrap(),
        &parsed[0].value
    );
    let parsed = declarations(
        &"text-decoration:var();text-decoration-line:underline;invented-property:red"
            .encode_utf16()
            .collect::<Vec<_>>(),
    );
    let compiled = compile_declarations(&parsed, limits()).unwrap();
    assert_eq!(compiled.len(), 1);
    assert!(compiled[0].declared_text_decoration.is_none());
}
#[test]
fn marker_payload_shares_existing_retained_output_budget_without_materializing_public_shorthand() {
    use turbo_dom::rtdom::computed_css::{
        computed_values::ValueError, resolve::ResolveError, ComputedCssError,
    };
    let prepared = model(".x{text-decoration:underline red}");
    let answers = answers(&prepared);
    let output = resolve(&prepared, &answers, None, None);
    let value_units = output
        .values
        .values
        .values()
        .map(|value| value.as_units().len())
        .sum::<usize>();
    let marker_units = output
        .declared_text_decoration
        .as_ref()
        .unwrap()
        .value
        .as_units()
        .len();
    assert!(!output.public_keys.contains("text-decoration"));
    assert!(matches!(
        resolve_bounded(&prepared, &answers, None, None, value_units),
        Err(ResolveError::Values(ValueError::Computation(
            ComputedCssError::ResourceLimit
        )))
    ));
    assert_eq!(
        marker(
            &resolve_bounded(&prepared, &answers, None, None, value_units + marker_units).unwrap()
        ),
        marker(&output)
    );
}

#[test]
fn marker_is_not_inherited_but_literal_inherit_annotation_is_preserved_over_computed_parent() {
    let prepared = model(".x{text-decoration:line-through red}");
    let parent = resolve(&prepared, &answers(&prepared), None, None);
    let empty = model(".x{}");
    let child = resolve(&empty, &answers(&empty), None, Some(&parent));
    assert_eq!(marker(&child), None);
    let inherited = model(".x{text-decoration:inherit}");
    let child = resolve(&inherited, &answers(&inherited), None, Some(&parent));
    assert_eq!(marker(&child).as_deref(), Some("inherit"));
    assert_eq!(
        value(&child, "text-decoration-line").as_deref(),
        Some("line-through")
    );
}

#[test]
fn ua_marker_keeps_origin_without_adding_public_keys_or_retaining_model_owner() {
    let prepared = model_with_origins(".x{text-decoration:underline}", &[CascadeOrigin::UserAgent]);
    let output = resolve(&prepared, &answers(&prepared), None, None);
    assert!(output.public_keys.is_empty());
    assert_eq!(
        output.declared_text_decoration.as_ref().unwrap().origin,
        CascadeOrigin::UserAgent
    );
    drop(prepared);
    assert_eq!(marker(&output).as_deref(), Some("underline"));
}

#[test]
fn marker_uses_existing_scope_proximity_before_later_source_order() {
    let prepared = model("@scope{.x{text-decoration:underline}}.x{text-decoration:line-through}");
    let selectors = answers(&prepared)
        .into_iter()
        .map(|((id, item, _), answer)| {
            let scoped = prepared.metadata().compiled.metadata().selectors[id].scoped;
            ((id, item, scoped.then_some(1)), answer)
        })
        .collect();
    let output = resolve(&prepared, &selectors, None, None);
    assert_eq!(marker(&output).as_deref(), Some("underline"));
    assert_eq!(
        value(&output, "text-decoration-line").as_deref(),
        Some("underline")
    );
}
