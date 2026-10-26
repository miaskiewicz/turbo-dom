//! turbo-surf#36 items 1-2: `getComputedStyle` reports Chrome's computed `display` / `position`
//! / `visibility` for properties no author rule sets (UA display per element, blockification,
//! `[hidden]`, dialog / popover positions, CSS-wide keywords), and the document carries no
//! injected `<title>`. Checked against the Chrome 156 control (`computed_defaults_probe.js` +
//! `computed_defaults_cases.json` -> `computed_defaults_chrome156.json`, recorded by
//! scripts/select-element-chrome-control.mjs), every case and section. Case `dialog-top-layer`
//! (turbo-dom#30) covers modal dialogs, the top layer and `::backdrop`; `pseudo-elements` and
//! `pseudo-ua` (turbo-dom#47) the `pseudoElt` argument and the UA-styled pseudo-elements.
use super::*;
use sha2::{Digest, Sha256};

const PROBE: &str = include_str!("computed_defaults_probe.js");
const CASES: &[u8] = include_bytes!("computed_defaults_cases.json");
const CONTROL: &str = include_str!("computed_defaults_chrome156.json");

fn probe(html: &str) -> serde_json::Value {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_independent_html(scope, html);
    let code = format!("JSON.stringify({})", PROBE.trim().trim_end_matches(';'));
    let code = v8::String::new(scope, &code).unwrap();
    v8::tc_scope!(let scope, scope);
    let value = v8::Script::compile(scope, code, None)
        .and_then(|script| script.run(scope))
        .unwrap_or_else(|| panic!("{}", scope.exception().unwrap().to_rust_string_lossy(scope)))
        .to_rust_string_lossy(scope);
    reset();
    serde_json::from_str(&value).unwrap()
}

/// Flatten `value` into `path = json` rows so a mismatch names the element and property.
fn rows(prefix: &str, value: &serde_json::Value, out: &mut Vec<(String, String)>) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, value) in map {
                rows(&format!("{prefix}.{key}"), value, out);
            }
        }
        _ => out.push((prefix.to_owned(), value.to_string())),
    }
}

/// Every control row of the cases `select` keeps, as `path: binding …, chrome …` mismatches.
fn mismatches(select: impl Fn(&str) -> bool) -> Vec<String> {
    let control: serde_json::Value = serde_json::from_str(CONTROL).unwrap();
    assert!(control["browser"]
        .as_str()
        .unwrap()
        .starts_with("Chrome/156."));
    assert_eq!(control["errors"], serde_json::json!([]));
    assert_eq!(
        control["probeSha256"],
        format!("{:x}", Sha256::digest(PROBE.as_bytes()))
    );
    assert_eq!(
        control["casesSha256"],
        format!("{:x}", Sha256::digest(CASES))
    );
    let cases: serde_json::Value = serde_json::from_slice(CASES).unwrap();
    let mut found = Vec::new();
    for (index, case) in cases.as_array().unwrap().iter().enumerate() {
        let id = case["id"].as_str().unwrap();
        let expected = &control["result"][index];
        assert_eq!(expected["id"], case["id"]);
        if !select(id) {
            continue;
        }
        let actual = probe(case["html"].as_str().unwrap());
        let (mut want, mut got) = (Vec::new(), Vec::new());
        rows(id, &expected["result"], &mut want);
        rows(id, &actual, &mut got);
        let got: std::collections::BTreeMap<_, _> = got.into_iter().collect();
        for (path, value) in want {
            let binding = got
                .get(&path)
                .cloned()
                .unwrap_or_else(|| "<missing>".into());
            if binding != value {
                found.push(format!("{path}: binding {binding}, chrome {value}"));
            }
        }
    }
    found
}

#[test]
fn computed_defaults_and_head_match_chrome156_control() {
    let found = mismatches(|id| {
        !matches!(
            id,
            "dialog-top-layer" | "pseudo-elements" | "pseudo-ua-forms" | "pseudo-ua"
        )
    });
    assert!(
        found.is_empty(),
        "{} mismatches:\n{}",
        found.len(),
        found.join("\n")
    );
}

/// turbo-dom#30: show() vs showModal() vs close(), `:modal`, the top-layer position rule and
/// `::backdrop` reads (case `dialog-top-layer`).
#[test]
fn dialog_top_layer_matches_chrome156_control() {
    let found = mismatches(|id| id == "dialog-top-layer");
    assert!(
        found.is_empty(),
        "{} mismatches:\n{}",
        found.len(),
        found.join("\n")
    );
}

/// turbo-dom#47: `getComputedStyle(el, pseudoElt)` — `::before` / `::after` (legacy and bare
/// forms, escapes, comments), invalid pseudo-elements (empty declaration), strings without a
/// leading colon (the element), disconnected elements, live reads (case `pseudo-elements`).
#[test]
fn pseudo_elements_match_chrome156_control() {
    let found = mismatches(|id| id == "pseudo-elements");
    assert!(
        found.is_empty(),
        "{} mismatches:\n{}",
        found.len(),
        found.join("\n")
    );
}

/// turbo-dom#47: the UA-styled pseudo-elements (`::marker`, `::placeholder`, `::first-line`,
/// `::first-letter`, `::selection` and the other highlights, `::file-selector-button`,
/// `::details-content`, `::checkmark`, `::picker-icon`, `::picker(select)`,
/// `::scroll-marker-group`, `::column`, `::view-transition*`, `::-webkit-scrollbar*`,
/// `::scroll-button()`): Chrome's UA values, the author properties each accepts, inheritance
/// (highlights from the parent's highlight), and the forms that read blank or the element
/// (case `pseudo-ua`).
#[test]
#[ignore = "turbo-dom#47: fails until the fix"]
fn pseudo_element_ua_styles_match_chrome156_control() {
    let found = mismatches(|id| id == "pseudo-ua");
    assert!(
        found.is_empty(),
        "{} mismatches:\n{}",
        found.len(),
        found.join("\n")
    );
}

/// turbo-dom#47: every `pseudoElt` form Chrome 156 accepts — computed, blank (`::scroll-marker`,
/// invalid arguments, pseudo-classes after the pseudo-element) or the element's own style
/// (`::cue`, unknown `-webkit-` / `-internal-` names) — on a div, a list item and an input
/// (case `pseudo-ua-forms`).
#[test]
#[ignore = "turbo-dom#47: fails until the fix"]
fn pseudo_element_forms_match_chrome156_control() {
    let found = mismatches(|id| id == "pseudo-ua-forms");
    assert!(
        found.is_empty(),
        "{} mismatches:\n{}",
        found.len(),
        found.join("\n")
    );
}
