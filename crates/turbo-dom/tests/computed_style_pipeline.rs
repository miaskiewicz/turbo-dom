#![cfg(feature = "computed-style-grammar")]
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
    rule_matches::{matching_rules, MatchError, SelectorAnswers, TargetFacts},
    target_scope::{Boundary, BoundaryAnswers},
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
fn actual_pipeline_resolves_selected_custom_values_pending_shorthands_and_em_once() {
    let prepared =
        model(".x{--ink:red;--space:1px 2px;color:var(--ink);margin:var(--space);width:20em}");
    let result = resolve(&prepared, &answers(&prepared), None, None);
    assert_eq!(value(&result, "color"), Some("rgb(255, 0, 0)".to_owned()));
    assert_eq!(value(&result, "margin-left"), Some("2px".to_owned()));
    assert_eq!(value(&result, "width"), Some("320px".to_owned()));
    assert_eq!(result.custom.get("--space"), Some(&text("1px 2px")));
}
#[test]
fn media_supports_layers_inline_and_important_share_one_cascade() {
    let prepared=model("@layer first,second;@layer first{.x{color:red!important}}@layer second{.x{color:blue!important}}@media(min-width:700px){@supports(display:grid){.x{width:2em}}}");
    let plain = resolve(&prepared, &answers(&prepared), None, None);
    assert_eq!(value(&plain, "color"), Some("rgb(255, 0, 0)".to_owned()));
    assert_eq!(value(&plain, "width"), Some("32px".to_owned()));
    let inline = text("color:green!important");
    let result = resolve(&prepared, &answers(&prepared), Some(&inline), None);
    assert_eq!(value(&result, "color"), Some("rgb(0, 128, 0)".to_owned()));
    let rollback = text("color:revert-layer!important");
    let restored = resolve(&prepared, &answers(&prepared), Some(&rollback), None);
    assert_eq!(value(&restored, "color"), Some("rgb(255, 0, 0)".to_owned()));
}
#[test]
fn invalid_computed_winner_unsets_and_inherits_computed_tokens_without_recomputation() {
    let parent = model(".x{font-size:10px;width:20em;color:red}");
    let parent = resolve(&parent, &answers(&parent), None, None);
    let child = model(".x{font-size:20px;width:inherit;color:blue;color:var(--missing)}");
    let result = resolve(&child, &answers(&child), None, Some(&parent));
    assert_eq!(value(&result, "width"), Some("200px".to_owned()));
    assert_eq!(value(&result, "color"), Some("rgb(255, 0, 0)".to_owned()));
    assert_eq!(result.values.font_size_px, 20.0);
}
#[test]
fn missing_matching_capabilities_are_typed_and_retained_results_own_exact_css_only() {
    let prepared = model(".x{color:red}");
    let empty = SelectorAnswers::new();
    let boundaries_empty = BoundaryAnswers::new();
    let facts = TargetFacts {
        containers: None,
        path: &[2, 1],
        implicit_scope_root: 1,
        pseudo: None,
        selectors: &empty,
        routes: None,
        boundaries: &boundaries_empty,
    };
    assert!(matches!(
        matching_rules(&prepared, &facts, 64),
        Err(MatchError::MissingSelectorAnswer)
    ));
    let answers = answers(&prepared);
    let facts = TargetFacts {
        selectors: &answers,
        ..facts
    };
    let matched = matching_rules(&prepared, &facts, 64).unwrap();
    let bytes = prepared.retained_payload_bytes();
    assert!(bytes > 0);
    drop(prepared);
    assert_eq!(matched.prepared().retained_payload_bytes(), bytes);
    let candidates = turbo_dom::rtdom::computed_css::candidate_set::collect_candidates(&matched);
    assert_eq!(candidates.ordinary["color"][0].value, &text("red"));
}
#[test]
fn scope_limits_and_proximity_use_exhaustive_original_node_answers() {
    let prepared = model("@scope(.root) to(.limit){.x{color:blue}}");
    let domain = AnswerDomain::new(vec![2, 1]).unwrap();
    let mut boundaries: BoundaryAnswers = BTreeMap::from([
        (
            (0, Boundary::Start, 1),
            MechanicalAnswer::new(domain.clone(), vec![1]).unwrap(),
        ),
        (
            (0, Boundary::End, 1),
            MechanicalAnswer::new(domain.clone(), vec![]).unwrap(),
        ),
    ]);
    let selectors = answers(&prepared)
        .into_iter()
        .map(|((id, item, _), answer)| ((id, item, Some(1)), answer))
        .collect();
    let facts = TargetFacts {
        containers: None,
        path: &[2, 1],
        implicit_scope_root: 1,
        pseudo: None,
        selectors: &selectors,
        routes: None,
        boundaries: &boundaries,
    };
    let matched = matching_rules(&prepared, &facts, 64).unwrap();
    assert_eq!(matched.rules()[0].proximity(), 1);
    boundaries.insert(
        (0, Boundary::End, 1),
        MechanicalAnswer::new(domain, vec![2]).unwrap(),
    );
    let facts = TargetFacts {
        containers: None,
        path: &[2, 1],
        implicit_scope_root: 1,
        pseudo: None,
        selectors: &selectors,
        routes: None,
        boundaries: &boundaries,
    };
    assert!(matching_rules(&prepared, &facts, 64)
        .unwrap()
        .rules()
        .is_empty());
}

