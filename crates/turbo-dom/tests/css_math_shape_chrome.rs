//! turbo-dom#338: tree-counting functions (`sibling-index()` / `sibling-count()`) in math
//! expressions and the `shape()` basic shape, spelled by `rtdom::css_shorthand` against the
//! Chrome 156 control `tests/fixtures/css_math_shape_chrome156.json` (probe
//! `css-longhand-probe.js`, cases `css-math-shape-cases.json` from
//! `scripts/make-css-math-shape-cases.py`, the pairs of turbo-dom-browser-env's
//! `css_tree_counting_shape` control). A value a grammar admits or rejects must match Chrome;
//! an unchecked one (the embedder's grammar decides) must spell like Chrome when Chrome
//! admits it.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use turbo_dom::rtdom::css_shorthand::{
    is_shorthand, longhand_alias, longhand_value, DeclarationBlock, Spelled,
};

const PROBE: &str = include_str!("fixtures/css-longhand-probe.js");
const CASES: &str = include_str!("fixtures/css-math-shape-cases.json");
const CONTROL: &str = include_str!("fixtures/css_math_shape_chrome156.json");

fn sha(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

fn longhand(name: &str, value: &str) -> Option<String> {
    match longhand_value(name, value) {
        Spelled::Valid(text) | Spelled::Unchecked(text) => Some(text),
        Spelled::Invalid => None,
    }
}

/// Our spelling of `name: value`, or `None` when the embedder's grammar decides and Chrome
/// rejects the value.
fn ours(name: &str, value: &str, chrome: &str) -> Option<String> {
    if is_shorthand(name) {
        let mut block = DeclarationBlock::new();
        block.set_property(name, value, "", &longhand);
        return Some(block.property_value(name));
    }
    let property = longhand_alias(name).unwrap_or(name);
    match longhand_value(property, value) {
        Spelled::Invalid => Some(String::new()),
        Spelled::Unchecked(_) if chrome.is_empty() => None,
        Spelled::Valid(text) | Spelled::Unchecked(text) => Some(text),
    }
}

#[test]
fn tree_counting_and_shape_spell_like_chrome156() {
    let control: Value = serde_json::from_str(CONTROL).unwrap();
    assert!(control["browser"]
        .as_str()
        .unwrap()
        .starts_with("Chrome/156.0.8078."));
    assert_eq!(control["errors"], json!([]));
    assert_eq!(control["probeSha256"], sha(PROBE));
    assert_eq!(control["casesSha256"], sha(CASES));
    let rows = control["result"][0]["result"].as_array().unwrap();
    assert!(rows.len() > 200, "{} pairs", rows.len());
    let mut failures = Vec::new();
    for row in rows {
        let (name, value, chrome) = (
            row[0].as_str().unwrap(),
            row[1].as_str().unwrap(),
            row[2].as_str().unwrap(),
        );
        let Some(ours) = ours(name, value, chrome) else {
            continue;
        };
        if ours != chrome {
            failures.push(format!(
                "{name}: {value}\n    ours:   {ours:?}\n    chrome: {chrome:?}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} pairs differ from Chrome:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
