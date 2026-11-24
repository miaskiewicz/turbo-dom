//! Raw custom token computation against the explicitly versioned Chrome159 oracle.
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use turbo_dom::rtdom::computed_css::{
    compute_custom_properties, declarations, ComputationLimits, CssText,
};

const PROBE: &str = include_str!("fixtures/computed-css-variables-probe.mjs");
const CASES: &str = include_str!("fixtures/computed-css-variables-cases.json");
const CONTROL: &str = include_str!("fixtures/computed_css_variables_chrome159.json");

fn units(value: &Value) -> Vec<u16> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|unit| u16::try_from(unit.as_u64().unwrap()).unwrap())
        .collect()
}

fn limits() -> ComputationLimits {
    ComputationLimits {
        max_nesting: 64,
        max_output_units: 4096,
        max_total_units: 16384,
    }
}

fn winners(value: &Value) -> BTreeMap<String, CssText> {
    declarations(&units(value))
        .into_iter()
        .map(|declaration| (declaration.name, declaration.value))
        .collect()
}

fn own_winners(case: &Value) -> BTreeMap<String, CssText> {
    let Some(entries) = case.get("ownValues") else {
        return winners(&case["ownUnits"]);
    };
    entries
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| {
            (
                entry["name"].as_str().unwrap().to_owned(),
                CssText::from_units(units(&entry["units"])),
            )
        })
        .collect()
}

#[test]
fn computed_custom_tokens_match_chrome159() {
    let cases: Value = serde_json::from_str(CASES).unwrap();
    let control: Value = serde_json::from_str(CONTROL).unwrap();
    assert_eq!(control["browser"], "159.0.8154.6");
    assert_eq!(
        control["probeSha256"],
        format!("{:x}", Sha256::digest(PROBE))
    );
    assert_eq!(
        control["casesSha256"],
        format!("{:x}", Sha256::digest(CASES))
    );
    let cases = cases.as_array().unwrap();
    let rows = control["result"].as_array().unwrap();
    assert_eq!(cases.len(), rows.len());
    assert_eq!(cases.len(), 31);
    for (case, row) in cases.iter().zip(rows) {
        assert_eq!(case["id"], row["id"]);
        let parent =
            compute_custom_properties(&BTreeMap::new(), &winners(&case["parentUnits"]), limits())
                .unwrap();
        let computed =
            compute_custom_properties(&parent.values(), &own_winners(case), limits()).unwrap();
        for (name, expected) in row["values"].as_object().unwrap() {
            // Ordinary-property grammar/value computation is a separate stage.
            if name.starts_with("--") {
                let actual = computed.get(name).map_or(&[][..], CssText::as_units);
                assert_eq!(actual, units(expected), "{} / {name}", case["id"]);
            }
        }
    }
}
