#![cfg(feature = "host-internals")]
use sha2::{Digest, Sha256};
use std::sync::Once;
use turbo_dom_browser_env::{browser_env, host_internals as host};
const HTML: &str = "<!doctype html><body></body>";
const CASES: &str = include_str!("fixtures/display_boundary_cases.json");
const SAMPLE: &str = "JSON.stringify({time:motion.currentTime,display:getComputedStyle(target).display,progress:motion.effect.getComputedTiming().progress})";
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
    eval(scope,"globalThis.work={cascades:0,updates:0,display:0};globalThis.__noteCascadeRun=()=>{work.cascades++};globalThis.__noteCssUpdatePass=()=>{work.updates++}");
    browser_env::install_independent_html(scope, HTML);
    browser_env::run_engine_script(scope,"let clockHook=null;globalThis.changeClockHook=next=>{clockHook=next};__tsNs.installAnimationHost({now:()=>clockHook?clockHook():1000,frame(){},task(){}});const originalDisplay=__tsNs.computedDisplay;__tsNs.computedDisplay=(...args)=>{work.display++;return originalDisplay(...args)};globalThis.providerCalls=0;__tsNs.layoutUsedSize=()=>{providerCalls++;throw Error('renderedness must not request geometry')}").unwrap();
    let owner = host::enter_context_document(context).unwrap();
    test(scope);
    drop(owner);
    browser_env::reset();
}

fn control(id: &str) {
    let cases: serde_json::Value = serde_json::from_str(CASES).unwrap();
    let oracle: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/display_boundary_chrome159.json")).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(CASES)),
        oracle["casesSHA256"].as_str().unwrap()
    );
    let input = cases
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == id)
        .unwrap();
    let expected = oracle["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == id)
        .unwrap();
    with_dom(|scope| {
        eval(scope, input["setup"].as_str().unwrap());
        let mut samples = Vec::new();
        for time in input["times"].as_array().unwrap() {
            eval(scope, &format!("motion.currentTime={time}"));
            let value = eval(scope, SAMPLE).to_rust_string_lossy(scope);
            samples.push(serde_json::from_str::<serde_json::Value>(&value).unwrap());
        }
        let mut actual = serde_json::json!({"id":id,"samples":samples});
        if input["restore"].as_bool() == Some(true) {
            let value=eval(scope,"target.style.display='block';globalThis.first=getComputedStyle(target).display;globalThis.second=getComputedStyle(target).display;JSON.stringify({first,second,transitions:target.getAnimations().filter(a=>a instanceof CSSTransition).length})").to_rust_string_lossy(scope);
            actual["restore"] = serde_json::from_str(&value).unwrap();
        }
        eprintln!("display boundary {actual}");
        assert_samples(&actual["samples"], &expected["samples"]);
        assert_eq!(actual["restore"], expected["restore"]);
        assert!(eval(scope, "providerCalls===0").is_true());
    });
}
fn assert_samples(actual: &serde_json::Value, expected: &serde_json::Value) {
    let actual = actual.as_array().unwrap();
    let expected = expected.as_array().unwrap();
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.iter().zip(expected) {
        assert_eq!(actual["time"], expected["time"]);
        assert_eq!(actual["display"], expected["display"]);
        match (actual["progress"].as_f64(), expected["progress"].as_f64()) {
            (Some(actual), Some(expected)) => assert!((actual - expected).abs() <= 1e-12),
            _ => assert_eq!(actual["progress"], expected["progress"]),
        }
    }
}

