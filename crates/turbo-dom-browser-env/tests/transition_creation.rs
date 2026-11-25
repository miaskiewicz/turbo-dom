#![cfg(feature = "host-internals")]
use sha2::{Digest, Sha256};
use std::sync::Once;
use turbo_dom_browser_env::{browser_env, host_internals::*};

fn eval<'s>(scope: &mut v8::PinScope<'s, '_>, source: &str) -> v8::Local<'s, v8::Value> {
    let source = v8::String::new(scope, source).unwrap();
    v8::Script::compile(scope, source, None)
        .unwrap()
        .run(scope)
        .unwrap()
}

fn object<'s>(scope: &mut v8::PinScope<'s, '_>, source: &str) -> v8::Local<'s, v8::Object> {
    v8::Local::try_from(eval(scope, source)).unwrap()
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
    browser_env::install_independent_html(scope, "<!doctype html><div id=parent style='width:400px'><div id=target style='height:20px'></div></div>");
    browser_env::run_engine_script(
        scope,
        r#"
        __tsNs.installAnimationHost({now:()=>1000,frame(){},task(){}});
    "#,
    )
    .unwrap();
    test(scope);
    browser_env::reset();
}

fn native_case(case: &serde_json::Value) -> serde_json::Value {
    let mut result = serde_json::Value::Null;
    with_dom(|scope| {
        let source = format!(
            r#"(()=>{{
            const item={case},target=document.getElementById('target');
            globalThis.target=target;
            target.style.transitionProperty='none';target.style.transitionDuration='0s';
            target.style.interpolateSize=item.interpolateSize||'numeric-only';
            target.style.setProperty(item.property,item.before);
            getComputedStyle(target).opacity;target.getAnimations();
            target.style.transitionProperty=item.property;target.style.transitionDuration='10000ms';
            target.style.transitionTimingFunction='linear';target.style.transitionBehavior=item.behavior||'normal';
            getComputedStyle(target).opacity;
            const beforeEffects=target.getAnimations().length;
            target.style.setProperty(item.property,item.after);
            getComputedStyle(target).opacity;
            const effects=target.getAnimations().map(effect=>({{
                constructor:effect.constructor.name,
                transitionProperty:effect instanceof CSSTransition?effect.transitionProperty:null,
                keyframes:effect.effect.getKeyframes()
            }}));
            return JSON.stringify({{beforeEffects,effects}});
        }})()"#
        );
        let value = eval(scope, &source).to_rust_string_lossy(scope);
        result = serde_json::from_str(&value).unwrap();
        let document = object(scope, "document");
        let target = object(scope, "target");
        let handle = handle_of(scope, target).unwrap();
        let sample = sample_presented_effects(scope, document, &[handle]);
        result["hostSnapshot"] = serde_json::Value::String(format!("{sample:?}"));
    });
    result
}

#[test]
fn transition_creation_matches_chrome159_before_any_sampling_admission() {
    let source = include_bytes!("fixtures/transition_creation_cases.json");
    let controls: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/transition_creation_chrome159.json")).unwrap();
    assert!(controls["browser"].as_str().unwrap().starts_with("159."));
    assert_eq!(
        controls["casesSha256"],
        format!("{:x}", Sha256::digest(source))
    );
    let cases: Vec<serde_json::Value> = serde_json::from_slice(source).unwrap();
    assert_eq!(cases.len(), 24);
    let mut mismatches = Vec::new();
    let mut results = Vec::new();
    for case in &cases {
        let control = controls["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|control| control["id"] == case["id"])
            .unwrap();
        assert_eq!(control["result"]["beforeEffects"], 0);
        let got = native_case(case);
        assert_eq!(got["beforeEffects"], 0, "initial native state: {case:?}");
        let expected = control["result"]["effects"].as_array().unwrap().len();
        let actual = got["effects"].as_array().unwrap().len();
        if actual != expected {
            mismatches.push(format!(
                "{}: Chrome {expected}, native {actual}",
                case["id"]
            ));
        }
        results.push(serde_json::json!({"id":case["id"],"expected":expected,"actual":got}));
    }
    if let Some(path) = std::env::var_os("TURBO_TRANSITION_CREATION_RECEIPT") {
        std::fs::write(path, serde_json::to_vec_pretty(&results).unwrap()).unwrap();
    }
    assert!(
        mismatches.is_empty(),
        "public transition creation mismatches:\n{}",
        mismatches.join("\n")
    );
}
