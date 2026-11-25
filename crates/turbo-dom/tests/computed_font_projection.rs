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
fn oracle() -> serde_json::Value {
    let source = include_bytes!("fixtures/font-used-159/font-used-probe.mjs");
    let value: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/font-used-159/FONT-USED-CHROME159.json"
    ))
    .unwrap();
    assert_eq!(value["browser"], "159.0.8154.6");
    assert_eq!(
        value["sourceSha256"],
        format!("{:x}", Sha256::digest(source))
    );
    value
}
fn project(
    style: &ComputedStyle,
) -> turbo_dom::rtdom::computed_css::cssom_serialization::SerializedCssom {
    use turbo_dom::rtdom::computed_css::cssom_serialization::{
        serialize_cssom_with_context, CssomSerializationContext, CssomSerializationLimits,
        TextDecorationSerialization,
    };
    serialize_cssom_with_context(
        &style.values.values,
        &style.public_keys,
        &CssomSerializationContext {
            text_decoration: TextDecorationSerialization::Declared,
            declared_text_decoration: style.declared_text_decoration.as_ref().map(Into::into),
            font_size_px: Some(style.values.font_size_px),
        },
        CssomSerializationLimits {
            retained_units: 32768,
            computation: limits(),
        },
    )
    .unwrap()
}
#[test]
fn absolute_and_relative_weights_use_typed_existing_grammar_against_actual_chrome159() {
    use turbo_dom::rtdom::computed_css::font_values::compute_font_weight;
    let control = oracle();
    for row in control["result"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row.get("invalidWeight").is_some())
    {
        assert_eq!(row["supports"], false);
        assert_eq!(row["authored"], "");
        assert_eq!(row["usedWeight"], "400");
    }
    for row in control["result"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row.get("computed").is_some())
    {
        let source = if let Some(relative) = row["relative"].as_str() {
            relative.to_owned()
        } else {
            row["weight"].as_str().unwrap().to_owned()
        };
        let parent = row["relative"]
            .as_str()
            .map(|_| text(&row["weight"].as_u64().unwrap().to_string()));
        let original = text(&source);
        let value = compute_font_weight(&original, parent.as_ref(), limits()).unwrap();
        assert_eq!(
            String::from_utf16(value.as_units()).unwrap(),
            row["computed"].as_str().unwrap()
        );
        assert_eq!(original, text(&source));
    }
}
#[test]
fn source_inline_and_parent_child_percentage_unitless_and_normal_inverse_match_chrome() {
    let control = oracle();
    for mode in ["stylesheet", "inline"] {
        for height in ["1.5", "150%", "normal"] {
            let parent_source = format!("font-size:12px;line-height:{height};font-weight:bold");
            let child_source = "font-size:20px;line-height:inherit;font-weight:inherit";
            let parent_model = model(&format!(
                ".x{{{}}}",
                if mode == "stylesheet" {
                    &parent_source
                } else {
                    ""
                }
            ));
            let child_model = model(&format!(
                ".x{{{}}}",
                if mode == "stylesheet" {
                    child_source
                } else {
                    ""
                }
            ));
            let parent_inline = (mode == "inline").then(|| text(&parent_source));
            let child_inline = (mode == "inline").then(|| text(child_source));
            let parent = resolve(
                &parent_model,
                &answers(&parent_model),
                parent_inline.as_ref(),
                None,
            );
            let child = resolve(
                &child_model,
                &answers(&child_model),
                child_inline.as_ref(),
                Some(&parent),
            );
            let stored = if height == "150%" { "18px" } else { height };
            assert_eq!(value(&parent, "line-height").as_deref(), Some(stored));
            assert_eq!(value(&child, "line-height").as_deref(), Some(stored));
            for (id, style) in [("parent", &parent), ("child", &child)] {
                let expected = control["result"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|row| {
                        row["mode"] == mode && row["id"] == id && row["lineHeightSource"] == height
                    })
                    .unwrap();
                let used = project(style);
                for (name, field) in [("line-height", "lineHeight"), ("font-weight", "fontWeight")]
                {
                    assert_eq!(
                        String::from_utf16(used.values[name].as_units()).unwrap(),
                        expected[field].as_str().unwrap(),
                        "{mode}:{height}:{id}:{name}"
                    );
                }
            }
            if let Some(raw) = parent_inline {
                assert_eq!(raw, text(&parent_source));
            }
            if let Some(raw) = child_inline {
                assert_eq!(raw, text(child_source));
            }
        }
    }
}
#[test]
fn font_used_projection_requires_only_actual_needed_context_and_checks_finite_boundaries() {
    use turbo_dom::rtdom::computed_css::font_values::{
        compute_font_weight, line_height_used, FontValueError,
    };
    assert_eq!(
        line_height_used(&text("1.5"), None, limits()),
        Err(FontValueError::RequiresFontSize)
    );
    for size in [f64::NAN, f64::INFINITY, -1.0] {
        assert_eq!(
            line_height_used(&text("1.5"), Some(size), limits()),
            Err(FontValueError::InvalidFontSize)
        );
    }
    for source in ["normal", "18px", "150%"] {
        assert_eq!(
            line_height_used(&text(source), Some(f64::NAN), limits()),
            Ok(None)
        );
    }
    for (source, size) in [("0", None), ("1.5", Some(0.0))] {
        assert_eq!(
            line_height_used(&text(source), size, limits()),
            Ok(Some(text("0px")))
        );
    }
    assert_eq!(
        line_height_used(&text("1e30"), Some(1e30), limits()),
        Ok(Some(text("3.40282e+38px")))
    );
    assert_eq!(
        line_height_used(&text("-1"), Some(12.0), limits()),
        Err(FontValueError::InvalidLineHeight)
    );
    assert_eq!(
        compute_font_weight(&text("bolder"), None, limits()),
        Err(FontValueError::RequiresParentFontWeight)
    );
    assert_eq!(
        compute_font_weight(&text("lighter"), Some(&text("bolder")), limits()),
        Err(FontValueError::RequiresParentFontWeight)
    );
    assert_eq!(
        compute_font_weight(&text("0"), None, limits()),
        Err(FontValueError::InvalidFontWeight)
    );
    assert_eq!(
        compute_font_weight(&text("1001"), None, limits()),
        Err(FontValueError::InvalidFontWeight)
    );
}
#[test]
fn font_stage_preserves_typed_source_output_and_retained_limits() {
    use turbo_dom::rtdom::computed_css::{
        font_values::{compute_font_weight, line_height_used, FontValueError},
        ComputedCssError,
    };
    let mut narrow = limits();
    narrow.max_nesting = 0;
    assert_eq!(
        compute_font_weight(&text("bold"), None, narrow),
        Err(FontValueError::Computation(ComputedCssError::NestingLimit))
    );
    let mut narrow = limits();
    narrow.max_output_units = 1;
    assert_eq!(
        line_height_used(&text("1"), Some(12.0), narrow),
        Err(FontValueError::Computation(ComputedCssError::ResourceLimit))
    );
    let mut narrow = limits();
    narrow.max_output_units = 4;
    assert_eq!(
        line_height_used(&text("1.5"), Some(12.0), narrow),
        Ok(Some(text("18px")))
    );
    assert_eq!(
        compute_font_weight(&text("400"), Some(&text("unneeded-parent")), limits()),
        Ok(text("400"))
    );
    let empty = model(".x{}");
    let style = resolve(&empty, &answers(&empty), None, None);
    assert!(!style.values.values.contains_key("font-weight"));
    assert!(!project(&style).values.contains_key("font-weight"));
}

