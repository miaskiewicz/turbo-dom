use super::*;
use sha2::{Digest, Sha256};

// Runs the SAME probe that scripts/select-element-chrome-control.mjs captured in Chrome 156 and
// compares every phase and pseudo-class against the saved control.
#[test]
fn form_pseudo_classes_match_chrome156_control() {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    let cases_bytes = include_bytes!("form_pseudo_cases.json");
    let probe = include_str!("form_pseudo_probe.js");
    let cases: serde_json::Value = serde_json::from_slice(cases_bytes).unwrap();
    install_independent_html(scope, cases[0]["html"].as_str().unwrap());
    v8::tc_scope!(let scope, scope);
    let source = v8::String::new(scope, probe).unwrap();
    let Some(result) =
        v8::Script::compile(scope, source, None).and_then(|script| script.run(scope))
    else {
        panic!("{}", scope.exception().unwrap().to_rust_string_lossy(scope));
    };
    let actual = v8::json::stringify(scope, result)
        .unwrap()
        .to_rust_string_lossy(scope);
    let actual: serde_json::Value = serde_json::from_str(&actual).unwrap();
    let control: serde_json::Value =
        serde_json::from_str(include_str!("form_pseudo_chrome156.json")).unwrap();
    assert_eq!(control["browser"], "Chrome/156.0.8078.47");
    assert_eq!(control["errors"], serde_json::json!([]));
    assert_eq!(
        control["probeSha256"],
        format!("{:x}", Sha256::digest(probe.as_bytes()))
    );
    assert_eq!(
        control["casesSha256"],
        format!("{:x}", Sha256::digest(cases_bytes))
    );
    let expected = &control["result"][0]["result"];
    let mut mismatches = Vec::new();
    for (phase, chrome) in expected.as_object().unwrap() {
        match chrome.as_object() {
            Some(entries) => {
                for (key, value) in entries {
                    if &actual[phase][key] != value {
                        mismatches.push(format!(
                            "{phase} {key}\n  binding: {}\n  chrome:  {value}",
                            actual[phase][key]
                        ));
                    }
                }
            }
            None if &actual[phase] != chrome => mismatches.push(format!(
                "{phase}\n  binding: {}\n  chrome:  {chrome}",
                actual[phase]
            )),
            None => {}
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
    assert_eq!(&actual, expected);
    reset();
}

// The classifier is private and reads only install-time captures: guest prototype changes
// between queries neither break nor observe it.
#[test]
fn form_pseudo_classifier_ignores_guest_prototypes() {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_independent_html(
        scope,
        "<!doctype html><body><form><input id=a type=checkbox checked><input id=b type=radio name=r><input id=c required></form>",
    );
    v8::tc_scope!(let scope, scope);
    let source = v8::String::new(
        scope,
        r#"(() => {
          const seen = [];
          Map.prototype.get = function () { seen.push('map'); return undefined; };
          Array.prototype.indexOf = function () { seen.push('indexOf'); return -1; };
          Object.defineProperty(Array.prototype, '0', { set(value) { seen.push('setter'); }, configurable: true });
          Object.defineProperty(HTMLOptionElement.prototype, 'selected', { get() { seen.push('selected'); return true; }, configurable: true });
          const ids = (selector) => Array.from(document.querySelectorAll(selector), (element) => element.id).join();
          const result = [ids(':checked'), ids(':indeterminate'), ids(':invalid'), ids('input:read-write'),
            typeof globalThis.__captureFormPseudo, typeof globalThis.__captureFormInternals];
          delete Array.prototype[0];
          return JSON.stringify([result, seen]);
        })()"#,
    )
    .unwrap();
    let Some(result) =
        v8::Script::compile(scope, source, None).and_then(|script| script.run(scope))
    else {
        panic!("{}", scope.exception().unwrap().to_rust_string_lossy(scope));
    };
    assert_eq!(
        result.to_rust_string_lossy(scope),
        r#"[["a","b",",c","c","undefined","undefined"],[]]"#
    );
    reset();
}