#[test]
fn waapi_enter_first_active_sample() {
    control("waapi-enter-delay0")
}
#[test]
fn waapi_enter_before_delay_and_exact_start() {
    control("waapi-enter-delay100")
}
#[test]
fn waapi_exit_first_active_sample() {
    control("waapi-exit-delay0")
}
#[test]
fn waapi_exit_before_delay_and_exact_start() {
    control("waapi-exit-delay100")
}
#[test]
fn css_exit_end_and_restore() {
    control("css-exit-delay0")
}
#[test]
fn css_exit_delayed_end_and_restore() {
    control("css-exit-delay100")
}
#[test]
fn closed_shadow_exit_end_and_restore_keep_css_and_script_effects_distinct() {
    let cases: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/composed_css_renderedness_cases.json"
    ))
    .unwrap();
    let input = cases
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == "display-exit-transition")
        .unwrap();
    with_dom(|scope| {
        eval(scope, input["setup"].as_str().unwrap());
        let mut counts = Vec::new();
        for step in input["steps"].as_array().unwrap() {
            eval(scope, step["script"].as_str().unwrap());
            let value=eval(scope,"JSON.stringify(targets.map(target=>{const effects=target.getAnimations();return{css:effects.filter(a=>a instanceof CSSAnimation).length,waapi:effects.filter(a=>!(a instanceof CSSAnimation)&&!(a instanceof CSSTransition)).length}}))").to_rust_string_lossy(scope);
            counts.push(serde_json::from_str::<serde_json::Value>(&value).unwrap());
        }
        eprintln!("closed shadow boundary {counts:?}");
        assert_eq!(
            counts,
            vec![
                serde_json::json!([{"css":1,"waapi":1}]),
                serde_json::json!([{"css":1,"waapi":1}]),
                serde_json::json!([{"css":0,"waapi":1}]),
                serde_json::json!([{"css":1,"waapi":1}])
            ]
        );
        assert!(eval(scope, "providerCalls===0").is_true());
    });
}

fn stable_work(display_effect: bool) {
    with_dom(|scope| {
        eval(scope,"document.head.innerHTML='<style>@keyframes pulse{from{opacity:.2}to{opacity:.8}}.motion{animation:pulse 1000ms linear both paused}</style>';globalThis.container=document.body;for(let i=0;i<4;i++){const next=document.createElement('section');container.appendChild(next);container=next}for(let i=0;i<16;i++){const child=document.createElement('div');child.className='motion';container.appendChild(child)}");
        if display_effect {
            eval(scope,"globalThis.displayMotion=container.animate([{display:'block'},{display:'block'}],{duration:1000,fill:'both'});displayMotion.pause();displayMotion.currentTime=500");
        }
        eval(scope,"document.getAnimations();document.getAnimations();work.cascades=0;work.updates=0;work.display=0;providerCalls=0");
        for _ in 0..5 {
            let count = eval(scope, "document.getAnimations().length")
                .number_value(scope)
                .unwrap();
            assert_eq!(count, if display_effect { 17.0 } else { 16.0 });
        }
        let work = eval(scope, "JSON.stringify({...work,provider:providerCalls})")
            .to_rust_string_lossy(scope);
        eprintln!("stable lifecycle display_effect={display_effect} work={work}");
        assert!(eval(scope, "work.updates===0&&providerCalls===0").is_true());
    });
}
#[test]
fn stable_opacity_inputs_keep_css_updates_cached() {
    stable_work(false)
}
#[test]
fn stable_display_inputs_keep_css_updates_cached() {
    stable_work(true)
}

