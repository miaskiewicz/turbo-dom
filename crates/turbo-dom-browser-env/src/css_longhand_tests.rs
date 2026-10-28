//! turbo-dom#97: a longhand set on `element.style` reads back in Chrome 156's specified-value
//! spelling (`0` -> `0px`, units and keywords lowercase, `url("...")`, double-quoted strings,
//! colors as `rgb()`), and values Chrome accepts are not rejected. Runs turbo-dom's probe
//! (`crates/turbo-dom/tests/fixtures/css-longhand-probe.js` on `css-longhand-cases.json`) and
//! compares every pair with the Chrome control `css_longhand_chrome156.json`.
use super::*;
use crate::browser_env::crate_surface::harness_tests::thrown;
use sha2::{Digest, Sha256};

/// Pairs whose binding result differs from Chrome for a documented reason, by `name|value`.
const KNOWN_GAPS: &[&str] = &[
    // turbo-dom#117: math functions in easings, hsl() hue, text-shadow order, area tokens.
    "transition-timing-function|cubic-bezier(calc(1px), 0, 1, 1)",
    "transition-timing-function|cubic-bezier(min(0.5, 1), 0, 1, 1)",
    "transition-timing-function|cubic-bezier(a, 0, 1, 1)",
    "transition-timing-function|cubic-bezier(calc(2px), 0, 1, 1)",
    "transition-timing-function|linear(calc(0), 1)",
    "transition-timing-function|linear(0, 1 calc(50%))",
    "color|hsl(1px 50% 50%)",
    "color|hsl(10% 50% 50%)",
    "color|rgb(from red r g b / 1 1)",
    "text-shadow|1px 1px #ABC",
    "grid-template-areas|\"a\\\"b\"",
    "grid-template-areas|\"a\\\\b\"",
    // turbo-dom#117 (offset rotate `0`): the binding's declaration layer accepts a unitless 0
    // angle in offset-rotate, which Chrome 156 rejects.
    "offset-rotate|0",
    "offset-rotate|auto 0",
    "offset-rotate|0 reverse",
];

fn fixture(name: &str) -> String {
    let path = format!(
        "{}/../turbo-dom/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("turbo-dom's fixture {path}: {error}"))
}

#[test]

fn inline_style_longhands_spell_values_like_chrome156() {
    let probe = fixture("css-longhand-probe.js");
    let cases = fixture("css-longhand-cases.json");
    let control: serde_json::Value =
        serde_json::from_str(&fixture("css_longhand_chrome156.json")).unwrap();
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
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, cases[0]["html"].as_str().unwrap());
    let code = format!("JSON.stringify({})", probe.trim().trim_end_matches(';'));
    let code = v8::String::new(scope, &code).unwrap();
    let actual: serde_json::Value = {
        v8::tc_scope!(let scope, scope);
        let value = v8::Script::compile(scope, code, None)
            .and_then(|script| script.run(scope))
            .unwrap_or_else(|| panic!("probe threw: {}", thrown(scope.exception(), scope)))
            .to_rust_string_lossy(scope);
        serde_json::from_str(&value).unwrap()
    };
    reset();
    let expected = control["result"][0]["result"].as_array().unwrap();
    let ours = actual.as_array().unwrap();
    assert_eq!(ours.len(), expected.len());
    let mut failures = Vec::new();
    let mut failed = Vec::new();
    for (our, row) in ours.iter().zip(expected) {
        let id = format!("{}|{}", row[0].as_str().unwrap(), row[1].as_str().unwrap());
        if our != row {
            if !KNOWN_GAPS.contains(&id.as_str()) {
                failures.push(format!("{id}\n    binding: {our}\n    chrome:  {row}"));
            }
            failed.push(id);
        }
    }
    let stale: Vec<_> = KNOWN_GAPS
        .iter()
        .filter(|id| !failed.iter().any(|failed| failed == *id))
        .collect();
    assert!(stale.is_empty(), "stale KNOWN_GAPS: {stale:?}");
    assert!(
        failures.is_empty(),
        "{} of {} pairs differ from Chrome:\n{}",
        failures.len(),
        expected.len(),
        failures.join("\n")
    );
}