#[test]
fn geometry_projection_masks_only_conditional_color_and_keeps_original_css_ownership() {
    let prepared = model(".x{color:red;width:10px}@container(min-width:300px){.x{color:blue}}");
    let plan = prepared.container_geometry_plan().unwrap();
    let geometry = plan.styles_for_measurement();
    assert!(geometry.is_geometry_only());
    assert!(!prepared.is_geometry_only());
    assert!(std::ptr::eq(prepared.metadata(), geometry.metadata()));
    let result = resolve(geometry, &answers(geometry), None, None);
    assert_eq!(value(&result, "color"), Some("rgb(255, 0, 0)".into()));
    assert_eq!(value(&result, "width"), Some("10px".into()));
    let facts = TargetFacts {
        path: &[2, 1],
        implicit_scope_root: 1,
        pseudo: None,
        selectors: &answers(&prepared),
        boundaries: &BTreeMap::new(),
        routes: None,
        containers: None,
    };
    assert!(matches!(
        matching_rules(&prepared, &facts, 4096),
        Err(MatchError::RequiresContainer(_))
    ));
}

#[test]
fn geometry_projection_refuses_custom_and_layout_properties_and_unknown_definitions() {
    use turbo_dom::rtdom::computed_css::prepared::ContainerGeometryError;
    for (css, expected) in [
        ("@container(min-width:1px){.x{width:10px}}", "width"),
        (
            "@container(min-width:1px){.x{--size:10px;color:blue}}",
            "--size",
        ),
    ] {
        assert!(
            matches!(model(css).container_geometry_plan(),Err(ContainerGeometryError::RequiresLayoutDependency(name)) if name==expected)
        );
    }
    for (css, expected) in [
        ("@container(min-width:1px){@property --x{syntax:'<color>';inherits:false;initial-value:red}.x{color:blue}}","property"),
        ("@container(min-width:1px){@counter-style marker{system:cyclic;symbols:'x'}.x{color:blue}}","counter-style"),
        ("@font-face{font-family:Example;src:url(example.woff2)}.x{color:red}","font-face"),
        ("@future; .x{color:red}","future"),
    ] {
        assert!(matches!(model(css).container_geometry_plan(),Err(ContainerGeometryError::RequiresDefinition(name)) if name==expected));
    }
    assert!(model("@supports(display:invented-display){@font-face{font-family:Example;src:url(example.woff2)}}.x{color:red}").container_geometry_plan().is_ok());
}

#[test]
fn geometry_projection_preserves_container_layer_registration_and_outside_width_winners() {
    let prepared = model(
        "@container(min-width:1000px){@layer b,a;}@layer a{.x{width:10px}}@layer b{.x{width:20px}}",
    );
    let plan = prepared.container_geometry_plan().unwrap();
    assert!(std::ptr::eq(
        prepared.metadata(),
        plan.styles_for_measurement().metadata()
    ));
    let normal = resolve(&prepared, &answers(&prepared), None, None);
    let geometry = resolve(
        plan.styles_for_measurement(),
        &answers(&prepared),
        None,
        None,
    );
    assert_eq!(value(&normal, "width"), Some("10px".into()));
    assert_eq!(value(&geometry, "width"), value(&normal, "width"));
}

