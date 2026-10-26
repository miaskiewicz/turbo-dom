use super::*;
use sha2::{Digest, Sha256};

// The control was captured by running the same probe in a fresh Chrome 156 profile through
// turbo-surf's harness/detection/sandbox-navigation-chrome.mjs collector.
#[test]
fn meter_progress_and_output_reflection_match_chrome_156() {
    let cases = include_bytes!("form_outputs_cases.json");
    let probe = include_str!("form_outputs_probe.js");
    let control: serde_json::Value =
        serde_json::from_str(include_str!("form_outputs_chrome156.json")).unwrap();
    assert_eq!(control["browser"], "Chrome/156.0.8078.47");
    assert_eq!(control["errors"], serde_json::json!([]));
    assert_eq!(
        control["probeSha256"],
        format!("{:x}", Sha256::digest(probe.as_bytes()))
    );
    assert_eq!(
        control["casesSha256"],
        format!("{:x}", Sha256::digest(cases))
    );
    let cases: serde_json::Value = serde_json::from_slice(cases).unwrap();
    assert_eq!(control["result"].as_array().unwrap().len(), 1);
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope,isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_independent_html(scope, cases[0]["html"].as_str().unwrap());
    v8::tc_scope!(let scope,scope);
    let source = v8::String::new(scope, probe).unwrap();
    let Some(value) = v8::Script::compile(scope, source, None).and_then(|script| script.run(scope))
    else {
        panic!("{}", scope.exception().unwrap().to_rust_string_lossy(scope));
    };
    let actual = v8::json::stringify(scope, value)
        .unwrap()
        .to_rust_string_lossy(scope);
    let actual: serde_json::Value = serde_json::from_str(&actual).unwrap();
    let expected = &control["result"][0]["result"];
    // Compare section by section so a regression names the member family that drifted.
    for (key, value) in expected.as_object().unwrap() {
        assert_eq!(&actual[key], value, "section {key} drifted from Chrome");
    }
    assert_eq!(&actual, expected);
    reset();
}