#[test]
fn used_projection_charges_final_pixels_not_both_computed_and_used_strings() {
    use turbo_dom::rtdom::computed_css::cssom_serialization::{
        serialize_cssom_with_context, CssomSerializationContext, CssomSerializationError,
        CssomSerializationLimits, TextDecorationSerialization,
    };
    let ordinary = BTreeMap::from([("line-height".to_owned(), text("1.5"))]);
    let public = ordinary.keys().cloned().collect();
    let context = CssomSerializationContext {
        text_decoration: TextDecorationSerialization::Declared,
        declared_text_decoration: None,
        font_size_px: Some(12.0),
    };
    let exact = 2 * "line-height".len() + "18px".len();
    let output = serialize_cssom_with_context(
        &ordinary,
        &public,
        &context,
        CssomSerializationLimits {
            retained_units: exact,
            computation: limits(),
        },
    )
    .unwrap();
    assert_eq!(output.values["line-height"], text("18px"));
    assert_eq!(ordinary["line-height"], text("1.5"));
    assert_eq!(
        serialize_cssom_with_context(
            &ordinary,
            &public,
            &context,
            CssomSerializationLimits {
                retained_units: exact - 1,
                computation: limits()
            }
        ),
        Err(CssomSerializationError::ResourceLimit)
    );
}

