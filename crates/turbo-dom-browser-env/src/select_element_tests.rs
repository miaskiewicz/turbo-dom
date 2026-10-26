use super::*;
use sha2::{Digest, Sha256};

// Runs the SAME probe that scripts/select-element-chrome-control.mjs captured in Chrome 156 and
// compares every section against the saved control.
#[test]
fn select_element_members_match_chrome156_control() {
    super::super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope,isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    let cases_bytes = include_bytes!("select_element_cases.json");
    let probe = include_str!("select_element_probe.js");
    let cases: serde_json::Value = serde_json::from_slice(cases_bytes).unwrap();
    install_independent_html(scope, cases[0]["html"].as_str().unwrap());
    v8::tc_scope!(let scope,scope);
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
        serde_json::from_str(include_str!("select_element_chrome156.json")).unwrap();
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
    let mismatches: Vec<String> = expected
        .as_object()
        .unwrap()
        .iter()
        .filter(|(key, value)| &actual[key.as_str()] != *value)
        .map(|(key, value)| format!("{key}\n  binding: {}\n  chrome:  {value}", actual[key]))
        .collect();
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
    assert_eq!(&actual, expected);
    reset();
}

// The indexed interceptor sits on the shared node template: other nodes keep ordinary indexed
// expandos, and the select helper is not reachable from page script.
#[test]
fn select_indexed_interceptor_leaves_other_nodes_ordinary() {
    super::super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope,isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_independent_html(scope, "<!doctype html><body>");
    v8::tc_scope!(let scope,scope);
    let source = v8::String::new(
        scope,
        r#"(() => {
          const div = document.createElement('div'); div[0] = 'x'; div[7] = 'y';
          const text = document.createTextNode('t'); text[0] = 1;
          const select = document.createElement('select'); select.append(document.createElement('option'));
          return JSON.stringify([div[0], div[7], Object.keys(div).filter(k => /^\d+$/.test(k)), text[0],
            typeof globalThis.__captureSelectIndexed, 'selectSelection' in globalThis[Symbol.for('ts')],
            select[0] === select.options[0], select.length]);
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
        r#"["x","y",["0","7"],1,"undefined",false,true,1]"#
    );
    reset();
}