fn host_target<'s>(scope: &mut v8::PinScope<'s, '_>) -> (v8::Local<'s, v8::Object>, host::Handle) {
    eval(scope,"document.head.innerHTML='<style>@keyframes probe{from{opacity:.2}to{opacity:.8}}.motion{animation:probe 1000ms linear both paused}</style>';globalThis.container=document.createElement('div');document.body.appendChild(container);globalThis.target=document.createElement('div');target.className='motion';container.appendChild(target);document.getAnimations()");
    let document = v8::Local::<v8::Object>::try_from(eval(scope, "document")).unwrap();
    let target = v8::Local::<v8::Object>::try_from(eval(scope, "target")).unwrap();
    let handle = host::handle_of(scope, target).unwrap();
    host::sample_presented_effects(scope, document, &[handle]).unwrap();
    (document, handle)
}
#[test]
fn owned_opacity_batch_keeps_the_no_display_inverse() {
    with_dom(|scope| {
        let (document, handle) = host_target(scope);
        eval(scope, "work.display=0");
        host::sample_presented_effects(scope, document, &[handle]).unwrap();
        assert!(eval(scope, "work.display===0&&providerCalls===0").is_true());
    });
}
#[test]
fn clock_created_display_and_custom_effects_invalidate_owned_cache_proof() {
    for frames in [
        "[{display:'none'},{display:'none'}]",
        "[{'--state':'none'},{'--state':'none'}]",
    ] {
        with_dom(|scope| {
            let (document, handle) = host_target(scope);
            eval(scope, "container.style.setProperty('--state','block')");
            host::sample_presented_effects(scope, document, &[handle]).unwrap();
            eval(scope,&format!("work.display=0;changeClockHook(()=>{{changeClockHook(null);globalThis.motion=container.animate({frames},{{duration:1000,fill:'both'}});motion.pause();motion.currentTime=500;return 1000}})"));
            let _sample = host::sample_presented_effects(scope, document, &[handle]);
            assert!(eval(scope, "work.display>0&&providerCalls===0").is_true());
        });
    }
}
#[test]
fn clock_adoption_refuses_the_old_document_without_a_cached_path() {
    with_dom(|scope| {
        let (document, handle) = host_target(scope);
        eval(scope,"globalThis.other=document.implementation.createHTMLDocument('other');changeClockHook(()=>{changeClockHook(null);other.body.appendChild(target);return 1000})");
        assert!(host::sample_presented_effects(scope, document, &[handle]).is_none());
        assert!(eval(scope, "target.ownerDocument===other&&providerCalls===0").is_true());
    });
}
#[test]
fn clock_created_foreign_effect_does_not_reuse_the_owned_cache_proof() {
    with_dom(|scope| {
        let (document, handle) = host_target(scope);
        let target = eval(scope, "target");
        let foreign = v8::Context::new(scope, Default::default());
        let create = {
            let inner = &mut v8::ContextScope::new(scope, foreign);
            browser_env::install_independent_html(inner, HTML);
            let key = v8::String::new(inner, "foreignTarget").unwrap();
            foreign.global(inner).set(inner, key.into(), target);
            eval(inner,"()=>{globalThis.foreignMotion=Element.prototype.animate.call(foreignTarget,[{display:'none'},{display:'none'}],{duration:1000,fill:'both'});foreignMotion.pause();foreignMotion.currentTime=500}")
        };
        let key = v8::String::new(scope, "createForeignEffect").unwrap();
        scope
            .get_current_context()
            .global(scope)
            .set(scope, key.into(), create);
        eval(scope,"work.display=0;changeClockHook(()=>{changeClockHook(null);createForeignEffect();return 1000})");
        assert!(host::sample_presented_effects(scope, document, &[handle]).is_none());
        assert!(eval(scope, "work.display>0&&providerCalls===0").is_true());
        assert!(host::sample_presented_effects(scope, document, &[handle]).is_none());
    });
}
#[test]
fn canceling_a_showing_display_effect_invalidates_prior_rendering() {
    for operation in [
        "show.cancel()",
        "show.effect=null",
        "show.effect.target=other",
        "show.effect.setKeyframes([{opacity:.2},{opacity:.8}])",
    ] {
        with_dom(|scope| {
            let (document, handle) = host_target(scope);
            eval(scope,"globalThis.other=document.createElement('aside');document.body.appendChild(other);target.style.display='none';globalThis.show=target.animate([{display:'block'},{display:'block'}],{duration:1000,fill:'both'});show.pause();show.currentTime=500");
            let before = host::sample_presented_effects(scope, document, &[handle]).unwrap();
            assert!(
                css_contributes(&before, handle),
                "initial showing effect: {before:?}"
            );
            eval(scope, operation);
            let after = host::sample_presented_effects(scope, document, &[handle]).unwrap();
            assert!(!css_contributes(&after, handle), "{operation}: {after:?}");
        });
    }
}
fn css_contributes(sample: &host::PresentedEffectsSnapshot, handle: host::Handle) -> bool {
    sample
        .targets
        .iter()
        .find(|target| target.element == handle)
        .unwrap()
        .effects
        .iter()
        .any(|effect| effect.kind == host::CssEffectKind::Animation && effect.contributes)
}

#[test]
fn unproved_variable_display_keeps_the_existing_refusal() {
    with_dom(|scope| {
        let (document, handle) = host_target(scope);
        eval(
            scope,
            "container.style.setProperty('--state','block');target.style.display='var(--state)'",
        );
        assert!(host::sample_presented_effects(scope, document, &[handle]).is_none());
        assert!(host::sample_presented_effects(scope, document, &[handle]).is_none());
        assert!(eval(scope, "providerCalls===0").is_true());
    });
}