#[test]
fn large_finite_fonts_clamp_before_inheritance_and_em_bases_against_actual_chrome159() {
    let probe = include_bytes!("fixtures/font-large-159/font-large-probe.mjs");
    let control: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/font-large-159/FONT-LARGE-CHROME159.json"
    ))
    .unwrap();
    assert_eq!(control["browser"], "159.0.8154.6");
    assert_eq!(
        control["version"]["revision"],
        "@98a2592026af8d673b9fa83b626ac68bdf4ae78a"
    );
    assert_eq!(
        control["sourceSha256"],
        format!("{:x}", Sha256::digest(probe))
    );
    for row in control["result"].as_array().unwrap() {
        let size = row["size"].as_str().unwrap();
        let height = row["height"].as_str().unwrap();
        let source = format!("font-size:{size};line-height:{height};width:1em");
        let child_source = "font-size:inherit;line-height:inherit;display:block;width:1em";
        let inline = row["mode"] == "inline";
        let parent_model = model(&format!(".x{{{}}}", if inline { "" } else { &source }));
        let child_model = model(&format!(".x{{{}}}", if inline { "" } else { child_source }));
        let parent_inline = inline.then(|| text(&source));
        let child_inline = inline.then(|| text(child_source));
        let parent = resolve(
            &parent_model,
            &answers(&parent_model),
            parent_inline.as_ref(),
            None,
        );
        let child = resolve(
            &child_model,
            &answers(&child_model),
            child_inline.as_ref(),
            Some(&parent),
        );
        for (style, font, line, width) in [
            (&parent, "fontSize", "lineHeight", "width"),
            (&child, "childFontSize", "childLineHeight", "childWidth"),
        ] {
            let used = project(style);
            assert_eq!(
                value(style, "font-size").unwrap(),
                row[font].as_str().unwrap(),
                "{size}:{height}:{font}"
            );
            assert_eq!(
                String::from_utf16(used.values["line-height"].as_units()).unwrap(),
                row[line].as_str().unwrap(),
                "{size}:{height}:{line}"
            );
            assert_eq!(
                String::from_utf16(used.values["width"].as_units()).unwrap(),
                row[width].as_str().unwrap(),
                "{size}:{height}:{width}"
            );
        }
        if let Some(raw) = parent_inline {
            assert_eq!(raw, text(&source));
        }
    }
}
#[test]
fn canonical_font_numeric_finalizers_preserve_finite_saturation_and_invalid_arithmetic_errors() {
    use turbo_dom::rtdom::computed_css::font_values::{
        finalize_font_size_px, font_used_pixels, FontValueError, MAX_CSS_FONT_SIZE_PX,
    };
    assert_eq!(finalize_font_size_px(9999.0), Ok(9999.0));
    assert_eq!(finalize_font_size_px(10001.0), Ok(MAX_CSS_FONT_SIZE_PX));
    assert_eq!(finalize_font_size_px(f64::MAX), Ok(MAX_CSS_FONT_SIZE_PX));
    for invalid in [f64::NAN, f64::INFINITY, -1.0] {
        assert_eq!(
            finalize_font_size_px(invalid),
            Err(FontValueError::InvalidFontSize)
        );
    }
    for finite in [1e40, f64::MAX] {
        assert_eq!(font_used_pixels(finite, 64), Ok(text("3.40282e+38px")));
    }
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            font_used_pixels(invalid, 64),
            Err(FontValueError::NonFiniteUsedLineHeight)
        );
    }
    assert_eq!(
        font_used_pixels(-1.0, 64),
        Err(FontValueError::InvalidLineHeight)
    );
}

#[test]
fn explicit_initial_preferences_are_preserved_below_css_font_cap_without_public_default_materialization(
) {
    use turbo_dom::rtdom::computed_css::computed_values::compute_values;
    for (preference, effective) in [("13.5px", 13.5), ("20000px", 10000.0)] {
        let context = ValueContext {
            initial: InitialContext {
                font_size: preference,
                font_family: "CustomActualFamily",
                color: "rgb(1, 2, 3)",
            },
            unit_bases: &[],
            font_size_unit_bases: &[],
            is_root: true,
        };
        let result = compute_values(
            None,
            &BTreeMap::new(),
            &context,
            turbo_dom::rtdom::computed_css::computed_values::ValueLimits {
                computation: limits(),
                retained_units: 32768,
            },
        )
        .unwrap();
        assert_eq!(result.font_size_px, effective);
        assert_eq!(result.root_font_size_px, effective);
        assert!(result.values.is_empty());
    }
}
