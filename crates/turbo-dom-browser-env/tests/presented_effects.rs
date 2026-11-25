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

fn init_v8() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let platform = v8::new_default_platform(0, false).make_shared();
        v8::V8::initialize_platform(platform);
        v8::V8::initialize();
    });
}

fn with_dom_using(independent: bool, test: impl FnOnce(&mut v8::PinScope)) {
    with_dom_lifecycle(independent, true, test);
}

fn with_dom_lifecycle(independent: bool, reset: bool, test: impl FnOnce(&mut v8::PinScope)) {
    with_html(
        independent,
        reset,
        "<!doctype html><div id=target style='width:100px;height:20px'></div>",
        test,
    );
}

fn with_html(independent: bool, reset: bool, html: &str, test: impl FnOnce(&mut v8::PinScope)) {
    init_v8();
    let isolate = &mut v8::Isolate::new(snapshot_params());
    isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    if independent {
        browser_env::install_independent_html(scope, html);
    } else {
        browser_env::install_with_html(scope, html);
    }
    browser_env::run_engine_script(
        scope,
        r#"
        let now=1000;
        globalThis.__clockCalls=0;globalThis.__steppingClock=false;
        __tsNs.installAnimationHost({now:()=>{__clockCalls++;return now+(__steppingClock?__clockCalls:0)},frame(){},task(){}});
        globalThis.__layoutCalls=0;
        __tsNs.layoutUsedSize=()=>{__layoutCalls++;throw Error('layout provider called')};
        __tsNs.resolveTransform=()=>{__layoutCalls++;throw Error('transform provider called')};
    "#,
    )
    .unwrap();
    test(scope);
    if reset {
        browser_env::reset();
    }
}

fn with_dom(test: impl FnOnce(&mut v8::PinScope)) {
    with_dom_using(true, test);
}

#[test]
fn discovers_paused_waapi_without_css_candidates_and_never_calls_layout() {
    with_dom(|scope| {
        eval(scope,"globalThis.target=document.getElementById('target');globalThis.effect=target.animate([{width:'100px'},{width:'200px'}],{duration:1000,fill:'both'});effect.pause();effect.currentTime=500");
        let document = object(scope, "document");
        let target = object(scope, "target");
        let handle = handle_of(scope, target).unwrap();
        let sample = sample_presented_effects(scope, document, &[])
            .expect("canonical WAAPI target discovered independently of CSS candidates");
        assert_eq!(sample.targets.len(), 1);
        assert_eq!(sample.targets[0].element, handle);
        let PresentedTargetState::Certified {
            animation,
            transition,
        } = &sample.targets[0].presentation
        else {
            panic!("absolute paused width must be certified: {sample:?}");
        };
        assert_eq!(
            animation,
            &[PresentedProperty {
                name: "width".into(),
                value: "150px".into()
            }]
        );
        assert!(transition.is_empty());
        assert!(eval(scope, "__layoutCalls===0").is_true());
    });
}

#[test]
fn canonical_context_evidence_includes_delay_and_distinguishes_css_ownership() {
    let probe = include_str!("fixtures/presented_context_phases_probe.js");
    let control: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/presented_context_phases_chrome159.json"
    ))
    .unwrap();
    assert!(control["browser"].as_str().unwrap().starts_with("159."));
    assert_eq!(
        control["probeSha256"],
        format!("{:x}", Sha256::digest(probe.as_bytes()))
    );
    let cases = control["result"].as_array().unwrap();
    assert_eq!(cases.len(), 14);
    for case in cases {
        let css = case["kind"] == "css";
        let action = phase_action(&case["action"]);
        with_dom(|scope| {
            let setup = if css {
                "const style=document.createElement('style');style.textContent='@keyframes context{from{opacity:.5}to{opacity:1}}';document.head.append(style);target.style.animation='context 1000ms 1000ms linear';globalThis.motion=target.getAnimations()[0]"
            } else {
                "globalThis.motion=target.animate([{opacity:.5},{opacity:1}],{duration:1000,delay:1000,fill:'none'})"
            };
            eval(scope, &format!("globalThis.target=document.getElementById('target');{setup};{action};__layoutCalls=0"));
            let document = object(scope, "document");
            let target = object(scope, "target");
            let handle = handle_of(scope, target).unwrap();
            let sample = sample_presented_effects(scope, document, &[handle]).unwrap();
            assert_eq!(
                sample.targets[0].establishes_stacking_context,
                case["hit"] == "cover",
                "{case:?}"
            );
            assert!(eval(scope, "__layoutCalls===0").is_true());
        });
    }
}

fn phase_action(action: &serde_json::Value) -> String {
    if let Some(time) = action.as_i64() {
        format!("motion.pause();motion.currentTime={time}")
    } else if action == "finish" {
        "motion.finish()".into()
    } else {
        "motion.cancel()".into()
    }
}

