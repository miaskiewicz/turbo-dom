//! turbo-dom#97: `rtdom::css_shorthand::longhand_value` against the Chrome 156 control
//! `tests/fixtures/css_longhand_chrome156.json` (probe `css-longhand-probe.js`, cases
//! `css-longhand-cases.json` from `scripts/make-css-longhand-cases.py`): every value a longhand
//! grammar admits is spelled like Chrome, every value it rejects Chrome rejects too, and the
//! context-free spelling of the other longhands matches whenever Chrome admits the value.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use turbo_dom::rtdom::css_shorthand::{longhand_alias, longhand_value, Spelled};

const PROBE: &str = include_str!("fixtures/css-longhand-probe.js");
const CASES: &str = include_str!("fixtures/css-longhand-cases.json");
const CONTROL: &str = include_str!("fixtures/css_longhand_chrome156.json");

/// Pairs whose Chrome spelling needs more than this module knows, by `name|value`.
const KNOWN_GAPS: &[&str] = &[
    // turbo-dom#117: math functions in easings, hsl() hue, text-shadow order, area tokens.
];

fn sha(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

#[test]
fn longhand_values_spell_like_chrome156() {
    let control: Value = serde_json::from_str(CONTROL).unwrap();
    assert_eq!(control["browser"], "Chrome/156.0.8078.4");
    assert_eq!(control["errors"], json!([]));
    assert_eq!(control["probeSha256"], sha(PROBE));
    assert_eq!(control["casesSha256"], sha(CASES));
    let rows = control["result"][0]["result"].as_array().unwrap();
    assert!(rows.len() > 2000, "{} pairs", rows.len());
    let mut failures = Vec::new();
    let mut failed = Vec::new();
    let mut checked = 0;
    for row in rows {
        let (name, value, chrome) = (
            row[0].as_str().unwrap(),
            row[1].as_str().unwrap(),
            row[2].as_str().unwrap(),
        );
        let property = longhand_alias(name).unwrap_or(name);
        let id = format!("{name}|{value}");
        let ours = match longhand_value(property, value) {
            Spelled::Invalid => String::new(),
            // The embedder decides validity: only Chrome's admitted values say the spelling.
            Spelled::Unchecked(_) if chrome.is_empty() => continue,
            Spelled::Valid(text) | Spelled::Unchecked(text) => text,
        };
        checked += 1;
        if ours != chrome {
            failed.push(id.clone());
            if !KNOWN_GAPS.contains(&id.as_str()) {
                failures.push(format!(
                    "{id}\n    ours:   {ours:?}\n    chrome: {chrome:?}"
                ));
            }
        }
    }
    assert!(checked > 1900, "only {checked} pairs checked");
    let stale: Vec<_> = KNOWN_GAPS
        .iter()
        .filter(|id| !failed.iter().any(|failed| failed == *id))
        .collect();
    assert!(stale.is_empty(), "stale KNOWN_GAPS: {stale:?}");
    assert!(
        failures.is_empty(),
        "{} pairs differ from Chrome:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
