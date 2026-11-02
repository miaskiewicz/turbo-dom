//! XPath from contexts inside a DocumentFragment, a shadow root and a detached subtree, and
//! the namespace axis, against the Chrome 156 control (turbo-dom#185):
//! `scripts/xpath-contexts-chrome-control.mjs` -> `xpath_contexts_chrome156.json`.
use crate::browser_env::*;

const PROBE: &str = include_str!("xpath_contexts_probe.js");

fn run_probe() -> serde_json::Value {
    crate::browser_env::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    // The control page's body holds the script that runs the probe.
    install_with_html(
        scope,
        "<!doctype html><html><head></head><body><script></script></body></html>",
    );
    v8::tc_scope!(let scope, scope);
    let code = v8::String::new(scope, &format!("JSON.stringify({PROBE})")).unwrap();
    let value = v8::Script::compile(scope, code, None)
        .and_then(|script| script.run(scope))
        .unwrap_or_else(|| panic!("{}", scope.exception().unwrap().to_rust_string_lossy(scope)));
    let text = value.to_rust_string_lossy(scope);
    reset();
    serde_json::from_str(&text).unwrap()
}

#[test]
fn xpath_fragment_shadow_and_namespace_contexts_match_chrome() {
    let control: serde_json::Value =
        serde_json::from_str(include_str!("xpath_contexts_chrome156.json")).unwrap();
    let actual = run_probe();
    let mut mismatches = Vec::new();
    for (context, cases) in control["result"].as_object().unwrap() {
        for (expression, expected) in cases.as_object().unwrap() {
            let got = &actual[context][expression];
            if got != expected {
                mismatches.push(format!(
                    "{context} {expression}: got {got}, Chrome {expected}"
                ));
            }
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

/// The control names and pins its probe, so a stale control is caught.
#[test]
fn xpath_contexts_control_pins_its_probe() {
    use sha2::{Digest, Sha256};
    let control: serde_json::Value =
        serde_json::from_str(include_str!("xpath_contexts_chrome156.json")).unwrap();
    assert_eq!(control["fixture"], "xpath_contexts_probe.js");
    assert_eq!(control["browser"], "156.0.8078.47");
    assert_eq!(
        control["probeSha256"],
        format!("{:x}", Sha256::digest(PROBE.as_bytes()))
    );
}