#[test]
fn context_metadata_obeys_presentation_policy_and_live_css_rule_ownership() {
    with_dom(|scope| {
        eval(scope, "globalThis.target=document.getElementById('target');const style=document.createElement('style');style.textContent='@keyframes context{from{opacity:1}to{opacity:1}}';document.head.append(style);target.style.animation='context 1000ms linear both';target.style.setProperty('opacity','1','important');globalThis.motion=target.getAnimations()[0];motion.pause();motion.currentTime=500");
        let document = object(scope, "document");
        let target = object(scope, "target");
        let handle = handle_of(scope, target).unwrap();
        assert!(
            sample_presented_effects(scope, document, &[handle])
                .unwrap()
                .targets[0]
                .establishes_stacking_context
        );
        set_animation_presentation(scope, AnimationPresentation::Disabled);
        assert!(
            !sample_presented_effects(scope, document, &[handle])
                .unwrap()
                .targets[0]
                .establishes_stacking_context
        );
        set_animation_presentation(scope, AnimationPresentation::Enabled);
        eval(scope, "target.style.animation='none'");
        assert!(
            !sample_presented_effects(scope, document, &[handle])
                .unwrap()
                .targets[0]
                .establishes_stacking_context
        );
    });
}

#[test]
fn context_metadata_preserves_existing_effects_when_css_creation_is_disabled() {
    with_dom(|scope| {
        eval(scope, "globalThis.target=document.getElementById('target');const style=document.createElement('style');style.textContent='@keyframes context{from{opacity:.5}to{opacity:1}}';document.head.append(style);target.style.animation='context 1000ms 1000ms linear';globalThis.motion=target.getAnimations()[0];motion.pause();motion.currentTime=500");
        browser_env::set_css_animations(scope, false);
        assert!(eval(scope, "document.getAnimations().includes(motion)").is_true());
        let document = object(scope, "document");
        let target = object(scope, "target");
        let handle = handle_of(scope, target).unwrap();
        let sample = sample_presented_effects(scope, document, &[handle]).unwrap();
        assert!(
            sample.targets[0].establishes_stacking_context,
            "canonical retained before-phase effect: {sample:?}"
        );
    });
    with_dom(|scope| {
        browser_env::set_css_animations(scope, false);
        eval(scope, "globalThis.target=document.getElementById('target');const style=document.createElement('style');style.textContent='@keyframes context{from{opacity:.5}to{opacity:1}}';document.head.append(style);target.style.animation='context 1000ms 1000ms linear'");
        assert!(eval(scope, "document.getAnimations().length===0").is_true());
        let document = object(scope, "document");
        let target = object(scope, "target");
        let handle = handle_of(scope, target).unwrap();
        let sample = sample_presented_effects(scope, document, &[handle]).unwrap();
        assert!(!sample.targets[0].establishes_stacking_context);
    });
}

fn snapshot_params() -> v8::CreateParams {
    match std::env::var_os("TURBO_DOM_SNAPSHOT_PATH").and(browser_env::snapshot_blob()) {
        Some(blob) => blob.create_params(),
        None => Default::default(),
    }
}

#[test]
fn default_document_zero_has_a_native_empty_certificate_without_guest_getters() {
    with_dom_using(false, |scope| {
        let document = object(scope, "document");
        let snapshot =
            sample_presented_effects(scope, document, &[]).expect("native default document zero");
        assert_eq!(snapshot.document_id, 0);
        assert!(snapshot.targets.is_empty());
        let fake = object(scope, "({})");
        assert!(sample_presented_effects(scope, fake, &[]).is_none());
        let proxy = object(scope, "globalThis.__traps=0;new Proxy(document,{get(){__traps++;throw Error('guest getter')}})");
        assert!(sample_presented_effects(scope, proxy, &[]).is_none());
        assert!(eval(scope, "__traps===0 && __layoutCalls===0").is_true());
    });
}

#[test]
fn css_animation_and_important_transition_keep_separate_cascade_origins() {
    with_dom(|scope| {
        eval(scope,"globalThis.target=document.getElementById('target');const style=document.createElement('style');style.textContent='@keyframes grow{from{width:100px}to{width:200px}}';document.head.append(style);target.style.animation='grow 1000ms linear both';globalThis.motion=target.getAnimations()[0];motion.pause();motion.currentTime=500;__layoutCalls=0");
        let document = object(scope, "document");
        let target = object(scope, "target");
        let handle = handle_of(scope, target).unwrap();
        let sample = sample_presented_effects(scope, document, &[handle]).unwrap();
        let PresentedTargetState::Certified {
            animation,
            transition,
        } = &sample.targets[0].presentation
        else {
            panic!("literal CSS width must be certified: {sample:?}");
        };
        assert_eq!(
            animation,
            &[PresentedProperty {
                name: "width".into(),
                value: "150px".into()
            }]
        );
        assert!(transition.is_empty());
        assert!(eval(scope, "__layoutCalls===0").is_true());
        eval(scope,"target.style.animation='none';target.style.transition='width 1000ms linear';target.style.setProperty('width','100px','important');getComputedStyle(target).width;target.style.setProperty('width','200px','important');globalThis.transition=target.getAnimations().find(a=>a instanceof CSSTransition);transition.pause();transition.currentTime=500;__layoutCalls=0");
        let sample = sample_presented_effects(scope, document, &[handle]).unwrap();
        let PresentedTargetState::Certified {
            animation,
            transition,
        } = &sample.targets[0].presentation
        else {
            panic!("literal transition must be certified: {sample:?}");
        };
        assert!(animation.is_empty());
        assert_eq!(
            transition,
            &[PresentedProperty {
                name: "width".into(),
                value: "150px".into()
            }]
        );
        assert!(eval(scope, "__layoutCalls===0").is_true());
    });
}

