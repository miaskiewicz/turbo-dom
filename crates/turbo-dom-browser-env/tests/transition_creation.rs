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

fn public_json(scope: &mut v8::PinScope, source: &str) -> serde_json::Value {
    let text = eval(scope, source).to_rust_string_lossy(scope);
    serde_json::from_str(&text).unwrap()
}

fn sampled(scope: &mut v8::PinScope) -> PresentedEffectsSnapshot {
    let document = object(scope, "document");
    let target = object(scope, "target");
    let handle = handle_of(scope, target).unwrap();
    sample_presented_effects(scope, document, &[handle]).unwrap()
}

#[test]
fn inherited_intrinsic_policy_preserves_real_unknown_effects_and_numeric_override() {
    for numeric in [false, true] {
        with_dom(|scope| {
            let result = public_json(
                scope,
                &format!(
                    r#"(()=>{{
                const parent=document.getElementById('parent'),grand=document.createElement('div');
                parent.before(grand);grand.append(parent);grand.style.interpolateSize='allow-keywords';
                const target=globalThis.target=document.getElementById('target');
                if({numeric})target.style.interpolateSize='numeric-only';
                target.style.transitionProperty='none';target.style.width='auto';getComputedStyle(target).opacity;
                target.style.transitionProperty='width';target.style.transitionDuration='10000ms';getComputedStyle(target).opacity;
                const before=target.getAnimations().length;
                target.style.width='100px';getComputedStyle(target).opacity;
                return JSON.stringify({{before,policy:getComputedStyle(target).interpolateSize,count:target.getAnimations().length}});
            }})()"#
                ),
            );
            assert_eq!(result["before"], 0);
            assert_eq!(result["count"], u32::from(!numeric));
            assert_eq!(
                result["policy"],
                if numeric {
                    "numeric-only"
                } else {
                    "allow-keywords"
                }
            );
            let sample = sampled(scope);
            if !numeric {
                assert!(
                    matches!(
                        sample.targets[0].presentation,
                        PresentedTargetState::Unknown { .. }
                    ),
                    "genuine intrinsic effect stays explicit: {sample:?}"
                );
            }
        });
    }
}

#[test]
fn transition_behavior_lists_follow_the_property_index() {
    for (behavior, expected) in [
        ("allow-discrete,normal", "max-width"),
        ("normal,allow-discrete", "width"),
    ] {
        with_dom(|scope| {
            let result = public_json(
                scope,
                &format!(
                    r#"(()=>{{
                const target=globalThis.target=document.getElementById('target');
                target.style.interpolateSize='numeric-only';target.style.transitionProperty='none';
                target.style.width='auto';target.style.maxWidth='none';getComputedStyle(target).opacity;
                target.style.transitionProperty='max-width,width';target.style.transitionDuration='10000ms';
                target.style.transitionBehavior='{behavior}';getComputedStyle(target).opacity;
                target.getAnimations();target.style.width='100px';target.style.maxWidth='100px';getComputedStyle(target).opacity;
                return JSON.stringify(target.getAnimations().map(a=>a.transitionProperty).sort());
            }})()"#
                ),
            );
            assert_eq!(result, serde_json::json!([expected]));
        });
    }
}

#[test]
fn a_nontransitionable_replacement_cancels_the_old_record_without_script_modification() {
    with_dom(|scope| {
        let result = public_json(
            scope,
            r#"(()=>{
            const target=globalThis.target=document.getElementById('target');
            target.style.transitionProperty='none';target.style.width='50px';getComputedStyle(target).opacity;
            target.style.transitionProperty='width';target.style.transitionDuration='10000ms';getComputedStyle(target).opacity;
            target.getAnimations();target.style.width='100px';getComputedStyle(target).opacity;
            globalThis.motion=target.getAnimations()[0];const before=target.getAnimations().length;
            target.style.width='auto';getComputedStyle(target).opacity;
            return JSON.stringify({before,after:target.getAnimations().length,state:motion.playState});
        })()"#,
        );
        assert_eq!(
            result,
            serde_json::json!({"before":1,"after":0,"state":"idle"})
        );
        let sample = sampled(scope);
        assert_eq!(sample.targets[0].effects.len(), 1);
        assert!(
            !sample.targets[0].effects[0].script_modified,
            "automatic cancellation stays automatic: {sample:?}"
        );
        assert!(matches!(
            sample.targets[0].effects[0].phase,
            CssEffectPhase::Idle
        ));
    });
}

