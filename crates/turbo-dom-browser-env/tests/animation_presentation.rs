#![cfg(feature = "host-internals")]
use std::sync::Once;
use turbo_dom_browser_env::{browser_env, host_internals::*};

const HTML: &str = r#"<!doctype html><style>
@keyframes fadeInUp {from {opacity:0;transform:translateY(8px)} to {opacity:1;transform:translateY(0)}}
@keyframes other {from {opacity:0} to {opacity:1}}
#message {animation:fadeInUp 0.3s ease-out}
button {width:32px;height:32px;padding:0;border:0}
</style><div id="message"><button id="feedback">Vote</button></div>"#;

fn eval<'s>(scope: &mut v8::PinScope<'s, '_>, source: &str) -> v8::Local<'s, v8::Value> {
    let source = v8::String::new(scope, source).unwrap();
    v8::Script::compile(scope, source, None)
        .unwrap()
        .run(scope)
        .unwrap()
}
fn element<'s>(scope: &mut v8::PinScope<'s, '_>, source: &str) -> v8::Local<'s, v8::Object> {
    v8::Local::try_from(eval(scope, source)).unwrap()
}
fn frame(scope: &mut v8::PinScope, delta: u32) {
    eval(scope, &format!("__frame({delta})"));
    scope.perform_microtask_checkpoint();
}
fn sample(scope: &mut v8::PinScope) -> CssEffectSnapshot {
    let target = element(scope, "message");
    css_effect_lifecycle(scope, target).expect("available host-only lifecycle")
}
fn with_dom(test: impl FnOnce(&mut v8::PinScope)) {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let platform = v8::new_default_platform(0, false).make_shared();
        v8::V8::initialize_platform(platform);
        v8::V8::initialize();
    });
    let isolate = &mut v8::Isolate::new(snapshot_params());
    isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    browser_env::install_independent_html(scope, HTML);
    browser_env::run_engine_script(scope, r#"
      let now=1000, handler=null, tasks=[];
      __tsNs.installAnimationHost({now:()=>now,frame(next){handler=next},task(next){tasks.push(next)}});
      globalThis.__jump=delta=>{now+=delta};
      globalThis.__frame=delta=>{now+=delta;const next=handler;handler=null;
        if(next){next.update();next.dispatch();next.commit()}
        const pending=tasks;tasks=[];for(const task of pending)task();};
      // Inspection must never enter either native-layout seam, even with percentage transforms.
      globalThis.__layoutCalls=0;
      __tsNs.layoutUsedSize=()=>{globalThis.__layoutCalls++;throw new Error('layout reentry')};
      __tsNs.resolveTransform=()=>{globalThis.__layoutCalls++;throw new Error('transform resolution reentry')};
    "#).unwrap();
    eval(
        scope,
        "globalThis.message=document.getElementById('message')",
    );
    test(scope);
    browser_env::reset();
}

fn snapshot_params() -> v8::CreateParams {
    match std::env::var_os("TURBO_DOM_SNAPSHOT_PATH").and(browser_env::snapshot_blob()) {
        Some(blob) => blob.create_params(),
        None => Default::default(),
    }
}

#[test]
fn presentation_proof_retains_real_lifecycle_and_underlying_style_changes() {
    with_dom(|scope| {
        assert!(!sample(scope).presentation_suppressed);
        assert!(!sample(scope).allows_underlying_geometry());
        assert!(set_animation_presentation(
            scope,
            AnimationPresentation::Disabled
        ));
        for declaration in [
            "fadeInUp 0.3s",
            "fadeInUp 0.3s infinite",
            "fadeInUp 0.3s paused",
            "fadeInUp 0.3s both",
            "unknown 0.3s",
        ] {
            eval(
                scope,
                &format!("message.style.animation={declaration:?};message.style.opacity='0.6'"),
            );
            let snapshot = sample(scope);
            assert!(
                snapshot.presentation_suppressed && snapshot.allows_underlying_geometry(),
                "{snapshot:?}"
            );
            assert!(snapshot.css_animations_enabled && snapshot.css_transitions_enabled);
            assert!(eval(scope,"getComputedStyle(message).opacity==='0.6' && typeof message.animate==='function' && typeof document.timeline==='object'").is_true());
        }
        eval(scope,"message.style.animation='none';globalThis.a=message.animate({opacity:[0,1]},{duration:100,fill:'both'});a.pause();a.currentTime=50");
        let script = sample(scope);
        let script_effect = script
            .effects
            .iter()
            .find(|effect| effect.kind == CssEffectKind::Script)
            .expect("script effect remains indexed alongside retained CSS records");
        assert!(script_effect.contributes);
        assert!(script.allows_underlying_geometry());
        eval(scope, "message.style.opacity='0.7'");
        assert!(eval(scope, "getComputedStyle(message).opacity==='0.7'").is_true());
        assert!(set_animation_presentation(
            scope,
            AnimationPresentation::Enabled
        ));
        let enabled = sample(scope);
        assert!(!enabled.presentation_suppressed && !enabled.allows_underlying_geometry());
        assert!(eval(scope, "getComputedStyle(message).opacity==='0.5'").is_true());
        assert!(set_animation_presentation(
            scope,
            AnimationPresentation::Disabled
        ));
        assert!(eval(
            scope,
            "getComputedStyle(message).opacity==='0.7' && a.currentTime===50"
        )
        .is_true());
    });
}