#[test]
fn entire_target_batch_uses_one_canonical_clock_sample() {
    with_dom(|scope| {
        eval(scope,"for(let i=0;i<2;i++){const el=document.createElement('div');el.style.width='100px';document.body.append(el);const a=el.animate([{width:'100px'},{width:'200px'}],{duration:1000,fill:'both'});a.pause();a.currentTime=500}");
        eval(scope, "__clockCalls=0;__steppingClock=true");
        let document = object(scope, "document");
        let sample = sample_presented_effects(scope, document, &[]).unwrap();
        assert_eq!(sample.targets.len(), 2);
        assert_eq!(sample.sample_time, Some(1001.0));
        assert!(eval(scope, "__clockCalls===1 && __layoutCalls===0").is_true());
    });
}

#[test]
fn unsupported_reference_box_interpolation_remains_unknown_without_layout() {
    with_dom(|scope| {
        eval(scope,"const target=document.getElementById('target');const a=target.animate([{transform:'translateX(0%)'},{transform:'translateX(50%)'}],{duration:1000,fill:'both'});a.pause();a.currentTime=500");
        let document = object(scope, "document");
        let sample = sample_presented_effects(scope, document, &[]).unwrap();
        let PresentedTargetState::Unknown { properties } = &sample.targets[0].presentation else {
            panic!("uncertified reference-box transform must stay Unknown");
        };
        assert_eq!(properties, &["transform"]);
        assert!(eval(scope, "__layoutCalls===0").is_true());
    });
}

fn expose(scope: &mut v8::PinScope, name: &str, value: v8::Local<v8::Object>) {
    let key = v8::String::new(scope, name).unwrap();
    let global = scope.get_current_context().global(scope);
    assert_eq!(
        global.define_own_property(scope, key.into(), value.into(), v8::PropertyAttribute::NONE),
        Some(true)
    );
}

#[test]
fn borrowed_foreign_animation_cannot_become_an_empty_owner_certificate() {
    with_dom(|scope| {
        let document = object(scope, "document");
        let target = object(scope, "document.getElementById('target')");
        let foreign = v8::Context::new(scope, Default::default());
        {
            let inner = &mut v8::ContextScope::new(scope, foreign);
            browser_env::install_independent_html(inner, "<!doctype html><main>other</main>");
            expose(inner, "foreignTarget", target);
            eval(inner, "globalThis.motion=Element.prototype.animate.call(foreignTarget,[{width:'100px'},{width:'200px'}],{duration:1000,fill:'both'});motion.pause();motion.currentTime=500");
        }
        assert!(sample_presented_effects(scope, document, &[]).is_none());
        assert!(eval(scope, "__layoutCalls===0").is_true());
    });
}

#[test]
fn adopting_an_animated_target_refuses_the_new_owner_without_rooting_a_tree_copy() {
    with_dom(|scope| {
        eval(scope, "globalThis.target=document.getElementById('target');globalThis.motion=target.animate([{width:'100px'},{width:'200px'}],{duration:1000,fill:'both'});motion.pause();motion.currentTime=500");
        let document = object(scope, "document");
        let target = object(scope, "target");
        assert_eq!(
            sample_presented_effects(scope, document, &[])
                .unwrap()
                .targets
                .len(),
            1
        );
        let foreign = v8::Context::new(scope, Default::default());
        {
            let inner = &mut v8::ContextScope::new(scope, foreign);
            browser_env::install_independent_html(inner, "<!doctype html><main>other</main>");
            expose(inner, "foreignTarget", target);
            eval(
                inner,
                "document.body.appendChild(document.adoptNode(foreignTarget))",
            );
            let new_document = object(inner, "document");
            assert!(sample_presented_effects(inner, new_document, &[]).is_none());
        }
        assert!(sample_presented_effects(scope, document, &[])
            .unwrap()
            .targets
            .is_empty());
    });
}

fn animated_value<'a>(target: &'a PresentedTarget, property: &str) -> &'a str {
    let PresentedTargetState::Certified { animation, .. } = &target.presentation else {
        panic!("expected certified presentation: {target:?}");
    };
    &animation
        .iter()
        .find(|pair| pair.name == property)
        .unwrap_or_else(|| panic!("missing {property}: {target:?}"))
        .value
}

#[test]
fn closed_shadow_keyframes_use_their_actual_scope_and_native_owner() {
    with_dom(|scope| {
        eval(scope,"const host=document.getElementById('target');globalThis.shadow=host.attachShadow({mode:'closed'});shadow.innerHTML='<style>@keyframes grow{from{width:100px}to{width:200px}}#inside{width:100px;animation:grow 1000ms linear both}</style><div id=inside></div>';globalThis.inside=shadow.querySelector('#inside');const a=inside.getAnimations()[0];a.pause();a.currentTime=500;__layoutCalls=0");
        let document = object(scope, "document");
        let inside = object(scope, "inside");
        let handle = handle_of(scope, inside).unwrap();
        let sample = sample_presented_effects(scope, document, &[handle]).unwrap();
        assert_eq!(sample.targets.len(), 1);
        assert_eq!(sample.targets[0].element, handle);
        assert_eq!(animated_value(&sample.targets[0], "width"), "150px");
        assert!(eval(scope, "__layoutCalls===0").is_true());
    });
}