#[test]
fn hidden_ancestors_cannot_acquire_an_own_display_exit_transition() {
    for initially_hidden in [false, true] {
        with_dom(|scope| {
            let result = public_json(
                scope,
                &format!(
                    r#"(()=>{{
                const target=globalThis.target=document.getElementById('target'),parent=document.getElementById('parent');
                if({initially_hidden})parent.style.display='none';
                target.style.display='block';target.style.transitionProperty='display';target.style.transitionDuration='10000ms';target.style.transitionBehavior='allow-discrete';
                getComputedStyle(target).opacity;target.getAnimations();
                parent.style.display='none';target.style.display='none';getComputedStyle(target).opacity;
                return JSON.stringify(target.getAnimations().length);
            }})()"#
                ),
            );
            assert_eq!(result, 0);
        });
    }
}

#[test]
fn scalar_display_sampling_does_not_resolve_the_other_properties_of_a_mixed_effect() {
    with_dom(|scope| {
        eval(
            scope,
            r#"globalThis.target=document.getElementById('target');
            globalThis.motion=target.animate([{display:'block',transform:'translateX(calc(0% + 1em))'},{display:'block',transform:'translateX(calc(50% + 1em))'}],{duration:10000,fill:'both'});
            motion.pause();motion.currentTime=5000;target.style.display='none';target.getAnimations();"#,
        );
        browser_env::run_engine_script(scope, r#"
            globalThis.__layoutCalls=0;globalThis.__transformCalls=0;
            __tsNs.layoutUsedSize=()=>{__layoutCalls++;throw Error('poisoned used-size provider')};
            __tsNs.resolveTransform=()=>{__transformCalls++;throw Error('poisoned transform resolver')};
        "#).unwrap();
        eval(scope, "target.style.transitionDelay='1ms'");
        let sample = sampled(scope);
        assert!(
            matches!(
                sample.targets[0].presentation,
                PresentedTargetState::Unknown { .. }
            ),
            "real unresolved reference-box effect remains Unknown: {sample:?}"
        );
        assert!(eval(scope, "__layoutCalls===0 && __transformCalls===0").is_true());
    });
}

#[test]
fn display_exit_preserves_independent_creation_and_presentation_switches() {
    with_dom(|scope| {
        eval(scope, "globalThis.target=document.getElementById('target');target.style.transitionProperty='display';target.style.transitionDuration='10000ms';target.style.transitionBehavior='allow-discrete';getComputedStyle(target).opacity;target.getAnimations();target.style.display='none';getComputedStyle(target).opacity;globalThis.motion=target.getAnimations()[0]");
        assert!(eval(scope, "motion.effect.getKeyframes()[0].display==='block'").is_true());
        browser_env::set_css_animations(scope, false);
        set_animation_presentation(scope, AnimationPresentation::Disabled);
        assert!(eval(scope, "target.getAnimations().includes(motion)").is_true());
        let sample = sampled(scope);
        assert!(sample.presentation_suppressed);
        assert_eq!(sample.targets[0].effects.len(), 1);
        assert!(
            matches!(&sample.targets[0].presentation, PresentedTargetState::Certified { animation, transition } if animation.is_empty() && transition.is_empty())
        );
    });
    with_dom(|scope| {
        browser_env::set_css_transitions(scope, false);
        assert!(eval(scope, "const target=document.getElementById('target');target.style.transitionProperty='display';target.style.transitionDuration='10000ms';target.style.transitionBehavior='allow-discrete';getComputedStyle(target).opacity;target.style.display='none';target.getAnimations().length===0").is_true());
    });
}
