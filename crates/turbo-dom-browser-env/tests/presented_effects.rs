#![cfg(feature = "host-internals")]
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
    init_v8();
    let isolate = &mut v8::Isolate::new(snapshot_params());
    isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    let html = "<!doctype html><div id=target style='width:100px;height:20px'></div>";
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
    browser_env::reset();
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