#[test]
fn absolute_inherited_font_and_unregistered_custom_tokens_are_origin_inputs() {
    with_dom(|scope| {
        eval(scope,"globalThis.target=document.getElementById('target');const s=document.createElement('style');s.textContent='@keyframes text{from{font-size:10px;--measure:100px}to{font-size:30px;--measure:200px}}#target{--measure:100px;width:var(--measure);animation:text 1000ms linear both}';document.head.append(s);target.innerHTML='<span>inherits presented font</span>';const a=target.getAnimations()[0];a.pause();a.currentTime=500;__layoutCalls=0");
        let document = object(scope, "document");
        let target = object(scope, "target");
        let handle = handle_of(scope, target).unwrap();
        let sample = sample_presented_effects(scope, document, &[handle]).unwrap();
        assert!(eval(scope, "__layoutCalls===0").is_true());
        assert!(eval(
            scope,
            "target.getAnimations()[0].effect.getKeyframes()[0]['--measure']===undefined && getComputedStyle(target).getPropertyValue('--measure')==='200px'"
        )
        .is_true());
        assert_eq!(animated_value(&sample.targets[0], "font-size"), "20px");
        assert_eq!(animated_value(&sample.targets[0], "--measure"), "200px");
    });
}

#[test]
fn independent_compatibility_switches_preserve_script_and_presentation_policy() {
    with_dom(|scope| {
        eval(scope,"globalThis.target=document.getElementById('target');globalThis.motion=target.animate([{width:'100px'},{width:'200px'}],{duration:1000,fill:'both'});motion.pause();motion.currentTime=500");
        browser_env::set_css_animations(scope, false);
        browser_env::set_css_transitions(scope, false);
        browser_env::set_media_mode(scope, browser_env::MediaMode::Jsdom);
        browser_env::set_touch_event_mode(scope, browser_env::TouchEventMode::Jsdom);
        browser_env::set_computed_style_mode(scope, browser_env::ComputedStyleMode::Jsdom);
        let document = object(scope, "document");
        let sample = sample_presented_effects(scope, document, &[]).unwrap();
        assert!(!sample.css_animations_enabled);
        assert!(!sample.css_transitions_enabled);
        assert_eq!(animated_value(&sample.targets[0], "width"), "150px");
        assert!(eval(scope, "typeof Element.prototype.animate==='undefined'").is_true());
        set_animation_presentation(scope, AnimationPresentation::Disabled);
        let suppressed = sample_presented_effects(scope, document, &[]).unwrap();
        assert!(suppressed.presentation_suppressed);
        let PresentedTargetState::Certified {
            animation,
            transition,
        } = &suppressed.targets[0].presentation
        else {
            panic!("trusted presentation suppression");
        };
        assert!(animation.is_empty() && transition.is_empty());
        set_animation_presentation(scope, AnimationPresentation::Enabled);
        let restored = sample_presented_effects(scope, document, &[]).unwrap();
        assert_eq!(animated_value(&restored.targets[0], "width"), "150px");
        assert!(eval(scope, "motion.currentTime===500 && __layoutCalls===0").is_true());
    });
}

#[test]
fn important_keyframe_declarations_do_not_become_presented_width() {
    with_dom(|scope| {
        eval(scope,"globalThis.target=document.getElementById('target');const s=document.createElement('style');s.textContent='@keyframes ignored{from{width:100px!important}to{width:200px!important}}#target{width:300px;animation:ignored 1000ms linear both}';document.head.append(s);const a=target.getAnimations()[0];a.pause();a.currentTime=500;__layoutCalls=0");
        let document = object(scope, "document");
        let target = object(scope, "target");
        let handle = handle_of(scope, target).unwrap();
        let sample = sample_presented_effects(scope, document, &[handle]).unwrap();
        assert!(eval(scope, "__layoutCalls===0").is_true());
        let PresentedTargetState::Certified {
            animation,
            transition,
        } = &sample.targets[0].presentation
        else {
            panic!("ignored important keyframes must have no presentation");
        };
        assert!(animation.is_empty() && transition.is_empty(), "{sample:?}");
        assert!(eval(scope,"target.getAnimations()[0].effect.getKeyframes().every(frame=>!Object.hasOwn(frame,'width'))").is_true());
    });
}

#[test]
fn unmodeled_css_declarations_cannot_be_certified_as_an_empty_effect() {
    with_dom(|scope| {
        eval(scope,"globalThis.target=document.getElementById('target');const s=document.createElement('style');s.textContent='@keyframes floatIt{from{float:left}to{float:right}}#target{animation:floatIt 1000ms linear both}';document.head.append(s);const a=target.getAnimations()[0];a.pause();a.currentTime=500;__layoutCalls=0");
        let document = object(scope, "document");
        let target = object(scope, "target");
        let handle = handle_of(scope, target).unwrap();
        let sample = sample_presented_effects(scope, document, &[handle]).unwrap();
        assert!(
            matches!(
                sample.targets[0].presentation,
                PresentedTargetState::Unknown { .. }
            ),
            "unmodeled float must not become an empty certificate: {sample:?}"
        );
        assert!(eval(scope, "__layoutCalls===0").is_true());
    });
}

