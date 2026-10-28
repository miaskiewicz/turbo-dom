//! turbo-dom#90: `element.style` (and `CSSStyleRule.style`) expand shorthands into longhands
//! and serialize them back like Chrome 156. Runs turbo-dom's shorthand probe
//! (`crates/turbo-dom/tests/fixtures/css-shorthand-probe.js` on `css-shorthand-cases.json`) in
//! the binding and compares every scenario with the Chrome control
//! `css_shorthand_chrome156.json` (the same fixture `rtdom::css_shorthand` is checked against).
use super::*;
use crate::browser_env::crate_surface::harness_tests::thrown;
use sha2::{Digest, Sha256};

/// Scenarios whose binding result differs from Chrome for a documented reason, by id.
const KNOWN_GAPS: &[&str] = &[
    // turbo-dom#117: math functions, hsl() hue, place-content / offset / columns /
    // timeline-trigger spellings (the npm port and the rtdom tests share them).
];

fn fixture(name: &str) -> String {
    let path = format!(
        "{}/../turbo-dom/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("turbo-dom's fixture {path}: {error}"))
}

fn run(html: &str, probe: &str) -> serde_json::Value {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, html);
    let code = format!("JSON.stringify({})", probe.trim().trim_end_matches(';'));
    let code = v8::String::new(scope, &code).unwrap();
    v8::tc_scope!(let scope, scope);
    let value = v8::Script::compile(scope, code, None)
        .and_then(|script| script.run(scope))
        .unwrap_or_else(|| panic!("probe threw: {}", thrown(scope.exception(), scope)))
        .to_rust_string_lossy(scope);
    reset();
    serde_json::from_str(&value).unwrap()
}

#[test]
fn inline_style_shorthands_expand_and_serialize_like_chrome156() {
    let probe = fixture("css-shorthand-probe.js");
    let cases = fixture("css-shorthand-cases.json");
    let control: serde_json::Value =
        serde_json::from_str(&fixture("css_shorthand_chrome156.json")).unwrap();
    assert_eq!(control["browser"], "Chrome/156.0.8078.4");
    assert_eq!(control["errors"], serde_json::json!([]));
    assert_eq!(
        control["probeSha256"],
        format!("{:x}", Sha256::digest(probe.as_bytes()))
    );
    assert_eq!(
        control["casesSha256"],
        format!("{:x}", Sha256::digest(cases.as_bytes()))
    );
    let cases: serde_json::Value = serde_json::from_str(&cases).unwrap();
    let expected = &control["result"][0]["result"];
    let actual = run(cases[0]["html"].as_str().unwrap(), &probe);
    let mut failures = Vec::new();
    for key in ["table", "aliases", "rules"] {
        if actual[key] != expected[key] {
            failures.push(format!(
                "{key}:\n    binding: {}\n    chrome:  {}",
                actual[key], expected[key]
            ));
        }
    }
    let rows = expected["results"].as_array().unwrap();
    let ours = actual["results"].as_array().unwrap();
    assert_eq!(ours.len(), rows.len());
    let mut failed = Vec::new();
    for (row, our) in rows.iter().zip(ours) {
        let id = row["id"].as_str().unwrap();
        assert_eq!(our["id"], id);
        if our != row {
            failed.push(id.to_string());
            if !KNOWN_GAPS.contains(&id) {
                let mut out = Vec::new();
                for key in ["cssText", "items", "shorthands", "returns", "assembled"] {
                    if our.get(key) != row.get(key) {
                        out.push(format!(
                            "  {key}\n    binding: {}\n    chrome:  {}",
                            our.get(key).unwrap_or(&serde_json::Value::Null),
                            row.get(key).unwrap_or(&serde_json::Value::Null)
                        ));
                    }
                }
                failures.push(format!("{id}:\n{}", out.join("\n")));
            }
        }
    }
    let stale: Vec<_> = KNOWN_GAPS
        .iter()
        .filter(|id| !failed.iter().any(|failed| failed == *id))
        .collect();
    assert!(stale.is_empty(), "stale KNOWN_GAPS: {stale:?}");
    assert!(
        failures.is_empty(),
        "{} of {} scenarios differ from Chrome:\n{}",
        failures.len(),
        rows.len(),
        failures.join("\n")
    );
}
