#![cfg(feature = "computed-style-grammar")]
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use turbo_dom::rtdom::computed_css::{
    cssom_serialization::{
        serialize_cssom, CssomSerializationError as Error, CssomSerializationLimits,
        SerializedCssom,
    },
    CssText,
};
fn computation() -> turbo_dom::rtdom::computed_css::ComputationLimits {
    turbo_dom::rtdom::computed_css::ComputationLimits {
        max_nesting: 32,
        max_output_units: 65536,
        max_total_units: 65536,
    }
}
fn text(value: &str) -> CssText {
    CssText::from_units(value.encode_utf16().collect())
}
fn keys(values: &BTreeMap<String, CssText>) -> BTreeSet<String> {
    values.keys().cloned().collect()
}
fn serialize(values: &BTreeMap<String, CssText>, public: &BTreeSet<String>) -> SerializedCssom {
    serialize_cssom(
        values,
        public,
        CssomSerializationLimits {
            computation: computation(),
            retained_units: 65536,
        },
    )
    .unwrap()
}
fn units(value: &SerializedCssom) -> usize {
    value
        .values
        .iter()
        .map(|(name, value)| name.encode_utf16().count() + value.as_units().len())
        .sum::<usize>()
        + value
            .public_keys
            .iter()
            .map(|name| name.encode_utf16().count())
            .sum::<usize>()
}
#[test]
fn final_margin_and_font_maps_reuse_the_frozen_canonical_chrome156_serializer() {
    let control: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/css_shorthand_chrome156.json")).unwrap();
    assert_eq!(control["browser"], "Chrome/156.0.8078.4");
    for (field, source) in [
        (
            "probeSha256",
            include_bytes!("fixtures/css-shorthand-probe.js").as_slice(),
        ),
        (
            "casesSha256",
            include_bytes!("fixtures/css-shorthand-cases.json").as_slice(),
        ),
    ] {
        assert_eq!(control[field], format!("{:x}", Sha256::digest(source)));
    }
    let rows = control["result"][0]["result"]["results"]
        .as_array()
        .unwrap();
    for (id, property) in [("margin|1px", "margin"), ("font|12px Arial", "font")] {
        let row = rows.iter().find(|row| row["id"] == id).unwrap();
        let ordinary = row["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| {
                (
                    item[0].as_str().unwrap().to_owned(),
                    text(item[1].as_str().unwrap()),
                )
            })
            .collect();
        let output = serialize(&ordinary, &keys(&ordinary));
        assert_eq!(
            String::from_utf16(output.values[property].as_units()).unwrap(),
            row["shorthands"][property].as_str().unwrap()
        );
        assert!(output.public_keys.contains(property));
    }
}
#[test]
fn missing_or_nonpublic_longhands_never_complete_a_public_shorthand() {
    let ordinary: BTreeMap<_, _> = [
        ("margin-top", "1px"),
        ("margin-right", "2px"),
        ("margin-bottom", "1px"),
        ("margin-left", "2px"),
    ]
    .into_iter()
    .map(|(name, value)| (name.into(), text(value)))
    .collect();
    let public = BTreeSet::from(["margin-top".into()]);
    let output = serialize(&ordinary, &public);
    assert!(!output.values.contains_key("margin"));
    assert_eq!(output.public_keys, public);
    let mut partial = ordinary.clone();
    partial.remove("margin-left");
    assert!(!serialize(&partial, &keys(&ordinary))
        .values
        .contains_key("margin"));
    let mut unrepresentable = ordinary.clone();
    unrepresentable.insert("margin-top".into(), text("inherit"));
    assert!(!serialize(&unrepresentable, &keys(&ordinary))
        .values
        .contains_key("margin"));
}
#[test]
fn public_aliases_clone_exact_units_and_unknown_values_stay_raw() {
    let raw = CssText::from_units(vec![0xd800, 0x61, 0xdc00]);
    let ordinary = BTreeMap::from([
        ("overflow-wrap".into(), raw.clone()),
        ("unknown-matched-property".into(), raw.clone()),
    ]);
    let output = serialize(&ordinary, &keys(&ordinary));
    assert_eq!(output.values["word-wrap"], raw);
    assert!(output.public_keys.contains("word-wrap"));
    assert_eq!(
        output.values["unknown-matched-property"],
        ordinary["unknown-matched-property"]
    );
    assert!(!output.values.contains_key("unknown-unmatched-property"));
    let hidden = serialize(&ordinary, &BTreeSet::new());
    assert!(!hidden.values.contains_key("word-wrap"));
}
#[test]
fn shorthand_scalar_conversion_is_checked_and_custom_maps_are_a_separate_boundary() {
    let mut ordinary: BTreeMap<_, _> =
        ["margin-top", "margin-right", "margin-bottom", "margin-left"]
            .into_iter()
            .map(|name| (name.into(), text("1px")))
            .collect();
    ordinary.insert("margin-top".into(), CssText::from_units(vec![0xd800]));
    assert_eq!(
        serialize_cssom(
            &ordinary,
            &keys(&ordinary),
            CssomSerializationLimits {
                computation: computation(),
                retained_units: 4096
            }
        ),
        Err(Error::NonScalarLonghand("margin-top".into()))
    );
    let raw = CssText::from_units(vec![0xd800]);
    let custom = BTreeMap::from([("--exact".into(), raw.clone())]);
    assert_eq!(
        serialize_cssom(
            &custom,
            &BTreeSet::new(),
            CssomSerializationLimits {
                computation: computation(),
                retained_units: 4096
            }
        ),
        Err(Error::CustomPropertyInOrdinary("--exact".into()))
    );
    assert_eq!(custom["--exact"], raw);
    assert_eq!(
        serialize_cssom(
            &BTreeMap::new(),
            &BTreeSet::from(["--exact".into()]),
            CssomSerializationLimits {
                computation: computation(),
                retained_units: 4096
            }
        ),
        Err(Error::CustomPropertyInOrdinary("--exact".into()))
    );
}
#[test]
fn output_payload_names_and_public_keys_share_one_explicit_retained_quota() {
    let ordinary = BTreeMap::from([("overflow-wrap".into(), text("break-word"))]);
    let public = keys(&ordinary);
    let output = serialize(&ordinary, &public);
    let exact = units(&output);
    assert_eq!(
        serialize_cssom(
            &ordinary,
            &public,
            CssomSerializationLimits {
                computation: computation(),
                retained_units: exact
            }
        )
        .unwrap(),
        output
    );
    for retained_units in [0, exact - 1] {
        assert_eq!(
            serialize_cssom(
                &ordinary,
                &public,
                CssomSerializationLimits {
                    computation: computation(),
                    retained_units
                }
            ),
            Err(Error::ResourceLimit)
        );
    }
}
#[test]
fn conflicting_derived_alias_is_an_explicit_boundary_error_not_a_silent_overwrite() {
    let ordinary = BTreeMap::from([
        ("overflow-wrap".into(), text("break-word")),
        ("word-wrap".into(), text("normal")),
    ]);
    assert_eq!(
        serialize_cssom(
            &ordinary,
            &keys(&ordinary),
            CssomSerializationLimits {
                computation: computation(),
                retained_units: 4096
            }
        ),
        Err(Error::ConflictingDerivedValue("word-wrap".into()))
    );
}