#[test]
fn registered_custom_properties_are_unknown_instead_of_unregistered_discrete_values() {
    with_dom(|scope| {
        eval(
            scope,
            r#"globalThis.target=document.getElementById('target');const s=document.createElement('style');s.textContent='@property --measure{syntax: "<length>";inherits:false;initial-value:0px}@keyframes measure{from{--measure:100px}to{--measure:200px}}#target{animation:measure 1000ms linear both}';document.head.append(s);const a=target.getAnimations()[0];a.pause();a.currentTime=500;__layoutCalls=0"#,
        );
        let document = object(scope, "document");
        let target = object(scope, "target");
        let handle = handle_of(scope, target).unwrap();
        let sample = sample_presented_effects(scope, document, &[handle]).unwrap();
        assert!(
            matches!(
                sample.targets[0].presentation,
                PresentedTargetState::Unknown { .. }
            ),
            "registered interpolation must not be certified discrete: {sample:?}"
        );
        assert!(eval(scope, "__layoutCalls===0").is_true());
    });
}

#[test]
fn repaired_non_scalar_custom_payload_is_unknown_instead_of_a_lossy_certificate() {
    with_dom(|scope| {
        eval(
            scope,
            r#"const target=document.getElementById('target'),raw='"'+String.fromCharCode(0xd800)+'"';const a=target.animate([{'--unit':raw},{'--unit':raw}],{duration:1000,fill:'both'});a.pause();a.currentTime=500;__layoutCalls=0"#,
        );
        let document = object(scope, "document");
        let sample = sample_presented_effects(scope, document, &[]).unwrap();
        assert!(
            matches!(
                sample.targets[0].presentation,
                PresentedTargetState::Unknown { .. }
            ),
            "lossy native value transport must not be certified: {sample:?}"
        );
        assert!(eval(scope, "__layoutCalls===0").is_true());
    });
}

#[test]
fn presented_sampling_survives_startup_snapshot_rehydration() {
    init_v8();
    let mut window = browser_env::install_from_snapshot(
        "<!doctype html><div id=target style='width:100px'></div>",
    )
    .unwrap();
    let (isolate, context) = window.parts();
    isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
    {
        v8::scope!(let scope, isolate);
        let context = v8::Local::new(scope, context);
        let scope = &mut v8::ContextScope::new(scope, context);
        browser_env::run_engine_script(scope,"globalThis.__layoutCalls=0;__tsNs.layoutUsedSize=()=>{__layoutCalls++;throw Error('provider called')};__tsNs.resolveTransform=()=>{__layoutCalls++;throw Error('transform called')}").unwrap();
        eval(scope,"const target=document.getElementById('target');const a=target.animate([{width:'100px'},{width:'200px'}],{duration:1000,fill:'both'});a.pause();a.currentTime=500");
        let document = object(scope, "document");
        let sample =
            sample_presented_effects(scope, document, &[]).expect("rehydrated canonical sampler");
        assert_eq!(sample.targets.len(), 1);
        assert_eq!(animated_value(&sample.targets[0], "width"), "150px");
        assert!(eval(scope, "__layoutCalls===0").is_true());
    }
    browser_env::reset();
}

#[test]
fn svg_unitless_dash_offsets_use_canonical_length_interpolation() {
    with_dom(|scope| {
        eval(scope,"const s=document.createElement('style');s.textContent='@keyframes dash{from{stroke-dashoffset:0}to{stroke-dashoffset:100}}#shape{animation:dash 1000ms linear infinite}';document.head.append(s);document.body.innerHTML='<svg width=100 height=100><circle id=shape cx=50 cy=50 r=30 /></svg>';globalThis.shape=document.getElementById('shape');const a=shape.getAnimations()[0];a.pause();a.currentTime=500;__layoutCalls=0");
        let document = object(scope, "document");
        let shape = object(scope, "shape");
        let handle = handle_of(scope, shape).unwrap();
        let sample = sample_presented_effects(scope, document, &[handle]).unwrap();
        assert_eq!(
            animated_value(&sample.targets[0], "stroke-dashoffset"),
            "50px"
        );
        assert!(eval(scope, "__layoutCalls===0").is_true());
    });
}

#[test]
fn unregistered_custom_names_do_not_inherit_standard_property_interpolation_types() {
    with_dom(|scope| {
        eval(scope,"const target=document.getElementById('target');const a=target.animate([{'--width':'100px'},{'--width':'200px'}],{duration:1000,fill:'both'});a.pause();a.currentTime=500;__layoutCalls=0");
        let document = object(scope, "document");
        let sample = sample_presented_effects(scope, document, &[]).unwrap();
        assert_eq!(animated_value(&sample.targets[0], "--width"), "200px");
        assert!(eval(scope, "__layoutCalls===0").is_true());
    });
}

#[test]
fn simultaneous_isolates_keep_animation_ownership_separate() {
    with_dom(|outer| {
        eval(outer, "globalThis.effect=document.getElementById('target').animate([{width:'100px'},{width:'200px'}],{duration:1000,fill:'both'});effect.pause();effect.currentTime=500");
        // Creating a nested isolate enters it; dropping it restores the outer isolate.
        with_dom_lifecycle(true, false, |inner| {
            eval(inner, "globalThis.effect=document.getElementById('target').animate([{width:'100px'},{width:'300px'}],{duration:1000,fill:'both'});effect.pause();effect.currentTime=500");
            let document = object(inner, "document");
            let snapshot = sample_presented_effects(inner, document, &[]).unwrap();
            assert_eq!(snapshot.targets.len(), 1);
            assert!(matches!(&snapshot.targets[0].presentation,
                PresentedTargetState::Certified { animation, .. }
                if animation.iter().any(|pair| pair.name == "width" && pair.value == "200px")));
        });
        assert!(eval(outer, "effect.cancel();effect.playState==='idle'").is_true());
    });
    // A replacement isolate must not inherit weak handles from either destroyed isolate.
    discovers_paused_waapi_without_css_candidates_and_never_calls_layout();
}

