//! Custom element reaction order, template-contents upgrades and qualified-name extraction
//! through the binding, against two Chrome 156 controls:
//! - the npm runtime's `test/fixtures/dom_adoption_reactions_{probe.js,cases.json,chrome156.json}`
//!   (turbo-dom#29; read from the repository, not copied — standalone V8 has no `DOMParser`,
//!   so the probe's one `new DOMParser().parseFromString(…)` document is the binding's
//!   `parse_html_document` of the same markup);
//! - `dom_reactions_names_{probe.js,cases.json,chrome156.json}` (fragment-parser setters'
//!   reaction order and qualified names for elements / attributes), recorded by
//!   scripts/select-element-chrome-control.mjs on Chrome for Testing 156.0.8078.4.
//!
//! turbo-dom#42: `customElements.upgrade()` skips inert template contents and an element is
//! never constructed twice; #43: each element's reactions run before the next element's, and
//! the fragment parser queues upgrades before a setter's removals; #44: `a:b:c` is prefix `a`,
//! local name `b`; #54: an independently installed document's removals run their reactions
//! before the call returns.
use super::*;
use sha2::{Digest, Sha256};

const ADOPTION_PROBE: &str = include_str!("../../../test/fixtures/dom_adoption_reactions_probe.js");
const ADOPTION_CASES: &str =
    include_str!("../../../test/fixtures/dom_adoption_reactions_cases.json");
const ADOPTION_CONTROL: &str =
    include_str!("../../../test/fixtures/dom_adoption_reactions_chrome156.json");
const NAMES_PROBE: &str = include_str!("dom_reactions_names_probe.js");
const NAMES_CASES: &str = include_str!("dom_reactions_names_cases.json");
const NAMES_CONTROL: &str = include_str!("dom_reactions_names_chrome156.json");

/// Run `probe` over the control's single case and return (binding, chrome) results.
fn run(probe: &str, cases: &str, control: &str) -> (serde_json::Value, serde_json::Value) {
    let control: serde_json::Value = serde_json::from_str(control).unwrap();
    assert!(control["browser"]
        .as_str()
        .unwrap()
        .starts_with("Chrome/156."));
    assert_eq!(control["errors"], serde_json::json!([]));
    assert_eq!(
        control["probeSha256"],
        format!("{:x}", Sha256::digest(probe.as_bytes()))
    );
    assert_eq!(
        control["casesSha256"],
        format!("{:x}", Sha256::digest(cases.as_bytes()))
    );
    let cases: serde_json::Value = serde_json::from_str(cases).unwrap();
    let html = cases[0]["html"].as_str().unwrap();
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_independent_html(scope, html);
    let mut source = probe.trim().trim_end_matches(';').to_owned();
    if let Some(start) = source.find("new DOMParser().parseFromString(") {
        let end = start + source[start..].find("'text/html')").unwrap() + "'text/html')".len();
        let markup = source[start..end].split('\'').nth(1).unwrap().to_owned();
        let other = parse_html_document(scope, &markup, false);
        let key = v8::String::new(scope, "__probeOtherDocument").unwrap();
        context.global(scope).set(scope, key.into(), other.into());
        source.replace_range(start..end, "__probeOtherDocument");
    }
    let code = v8::String::new(scope, &format!("JSON.stringify({source})")).unwrap();
    v8::tc_scope!(let scope, scope);
    let value = v8::Script::compile(scope, code, None)
        .and_then(|script| script.run(scope))
        .unwrap_or_else(|| panic!("{}", scope.exception().unwrap().to_rust_string_lossy(scope)))
        .to_rust_string_lossy(scope);
    reset();
    (
        serde_json::from_str(&value).unwrap(),
        control["result"][0]["result"].clone(),
    )
}

/// The `log` rows between the `["--", step]` marker and the next marker.
fn step(log: &serde_json::Value, name: &str) -> Vec<serde_json::Value> {
    let rows = log.as_array().unwrap();
    let start = rows
        .iter()
        .position(|row| row[0] == "--" && row[1] == name)
        .unwrap_or_else(|| panic!("no step {name}"));
    rows[start + 1..]
        .iter()
        .take_while(|row| row[0] != "--")
        .cloned()
        .collect()
}