#[test]
fn authored_longhand_projection_and_word_wrap_alias_match_actual_chrome159() {
    let probe = include_bytes!("fixtures/cssom-projection-159/cssom-projection-probe.mjs");
    let control: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/cssom-projection-159/CSSOM-PROJECTION-ORACLE.json"
    ))
    .unwrap();
    assert_eq!(control["browser"], "159.0.8154.6");
    assert_eq!(
        control["sourceSha256"],
        format!("{:x}", Sha256::digest(probe))
    );
    let ordinary: BTreeMap<_, _> = [
        ("margin-top", "1px"),
        ("margin-right", "2px"),
        ("margin-bottom", "1px"),
        ("margin-left", "2px"),
        ("overflow-wrap", "break-word"),
    ]
    .into_iter()
    .map(|(name, value)| (name.into(), text(value)))
    .collect();
    let output = serialize(&ordinary, &keys(&ordinary));
    for (name, field) in [
        ("margin", "margin"),
        ("word-wrap", "alias"),
        ("overflow-wrap", "canonical"),
    ] {
        assert_eq!(
            String::from_utf16(output.values[name].as_units()).unwrap(),
            control["result"][field].as_str().unwrap()
        );
    }
    assert_eq!(control["result"]["wordWrap"], control["result"]["alias"]);
    assert!(!output.values.contains_key("not-a-property"));
    assert_eq!(control["result"]["unknown"], "");
}

#[test]
fn existing_equal_alias_keeps_input_values_and_adds_only_missing_public_provenance() {
    let ordinary = BTreeMap::from([
        ("overflow-wrap".into(), text("break-word")),
        ("word-wrap".into(), text("break-word")),
    ]);
    let public = BTreeSet::from(["overflow-wrap".into()]);
    let output = serialize(&ordinary, &public);
    assert_eq!(output.values, ordinary);
    assert!(output.public_keys.contains("word-wrap"));
}