#[test]
fn inherited_animated_context_cannot_certify_a_suppressed_additive_base() {
    with_dom(|scope| {
        eval(scope, "document.body.style.fontSize='10px';target=document.getElementById('target');target.style.width='calc(10em * sibling-index())';let parentEffect=document.body.animate([{fontSize:'10px'},{fontSize:'30px'}],{duration:1000,fill:'both'});parentEffect.pause();parentEffect.currentTime=500;let childEffect=target.animate([{width:'10px'},{width:'20px'}],{duration:1000,fill:'both',composite:'add'});childEffect.pause();childEffect.currentTime=500");
        let document = object(scope, "document");
        let target = object(scope, "target");
        let handle = handle_of(scope, target).unwrap();
        let sample = sample_presented_effects(scope, document, &[]).unwrap();
        let target = sample
            .targets
            .iter()
            .find(|target| target.element == handle)
            .unwrap();
        assert!(
            matches!(target.presentation, PresentedTargetState::Unknown { .. }),
            "suppressed 10em base cannot certify presented inherited font: {target:?}"
        );
    });
}

#[test]
fn imported_registration_cannot_certify_unregistered_custom_interpolation() {
    with_dom(|scope| {
        eval(
            scope,
            r#"let sheet=document.createElement('style');sheet.textContent='@import url("data:text/css,@property%20--size%7Bsyntax:%22%3Clength%3E%22;inherits:false;initial-value:0px%7D");';document.head.appendChild(sheet);target=document.getElementById('target');let effect=target.animate([{'--size':'100px'},{'--size':'200px'}],{duration:1000,fill:'both'});effect.pause();effect.currentTime=500"#,
        );
        let document = object(scope, "document");
        let sample = sample_presented_effects(scope, document, &[]).unwrap();
        assert!(
            matches!(
                sample.targets[0].presentation,
                PresentedTargetState::Unknown { .. }
            ),
            "imported registration completeness is unknown: {sample:?}"
        );
    });
}

#[test]
fn canonical_sampling_preserves_internal_exceptions() {
    with_dom(|scope| {
        browser_env::run_engine_script(
            scope,
            "__tsNs.refreshCssEffectInputs=()=>{throw Error('canonical sampling sentinel')}",
        )
        .unwrap();
        let document = object(scope, "document");
        let target = object(scope, "document.getElementById('target')");
        let handle = handle_of(scope, target).unwrap();
        v8::tc_scope!(let scope, scope);
        assert!(sample_presented_effects(scope, document, &[handle]).is_none());
        assert!(
            scope.has_caught(),
            "canonical failures must not become an unavailable certificate"
        );
        let exception = scope.exception().unwrap();
        assert!(exception
            .to_rust_string_lossy(scope)
            .contains("canonical sampling sentinel"));
    });
}

fn presented_animation_pairs(scope: &mut v8::PinScope) -> Vec<PresentedProperty> {
    let document = object(scope, "document");
    let sample = sample_presented_effects(scope, document, &[]).expect("owned canonical sample");
    assert_eq!(sample.targets.len(), 1);
    let PresentedTargetState::Certified { animation, .. } = &sample.targets[0].presentation else {
        panic!("modeled absolute effect must be certified: {sample:?}");
    };
    animation.clone()
}

#[test]
fn actual_mui_svg_dasharray_uses_canonical_piecewise_list_interpolation() {
    with_dom(|scope| {
        eval(scope, "document.body.innerHTML='<svg width=100 height=100><circle id=subject cx=50 cy=50 r=30 fill=none stroke=red /></svg>';const s=document.createElement('style');s.textContent='@keyframes motion{0%{stroke-dasharray:1px,200px;stroke-dashoffset:0px}50%{stroke-dasharray:100px,200px;stroke-dashoffset:-15px}100%{stroke-dasharray:1px,200px;stroke-dashoffset:-126px}}#subject{animation:motion 1000ms linear both}';document.head.append(s);globalThis.subject=document.getElementById('subject');globalThis.effect=subject.getAnimations()[0];effect.pause();__layoutCalls=0");
        for (time, array, offset) in [
            (0, "1px, 200px", "0px"),
            (250, "50.5px, 200px", "-7.5px"),
            (500, "100px, 200px", "-15px"),
            (750, "50.5px, 200px", "-70.5px"),
            (1000, "1px, 200px", "-126px"),
        ] {
            eval(scope, &format!("effect.currentTime={time};__layoutCalls=0"));
            let pairs = presented_animation_pairs(scope);
            for (name, value) in [("stroke-dasharray", array), ("stroke-dashoffset", offset)] {
                assert!(
                    pairs.contains(&PresentedProperty {
                        name: name.into(),
                        value: value.into()
                    }),
                    "{time} {pairs:?}"
                );
            }
            assert!(eval(scope, "__layoutCalls===0").is_true());
            assert!(eval(scope, &format!("getComputedStyle(subject).strokeDasharray==='{array}' && getComputedStyle(subject).strokeDashoffset==='{offset}'")).is_true());
        }
    });
}