/// `label: binding …, chrome …` for every listed key and log step that differs.
fn mismatches(
    binding: &serde_json::Value,
    chrome: &serde_json::Value,
    keys: &[&str],
    steps: &[&str],
) -> Vec<String> {
    let mut found = Vec::new();
    for key in keys {
        if binding[key] != chrome[key] {
            found.push(format!(
                "{key}: binding {}, chrome {}",
                binding[key], chrome[key]
            ));
        }
    }
    for name in steps {
        let (got, want) = (step(&binding["log"], name), step(&chrome["log"], name));
        if got != want {
            found.push(format!(
                "log {name}: binding {}, chrome {}",
                serde_json::Value::from(got),
                serde_json::Value::from(want)
            ));
        }
    }
    found
}

fn assert_none(found: &[String]) {
    assert!(
        found.is_empty(),
        "{} mismatches:\n{}",
        found.len(),
        found.join("\n")
    );
}

/// turbo-dom#42: `customElements.upgrade(clone of template contents)` leaves the inert
/// candidates alone; appending them to a detached element adopts without constructing; they
/// upgrade once (and only once) when connected.
#[test]
#[ignore = "turbo-dom#42: fails until the fix"]
fn template_contents_upgrade_once_matching_chrome156() {
    let (binding, chrome) = run(ADOPTION_PROBE, ADOPTION_CASES, ADOPTION_CONTROL);
    assert_none(&mismatches(
        &binding,
        &chrome,
        &["upgradeInert", "detached", "connected"],
        &["upgrade-inert", "append-detached", "connect"],
    ));
}

/// turbo-dom#43: reactions run element by element (importNode: an element's upgrade and its
/// attributeChangedCallback before the next element's upgrade), and the fragment parser
/// queues its elements' upgrades before innerHTML / outerHTML / setHTMLUnsafe remove the old
/// children.
#[test]
#[ignore = "turbo-dom#43: fails until the fix"]
fn reactions_run_element_by_element_matching_chrome156() {
    let (binding, chrome) = run(ADOPTION_PROBE, ADOPTION_CASES, ADOPTION_CONTROL);
    let mut found = mismatches(&binding, &chrome, &[], &["import", "innerHTML-replace"]);
    let (binding, chrome) = run(NAMES_PROBE, NAMES_CASES, NAMES_CONTROL);
    if binding["order"] != chrome["order"] {
        found.push(format!(
            "order: binding {}, chrome {}",
            binding["order"], chrome["order"]
        ));
    }
    assert_none(&found);
}

/// turbo-dom#44: a qualified name with more than one colon keeps the second part as its
/// local name (elements, attributes, attribute nodes).
#[test]
#[ignore = "turbo-dom#44: fails until the fix"]
fn qualified_names_split_strictly_matching_chrome156() {
    let (binding, chrome) = run(ADOPTION_PROBE, ADOPTION_CASES, ADOPTION_CONTROL);
    let mut found = mismatches(&binding, &chrome, &["nsTwoColons"], &[]);
    let (binding, chrome) = run(NAMES_PROBE, NAMES_CASES, NAMES_CONTROL);
    for (name, want) in chrome["names"].as_object().unwrap() {
        if binding["names"][name] != *want {
            found.push(format!(
                "names {name}: binding {}, chrome {want}",
                binding["names"][name]
            ));
        }
    }
    assert_none(&found);
}

/// The whole turbo-dom#29 control through the binding: every section and the full reaction log.
#[test]
#[ignore = "turbo-dom#42, #43, #44: fails until the fixes"]
fn dom_adoption_reactions_match_chrome156_control() {
    let (binding, chrome) = run(ADOPTION_PROBE, ADOPTION_CASES, ADOPTION_CONTROL);
    let mut found = Vec::new();
    for (key, want) in chrome.as_object().unwrap() {
        if binding[key] != *want {
            found.push(format!("{key}: binding {}, chrome {want}", binding[key]));
        }
    }
    assert_none(&found);
}

/// turbo-dom#54: in an independently installed document (this harness), removeChild() /
/// remove() / replaceChild() run their disconnectedCallbacks before they return.
#[test]
#[ignore = "turbo-dom#54: fails until the fix"]
fn independent_document_reactions_run_before_the_call_returns() {
    let (binding, chrome) = run(NAMES_PROBE, NAMES_CASES, NAMES_CONTROL);
    let mut found = Vec::new();
    if binding["removal"] != chrome["removal"] {
        found.push(format!(
            "removal: binding {}, chrome {}",
            binding["removal"], chrome["removal"]
        ));
    }
    assert_none(&found);
}