#[test]
fn shorthand_output_payload_obeys_quota_after_exact_input_projection_is_retained() {
    let ordinary: BTreeMap<_, _> = ["margin-top", "margin-right", "margin-bottom", "margin-left"]
        .into_iter()
        .map(|name| (name.to_owned(), text("1px")))
        .collect();
    let public = keys(&ordinary);
    let input_units = ordinary
        .iter()
        .map(|(name, value)| name.encode_utf16().count() + value.as_units().len())
        .sum::<usize>()
        + public
            .iter()
            .map(|name| name.encode_utf16().count())
            .sum::<usize>();
    assert_eq!(
        serialize_cssom(
            &ordinary,
            &public,
            CssomSerializationLimits {
                computation: computation(),
                retained_units: input_units
            }
        ),
        Err(Error::ResourceLimit)
    );
    assert_eq!(
        String::from_utf16(serialize(&ordinary, &public).values["margin"].as_units()).unwrap(),
        "1px"
    );
}

fn decoration_marker(
    value: CssText,
    origin: turbo_dom::rtdom::computed_css::prepared::CascadeOrigin,
) -> turbo_dom::rtdom::computed_css::resolve::DeclaredTextDecoration {
    turbo_dom::rtdom::computed_css::resolve::DeclaredTextDecoration {
        value,
        origin,
        important: false,
        inline: true,
        order: (usize::MAX, usize::MAX),
    }
}
#[test]
fn selected_declared_policy_matches_original_fifteen_case_api_without_reconstructing_cascade() {
    use turbo_dom::rtdom::computed_css::{
        cssom_serialization::{
            serialize_cssom_with_context, CssomSerializationContext, TextDecorationSerialization,
        },
        prepared::CascadeOrigin,
    };
    let raw = include_str!("fixtures/npm-cssom-compat-159/LEGACY-PRIVATE-RAW-RESULT.json");
    assert_eq!(
        format!("{:x}", Sha256::digest(raw.as_bytes())),
        "5ddf224505b78eb06ceb9225ea22c52acda440c599d153a77bb33357c06bfa83"
    );
    let legacy: serde_json::Value = serde_json::from_str(raw).unwrap();
    assert_eq!(
        legacy["artifactSha256"],
        "1b09f73d27559785020070d0e38691f09a5d2d021a518f5e93564f81deefcdf6"
    );
    let row = legacy["results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == "text-decoration-declared")
        .unwrap();
    let expected = row["legacy"]["values"][0]["computed"]["getPropertyValue"]["value"]
        .as_str()
        .unwrap();
    let marker = decoration_marker(text(expected), CascadeOrigin::Author);
    let ordinary: BTreeMap<_, _> = [
        ("text-decoration-line", "underline"),
        ("text-decoration-style", "dotted"),
        ("text-decoration-color", "rgb(255, 0, 0)"),
        ("text-decoration-thickness", "auto"),
        ("text-decoration", "underline dotted rgb(255, 0, 0)"),
    ]
    .into_iter()
    .map(|(name, value)| (name.to_owned(), text(value)))
    .collect();
    let before = ordinary.clone();
    let context = CssomSerializationContext {
        font_size_px: None,
        text_decoration: TextDecorationSerialization::Declared,
        declared_text_decoration: Some((&marker).into()),
    };
    let output = serialize_cssom_with_context(
        &ordinary,
        &keys(&ordinary),
        &context,
        CssomSerializationLimits {
            computation: computation(),
            retained_units: 4096,
        },
    )
    .unwrap();
    assert_eq!(
        String::from_utf16(output.values["text-decoration"].as_units()).unwrap(),
        expected
    );
    assert_eq!(ordinary, before);
    assert!(output.public_keys.contains("text-decoration"));
}
#[test]
fn declared_policy_does_not_publish_ua_or_infer_marker_from_public_longhands() {
    use turbo_dom::rtdom::computed_css::{
        cssom_serialization::{
            serialize_cssom_with_context, CssomSerializationContext, TextDecorationSerialization,
        },
        prepared::CascadeOrigin,
    };
    let ordinary: BTreeMap<_, _> = [
        ("text-decoration-line", "underline"),
        ("text-decoration-style", "wavy"),
        ("text-decoration-color", "rgb(0, 0, 255)"),
        ("text-decoration-thickness", "auto"),
    ]
    .into_iter()
    .map(|(name, value)| (name.to_owned(), text(value)))
    .collect();
    let ua = decoration_marker(text("underline"), CascadeOrigin::UserAgent);
    for marker in [None, Some(&ua)] {
        let context = CssomSerializationContext {
            font_size_px: None,
            text_decoration: TextDecorationSerialization::Declared,
            declared_text_decoration: marker.map(Into::into),
        };
        let output = serialize_cssom_with_context(
            &ordinary,
            &keys(&ordinary),
            &context,
            CssomSerializationLimits {
                computation: computation(),
                retained_units: 4096,
            },
        )
        .unwrap();
        assert!(!output.values.contains_key("text-decoration"));
        assert!(!output.public_keys.contains("text-decoration"));
        assert_eq!(
            output.values["text-decoration-line"],
            ordinary["text-decoration-line"]
        );
    }
    assert!(serialize(&ordinary, &keys(&ordinary))
        .values
        .contains_key("text-decoration"));
}
#[test]
fn declared_pending_utf16_has_exact_output_quota_and_computed_policy_ignores_marker() {
    use turbo_dom::rtdom::computed_css::{
        cssom_serialization::{
            serialize_cssom_with_context, CssomSerializationContext, TextDecorationSerialization,
        },
        prepared::CascadeOrigin,
    };
    let marker = decoration_marker(
        CssText::from_units(vec![0xd800, 0x61, 0xdc00]),
        CascadeOrigin::User,
    );
    let context = CssomSerializationContext {
        font_size_px: None,
        text_decoration: TextDecorationSerialization::Declared,
        declared_text_decoration: Some((&marker).into()),
    };
    let output = serialize_cssom_with_context(
        &BTreeMap::new(),
        &BTreeSet::new(),
        &context,
        CssomSerializationLimits {
            computation: computation(),
            retained_units: 4096,
        },
    )
    .unwrap();
    assert_eq!(output.values["text-decoration"], marker.value);
    let exact = units(&output);
    assert_eq!(
        serialize_cssom_with_context(
            &BTreeMap::new(),
            &BTreeSet::new(),
            &context,
            CssomSerializationLimits {
                computation: computation(),
                retained_units: exact
            }
        )
        .unwrap(),
        output
    );
    assert_eq!(
        serialize_cssom_with_context(
            &BTreeMap::new(),
            &BTreeSet::new(),
            &context,
            CssomSerializationLimits {
                computation: computation(),
                retained_units: exact - 1
            }
        ),
        Err(Error::ResourceLimit)
    );
    let context = CssomSerializationContext {
        font_size_px: None,
        text_decoration: TextDecorationSerialization::Computed,
        declared_text_decoration: Some((&marker).into()),
    };
    assert!(serialize_cssom_with_context(
        &BTreeMap::new(),
        &BTreeSet::new(),
        &context,
        CssomSerializationLimits {
            computation: computation(),
            retained_units: 0
        }
    )
    .unwrap()
    .values
    .is_empty());
}
#[test]
fn computed_background_and_font_share_real_chrome159_complete_longhand_controls() {
    use turbo_dom::rtdom::css_shorthand::{expand_css, longhands};
    let probe = include_bytes!("fixtures/computed-shorthand-159/computed-shorthand-probe.mjs");
    let control: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/computed-shorthand-159/COMPUTED-SHORTHAND-CHROME159.json"
    ))
    .unwrap();
    assert_eq!(control["browser"], "159.0.8154.6");
    assert_eq!(
        control["sourceSha256"],
        format!("{:x}", Sha256::digest(probe))
    );
    for row in control["result"].as_array().unwrap() {
        let id = row["id"].as_str().unwrap();
        let property = if id.starts_with("background-") {
            "background"
        } else {
            "font"
        };
        // Font shorthand explicitly sets its reset longhands. Populate those
        // admitted values with the canonical shared expander, then replace the
        // measured computed/used fields from the actual browser control.
        let mut ordinary: BTreeMap<String, CssText> = if property == "font" {
            let source = row["css"].as_str().unwrap().strip_prefix("font:").unwrap();
            expand_css("font", source)
                .unwrap()
                .into_iter()
                .map(|value| (value.name.to_owned(), text(&value.value)))
                .collect()
        } else {
            BTreeMap::new()
        };
        for name in longhands(property).unwrap() {
            if let Some(value) = row["computed"][*name].as_str() {
                ordinary.insert((*name).to_owned(), text(value));
            }
        }
        let output = serialize(&ordinary, &keys(&ordinary));
        assert_eq!(
            String::from_utf16(output.values[property].as_units()).unwrap(),
            row["computed"][property].as_str().unwrap(),
            "{id}"
        );
    }
}