#[test]
fn repeatable_svg_lists_normalize_units_and_repeat_to_least_common_multiple() {
    with_dom(|scope| {
        eval(scope, "document.body.innerHTML='<svg><circle id=subject /></svg>';globalThis.subject=document.getElementById('subject')");
        for (from, to, expected) in [
            ("1 200", "100 200", "50.5px, 200px"),
            ("2 4", "10 20 30", "6px, 12px, 16px, 7px, 11px, 17px"),
            ("2", "10 20 30", "6px, 11px, 16px"),
            ("1in 2px", "192px 4px", "144px, 3px"),
            ("10% 20%", "30% 40%", "20%, 30%"),
        ] {
            eval(scope, &format!("if(globalThis.effect)effect.cancel();globalThis.effect=subject.animate([{{strokeDasharray:'{from}'}},{{strokeDasharray:'{to}'}}],{{duration:1000,fill:'both'}});effect.pause();effect.currentTime=500;__layoutCalls=0"));
            let pairs = presented_animation_pairs(scope);
            assert!(
                pairs.contains(&PresentedProperty {
                    name: "stroke-dasharray".into(),
                    value: expected.into()
                }),
                "{from} -> {to}: {pairs:?}"
            );
            assert!(eval(scope, "__layoutCalls===0").is_true());
            assert!(eval(
                scope,
                &format!("getComputedStyle(subject).strokeDasharray==='{expected}'")
            )
            .is_true());
        }
    });
}

#[test]
fn svg_list_uncertainties_remain_unknown_and_nonadditive_values_do_not_sum() {
    with_dom(|scope| {
        eval(scope, "document.body.innerHTML='<svg><circle id=subject style=\"stroke-dasharray:1,2\" /></svg>';globalThis.subject=document.getElementById('subject')");
        for composite in ["add", "accumulate"] {
            eval(scope, &format!("if(globalThis.effect)effect.cancel();globalThis.effect=subject.animate([{{strokeDasharray:'10 20'}},{{strokeDasharray:'30 40'}}],{{duration:1000,fill:'both',composite:'{composite}'}});effect.pause();effect.currentTime=500;__layoutCalls=0"));
            let document = object(scope, "document");
            let sample = sample_presented_effects(scope, document, &[]).unwrap();
            assert!(matches!(
                sample.targets[0].presentation,
                PresentedTargetState::Unknown { .. }
            ));
            assert!(eval(scope, "__layoutCalls===0").is_true());
            assert!(eval(
                scope,
                "getComputedStyle(subject).strokeDasharray==='20px, 30px'"
            )
            .is_true());
        }
        eval(scope,"effect.cancel();effect=subject.animate([{strokeDasharray:'10% 20px'},{strokeDasharray:'30px 40%'}],{duration:1000,fill:'both'});effect.pause();effect.currentTime=500");
        let document = object(scope, "document");
        let sample = sample_presented_effects(scope, document, &[]).unwrap();
        assert!(matches!(
            sample.targets[0].presentation,
            PresentedTargetState::Unknown { .. }
        ));
    });
}

#[test]
fn chrome_ignored_ms_transform_does_not_hide_supported_css_effects() {
    with_dom(|scope| {
        for (from, to, expected) in [
            (
                "-ms-transform:rotate(0deg);transform:rotate(0deg)",
                "-ms-transform:rotate(360deg);transform:rotate(360deg)",
                "matrix(-1, 0, 0, -1, 0, 0)",
            ),
            (
                "-ms-transform:scale(0);transform:scale(0);opacity:0.1",
                "-ms-transform:scale(1);transform:scale(1);opacity:0.3",
                "matrix(0.5, 0, 0, 0.5, 0, 0)",
            ),
        ] {
            eval(scope, &format!("{{document.head.innerHTML='';const s=document.createElement('style');s.textContent='@keyframes motion{{from{{{from}}}to{{{to}}}}}#target{{animation:motion 1000ms linear both}}';document.head.append(s);globalThis.effect=document.getElementById('target').getAnimations()[0];effect.pause();effect.currentTime=500;__layoutCalls=0}}"));
            assert!(eval(scope, "document.styleSheets[0].cssRules[0].cssRules[0].style.getPropertyValue('-ms-transform')==='' && !('-ms-transform' in effect.effect.getKeyframes()[0])").is_true());
            let pairs = presented_animation_pairs(scope);
            let transform = pairs.iter().find(|pair| pair.name == "transform").unwrap();
            assert_matrix_close(&transform.value, expected);
            assert!(eval(scope, "__layoutCalls===0").is_true());
        }
    });
}

#[test]
fn repeatable_svg_list_expansion_is_bounded_before_composition() {
    with_dom(|scope| {
        eval(scope,"document.body.innerHTML='<svg><circle id=subject /></svg>';globalThis.subject=document.getElementById('subject');globalThis.effect=subject.animate([{strokeDasharray:Array(17).fill('1').join(' ')},{strokeDasharray:Array(19).fill('2').join(' ')}],{duration:1000,fill:'both'});effect.pause();effect.currentTime=500;__layoutCalls=0");
        let document = object(scope, "document");
        let sample = sample_presented_effects(scope, document, &[]).unwrap();
        assert!(matches!(
            sample.targets[0].presentation,
            PresentedTargetState::Unknown { .. }
        ));
        assert!(eval(scope, "__layoutCalls===0").is_true());
        assert!(eval(scope, "(()=>{try{getComputedStyle(subject).strokeDasharray;return false}catch(error){return error instanceof RangeError && /source-derived work budget/.test(error.message)}})()").is_true());
    });
}