#[test]
fn presentation_policy_is_owned_by_the_actual_window_not_callers_or_guest_flags() {
    with_dom(|scope| {
        assert!(set_animation_presentation(
            scope,
            AnimationPresentation::Disabled
        ));
        let disabled = sample(scope);
        let previous = scope.get_current_context();
        let context = v8::Context::new(scope, Default::default());
        let target = {
            let scope = &mut v8::ContextScope::new(scope, context);
            browser_env::install_independent_html(
                scope,
                "<div id=other style='animation:unknown 0.3s'></div>",
            );
            element(scope, "document.getElementById('other')")
        };
        let enabled = css_effect_lifecycle(scope, target).unwrap();
        assert!(!enabled.presentation_suppressed && !enabled.allows_underlying_geometry());
        assert_ne!(enabled.document_id, disabled.document_id);
        assert_eq!(scope.get_current_context(), previous);
        let uninstalled = v8::Context::new(scope, Default::default());
        {
            let scope = &mut v8::ContextScope::new(scope, uninstalled);
            eval(scope,"globalThis.calls=0;Object.defineProperty(Object.prototype,'setAnimationPresentation',{get(){calls++;throw Error('guest getter')}})");
            assert!(!set_animation_presentation(
                scope,
                AnimationPresentation::Disabled
            ));
            assert!(eval(scope, "calls===0").is_true());
        }

        eval(scope,"globalThis.presentation_suppressed=false;globalThis.setAnimationPresentation=()=>{};Object.defineProperty(message,'presentation_suppressed',{get(){throw Error('guest getter')}})");
        assert!(sample(scope).presentation_suppressed);
        let fake = element(scope, "Object.create(Element.prototype)");
        assert!(css_effect_lifecycle(scope, fake).is_none());
        {
            let scope = &mut v8::ContextScope::new(scope, context);
            retire_context(context);
            assert!(!set_animation_presentation(
                scope,
                AnimationPresentation::Disabled
            ));
        }
        assert!(css_effect_lifecycle(scope, target).is_none());
    });
}

#[test]
fn presentation_shadow_proof_requires_ownership_and_available_inspection() {
    with_dom(|scope| {
        assert!(set_animation_presentation(
            scope,
            AnimationPresentation::Disabled
        ));
        for mode in ["open", "closed"] {
            eval(
                scope,
                &format!(
                    r#"{{const host=document.createElement('div');document.body.append(host);const root=host.attachShadow({{mode:{mode:?}}});root.innerHTML='<style>@keyframes local{{to{{opacity:0}}}}div{{animation:local 1s infinite}}</style><div></div>';globalThis.shadowTarget=root.lastChild;}}"#
                ),
            );
            let target = element(scope, "shadowTarget");
            let snapshot = css_effect_lifecycle(scope, target).unwrap();
            assert!(snapshot.presentation_suppressed && snapshot.allows_underlying_geometry());
            assert_eq!(snapshot.element, handle_of(scope, target).unwrap());
        }
        browser_env::run_engine_script(
            scope,
            "__tsNs.refreshCssEffectInputs=()=>{throw Error('unavailable')}",
        )
        .unwrap();
        let target = element(scope, "message");
        assert!(css_effect_lifecycle(scope, target).is_none());
    });
}

#[test]
fn presentation_inspection_never_drives_frames_or_guest_microtasks_under_suppression() {
    with_dom(|scope| {
        assert!(set_animation_presentation(
            scope,
            AnimationPresentation::Disabled
        ));
        eval(scope,"globalThis.jobs=0;globalThis.a=message.animate({opacity:[0,1]},{duration:100,iterations:Infinity});a.ready.then(()=>jobs++);a.finished.then(()=>jobs++,()=>jobs++)");
        let initial = sample(scope);
        for _ in 0..3 {
            let next = sample(scope);
            assert_eq!(next.sample_time, initial.sample_time);
            assert_eq!(next.effects.len(), initial.effects.len());
            assert!(eval(scope, "jobs===0").is_true());
        }
        frame(scope, 16);
        assert!(eval(scope, "jobs===1 && a.playState==='running'").is_true());
        frame(scope, 1000);
        assert!(eval(scope, "jobs===1 && a.playState==='running'").is_true());
    });
}

#[test]
fn presentation_disabling_preserves_an_already_running_transition() {
    with_dom(|scope| {
        eval(scope,"message.style.cssText='animation:none;opacity:0.2;transition:opacity 100ms linear';getComputedStyle(message).opacity");
        frame(scope, 16);
        eval(
            scope,
            "message.style.opacity='0.8';getComputedStyle(message).opacity",
        );
        frame(scope, 16);
        let before = sample(scope);
        let transition = before
            .effects
            .iter()
            .find(|effect| effect.kind == CssEffectKind::Transition)
            .unwrap();
        let identity = transition.identity;
        assert_eq!(transition.play_state, "running");
        assert!(set_animation_presentation(
            scope,
            AnimationPresentation::Disabled
        ));
        let after = sample(scope);
        let transition = after
            .effects
            .iter()
            .find(|effect| effect.identity == identity)
            .unwrap();
        assert_eq!(transition.play_state, "running");
        assert!(after.presentation_suppressed && after.allows_underlying_geometry());
        assert!(eval(scope, "getComputedStyle(message).opacity==='0.8'").is_true());
    });
}
