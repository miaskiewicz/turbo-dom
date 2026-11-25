#![cfg(feature = "host-internals")]
use sha2::{Digest, Sha256};
use std::sync::Once;
use turbo_dom_browser_env::browser_env;

const HTML: &str = include_str!("fixtures/composed_css_renderedness.html");
const CASES: &str = include_str!("fixtures/composed_css_renderedness_cases.json");
const OBSERVE: &str = "JSON.stringify(targets.map(target=>{const effects=target.getAnimations();return{id:target.id,css:effects.filter(effect=>effect instanceof CSSAnimation).length,waapi:effects.filter(effect=>!(effect instanceof CSSAnimation)&&!(effect instanceof CSSTransition)).length,cssNames:effects.filter(effect=>effect instanceof CSSAnimation).map(effect=>effect.animationName)}}))";
fn eval<'s>(scope: &mut v8::PinScope<'s, '_>, source: &str) -> v8::Local<'s, v8::Value> {
    let source = v8::String::new(scope, source).unwrap();
    v8::Script::compile(scope, source, None)
        .unwrap()
        .run(scope)
        .unwrap()
}
fn with_dom(test: impl FnOnce(&mut v8::PinScope)) {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let platform = v8::new_default_platform(0, false).make_shared();
        v8::V8::initialize_platform(platform);
        v8::V8::initialize();
    });
    let isolate = &mut v8::Isolate::new(Default::default());
    isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    browser_env::install_independent_html(scope, HTML);
    browser_env::run_engine_script(scope,"__tsNs.installAnimationHost({now:()=>1000,frame(){},task(){}});globalThis.providerCalls=0;__tsNs.layoutUsedSize=()=>{providerCalls++;throw Error('renderedness must not request geometry')}").unwrap();
    test(scope);
    browser_env::reset();
}
fn control(id: &str) {
    let cases: serde_json::Value = serde_json::from_str(CASES).unwrap();
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/composed_css_renderedness_chrome159.json"
    ))
    .unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(CASES)),
        oracle["casesSha256"].as_str().unwrap()
    );
    assert_eq!(
        format!("{:x}", Sha256::digest(HTML)),
        oracle["htmlSha256"].as_str().unwrap()
    );
    let input = cases
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["id"] == id)
        .unwrap();
    let expected = oracle["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["id"] == id)
        .unwrap();
    with_dom(|scope| {
        eval(scope, input["setup"].as_str().unwrap());
        let mut actual = Vec::new();
        for step in input["steps"].as_array().unwrap() {
            eval(scope, step["script"].as_str().unwrap());
            let value = eval(scope, OBSERVE);
            let targets: serde_json::Value =
                serde_json::from_str(&value.to_rust_string_lossy(scope)).unwrap();
            actual.push(serde_json::json!({"id":step["id"],"targets":targets}));
        }
        assert!(eval(scope, "providerCalls===0").is_true());
        assert_eq!(serde_json::Value::Array(actual), expected["steps"], "{id}");
    });
}
#[test]
fn open_host() {
    control("open-host");
}
#[test]
fn closed_host() {
    control("closed-host");
}
#[test]
fn nested_open_closed() {
    control("nested-open-closed");
}
#[test]
fn assigned_closed_slot() {
    control("assigned-closed-slot");
}
#[test]
fn initially_unassigned_light() {
    control("initially-unassigned-light");
}
#[test]
fn suppressed_fallback() {
    control("suppressed-fallback");
}
#[test]
fn manual_closed_slot() {
    control("manual-closed-slot");
}