#[test]
fn captured_mui_effect_sources_are_certified_at_all_fifteen_oracle_samples() {
    use sha2::{Digest, Sha256};
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/captured_mui_animation_chrome159.json"
    ))
    .unwrap();
    assert_eq!(fixture["oracleVersion"], "159.0.8154.6");
    for case in fixture["cases"].as_array().unwrap() {
        let html = case["html"].as_str().unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(html)),
            case["htmlSHA256"].as_str().unwrap()
        );
        with_html(true, true, html, |scope| {
            eval(scope,"globalThis.subject=document.getElementById('subject');globalThis.fixtureEffects=document.getAnimations();for(const effect of fixtureEffects)effect.pause()");
            for sample in case["samples"].as_array().unwrap() {
                let time = sample["time"].as_u64().unwrap();
                eval(scope, &format!("for(const effect of fixtureEffects)effect.currentTime={time};__layoutCalls=0"));
                let pairs = presented_animation_pairs(scope);
                assert!(eval(scope, "__layoutCalls===0").is_true());
                if case["kind"] == "svg" {
                    let array = pairs
                        .iter()
                        .find(|pair| pair.name == "stroke-dasharray")
                        .unwrap();
                    let values: Vec<f64> = array
                        .value
                        .split(',')
                        .map(|value| value.trim().trim_end_matches("px").parse().unwrap())
                        .collect();
                    let expected: Vec<f64> = sample["observation"]["dasharray"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|value| value.as_f64().unwrap())
                        .collect();
                    assert_eq!(values, expected, "{} time{time}", case["id"]);
                    let offset = pairs
                        .iter()
                        .find(|pair| pair.name == "stroke-dashoffset")
                        .unwrap()
                        .value
                        .trim_end_matches("px")
                        .parse::<f64>()
                        .unwrap();
                    assert_eq!(
                        offset,
                        sample["observation"]["dashoffset"].as_f64().unwrap()
                    );
                } else {
                    assert!(
                        pairs.iter().any(|pair| pair.name == "transform"),
                        "{} time{time} {pairs:?}",
                        case["id"]
                    );
                    assert!(eval(scope,"Array.from(document.styleSheets[0].cssRules).find(rule=>rule.type===7).cssRules[0].style.getPropertyValue('-ms-transform')===''").is_true());
                    assert!(browser_env::run_engine_script(
                        scope,
                        "return __tsNs.cssSupportsProperty('-ms-transform','rotate(1deg)')===false"
                    )
                    .unwrap()
                    .is_true());
                }
            }
        });
    }
}

fn assert_matrix_close(actual: &str, expected: &str) {
    let values = |text: &str| -> Vec<f64> {
        text.trim_start_matches("matrix(")
            .trim_end_matches(')')
            .split(',')
            .map(|part| part.trim().parse().unwrap())
            .collect()
    };
    let actual = values(actual);
    let expected = values(expected);
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.iter().zip(expected) {
        assert!(
            (actual - expected).abs() < 0.00001,
            "matrix component {actual} != {expected}"
        );
    }
}

#[test]
fn captured_mui_effects_are_certified_without_pausing_or_seeking() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/captured_mui_animation_chrome159.json"
    ))
    .unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        with_html(true, true, case["html"].as_str().unwrap(), |scope| {
            let document = object(scope, "document");
            let subject = object(scope, "document.getElementById('subject')");
            let handle = handle_of(scope, subject).unwrap();
            let sample = sample_presented_effects(scope, document, &[handle]).unwrap();
            assert_eq!(sample.targets.len(), 1);
            assert!(
                matches!(&sample.targets[0].presentation, PresentedTargetState::Certified { animation,.. } if !animation.is_empty()),
                "{} {sample:?}",
                case["id"]
            );
            assert!(sample.targets[0]
                .effects
                .iter()
                .all(|effect| !effect.script_modified));
            assert!(eval(scope, "__layoutCalls===0").is_true());
        });
    }
}

#[test]
fn legacy_vendor_admission_preserves_webkit_and_custom_controls() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/vendor_animation_properties_chrome159.json"
    ))
    .unwrap();
    with_dom(|scope| {
        for row in fixture["output"].as_array().unwrap() {
            let name = serde_json::to_string(row["property"].as_str().unwrap()).unwrap();
            let supported = browser_env::run_engine_script(
                scope,
                &format!("return __tsNs.cssSupportsProperty({name},'rotate(1deg)')"),
            )
            .unwrap();
            assert_eq!(
                supported.is_true(),
                row["supports"].as_bool().unwrap(),
                "{}",
                row["property"]
            );
            let expected = serde_json::to_string(row["value"].as_str().unwrap()).unwrap();
            assert!(eval(scope,&format!("(()=>{{const el=document.createElement('div');el.style.setProperty({name},'rotate(1deg)');return el.style.getPropertyValue({name})==={expected}}})()")).is_true(),"{}",row["property"]);
        }
    });
}