fn resolve_layer_oracle(
    prepared: &PreparedStyles,
    answers: &SelectorAnswers,
    inline: Option<&CssText>,
    parent: Option<&ComputedStyle>,
    containers: Option<turbo_dom::rtdom::computed_css::rule_matches::ContainerMatchContext<'_>>,
) -> ComputedStyle {
    let empty = BTreeMap::new();
    let facts = TargetFacts {
        containers,
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
fn layer_oracle_answers(prepared: &PreparedStyles) -> SelectorAnswers {
    let tree = turbo_dom::rtdom::Tree::parse("<div id=container><div id=target></div></div>");
    let target = tree.query_selector("#target").unwrap();
    let domain = AnswerDomain::new(vec![2]).unwrap();
    prepared
        .metadata()
        .compiled
        .metadata()
        .selectors
        .iter()
        .enumerate()
        .map(|(id, selector)| {
            assert_eq!(selector.compiled.as_ref().unwrap().selectors().len(), 1);
            let source = String::from_utf16(selector.source.as_units()).unwrap();
            let matched = tree.matches_checked(target, &source).unwrap();
            (
                (id, 0, None),
                MechanicalAnswer::new(domain.clone(), if matched { vec![2] } else { vec![] })
                    .unwrap(),
            )
        })
        .collect()
}
#[test]
fn container_layers_follow_source_hashed_chrome159_at_both_real_container_sizes() {
    use sha2::{Digest, Sha256};
    use turbo_dom::rtdom::computed_css::{
        container::{
            ContainerAncestor, ContainerMeasurements, InlineAxis, PartialContentSize, SizeKind,
        },
        prepared::ContainerGeometryError,
        rule_matches::ContainerMatchContext,
    };
    let control: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/css_container_layers_chrome159.json")).unwrap();
    assert_eq!(control["version"], "159.0.8154.6");
    assert_eq!(
        control["sourceSha256"],
        format!(
            "{:x}",
            Sha256::digest(include_bytes!("fixtures/css-container-layers-oracle.mjs"))
        )
    );
    for row in control["results"].as_array().unwrap() {
        let css = row["css"].as_str().unwrap();
        let prepared = model(css);
        let answers = layer_oracle_answers(&prepared);
        let lengths = LengthContext { unit_bases: &[] };
        let ancestors = [ContainerAncestor {
            id: 1,
            names: &[],
            kind: SizeKind::InlineSize,
            inline_axis: InlineAxis::Horizontal,
            scroll_state: false,
            size: None,
            length_context: &lengths,
        }];
        let measurements = ContainerMeasurements::from([(
            1,
            PartialContentSize {
                width_px: Some(row["actualContainerWidth"].as_f64().unwrap()),
                height_px: None,
            },
        )]);
        let context = Some(ContainerMatchContext {
            ancestors: &ancestors,
            measurements: Some(&measurements),
            limits: limits(),
        });
        let computed = resolve_layer_oracle(&prepared, &answers, None, None, context);
        assert_eq!(
            value(&computed, "width").as_deref(),
            row["targetWidth"].as_str(),
            "{} at {}",
            row["id"],
            row["containerWidth"]
        );
        if row["id"] == "container-declaration" {
            if row["targetHeight"] == "30px" {
                assert_eq!(value(&computed, "height").as_deref(), Some("30px"));
            } else {
                assert_eq!(value(&computed, "height"), None);
            }
            assert!(
                matches!(prepared.container_geometry_plan(),Err(ContainerGeometryError::RequiresLayoutDependency(name))if name=="height")
            );
        } else {
            let plan = prepared.container_geometry_plan().unwrap();
            let projected = resolve(plan.styles_for_measurement(), &answers, None, None);
            assert_eq!(value(&projected, "width"), value(&computed, "width"));
        }
    }
}
