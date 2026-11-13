#![cfg(feature = "host-internals")]
use std::sync::Once;
use turbo_dom_browser_env::{browser_env, host_internals::*};

const HTML: &str = r#"<!doctype html><style>
@keyframes fadeInUp {from {opacity:0;transform:translateY(8px)} to {opacity:1;transform:translateY(0)}}
@keyframes other {from {opacity:0} to {opacity:1}}
#message {animation:fadeInUp .3s ease-out}
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

#[test]
fn completed_no_fill_is_trusted_only_after_canonical_finish_and_never_cached() {
    with_dom(|scope| {
        let pending = sample(scope);
        assert!(!pending.allows_underlying_geometry());
        assert!(pending.effects[0].pending);
        let identity = pending.effects[0].identity;
        frame(scope, 16);
        assert_eq!(sample(scope).effects[0].phase, CssEffectPhase::Active);
        frame(scope, 400);
        let done = sample(scope);
        assert!(eval(scope, "__layoutCalls===0").is_true());
        assert!(done.allows_underlying_geometry(), "{done:?}");
        assert_eq!(done.effects[0].identity, identity);
        assert!(!done.effects[0].contributes);
        assert!(eval(scope, "message.getAnimations().length===0").is_true());
        eval(
            scope,
            "message.style.transform='translateX(50%)';__layoutCalls=0",
        );
        assert!(sample(scope).allows_underlying_geometry());
        assert!(eval(scope, "__layoutCalls===0").is_true());
        eval(scope, "getComputedStyle(message).transform");
        assert!(eval(scope, "__layoutCalls>0").is_true());
        // A retained object is not proof after restart or declaration replacement.
        eval(scope, "message.style.animation='other .3s'");
        let restarted = sample(scope);
        assert!(!restarted.allows_underlying_geometry());
        assert_ne!(restarted.effects[0].identity, identity);
        eval(scope, "message.style.animation='unknown .3s'");
        let missing = sample(scope);
        assert_eq!(missing.animation_names, ["unknown"]);
        assert!(!missing.allows_underlying_geometry());
        eval(scope, "message.style.animation='none'");
        sample(scope);
        frame(scope, 16);
        assert!(sample(scope).allows_underlying_geometry());
    });
}

#[test]
fn strict_states_script_overrides_and_fill_keep_geometry_guarded() {
    with_dom(|scope| {
        for declaration in [
            "fadeInUp .3s infinite",
            "fadeInUp .3s paused",
            "fadeInUp .3s forwards",
            "fadeInUp .3s both",
            "fadeInUp .3s 10s",
            "fadeInUp .3s -0.1s",
        ] {
            eval(scope, &format!("message.style.animation={declaration:?}"));
            assert!(!sample(scope).allows_underlying_geometry(), "{declaration}");
            frame(scope, 16);
            if declaration != "fadeInUp .3s -0.1s" {
                frame(scope, 1000);
                assert!(!sample(scope).allows_underlying_geometry(), "{declaration}");
            }
            eval(scope, "message.style.animation='none'");
            sample(scope);
        }
        for mutation in [
            "a.pause()",
            "a.cancel()",
            "a.reverse()",
            "a.currentTime=300",
            "a.finish()",
            "a.effect.updateTiming({duration:1})",
            "a.effect.setKeyframes([{opacity:0},{opacity:1}])",
            "a.effect.target=document.getElementById('feedback')",
            "a.effect.composite='add'",
            "a.effect=null",
            r#"a.rangeStart="10%""#,
            r#"a.rangeEnd="90%""#,
        ] {
            eval(scope, "message.style.animation='fadeInUp .3s'");
            sample(scope);
            frame(scope, 16);
            eval(
                scope,
                &format!("{{const a=message.getAnimations()[0];{mutation}}}"),
            );
            frame(scope, 1000);
            assert!(!sample(scope).allows_underlying_geometry(), "{mutation}");
            eval(scope, "message.style.animation='none'");
            sample(scope);
        }
    });
}

#[test]
fn guest_web_animation_getters_cannot_forge_completion_or_intercept_host_read() {
    with_dom(|scope| {
        sample(scope);
        frame(scope, 16);
        eval(
            scope,
            r#"
          const fail=()=>{throw Error('page getter called')};
          Document.prototype.getAnimations=Element.prototype.getAnimations=fail;
          for(const [prototype,key] of [[CSSAnimation.prototype,'animationName'],
            [Animation.prototype,'effect'],[KeyframeEffect.prototype,'target']])
            Object.defineProperty(prototype,key,{get:fail,configurable:true});
          KeyframeEffect.prototype.getComputedTiming=fail;
          globalThis.__sheetReads=0;
          Object.defineProperty(HTMLStyleElement.prototype,'sheet',{configurable:true,get(){__sheetReads++;throw Error('guest sheet getter')}});
          Object.defineProperty(document.querySelector('style'),'sheet',{configurable:true,get(){__sheetReads++;throw Error('guest own sheet getter')}});
        "#,
        );
        assert!(!sample(scope).allows_underlying_geometry());
        frame(scope, 400);
        assert!(sample(scope).allows_underlying_geometry());
        assert!(eval(scope, "__sheetReads===0").is_true());
        assert!(eval(scope, "typeof globalThis.inspectElementCssEffects==='undefined' && typeof globalThis.__tsNs==='undefined'").is_true());
        let non_element = element(scope, "document");
        assert!(css_effect_lifecycle(scope, non_element).is_none());
        browser_env::set_css_animations(scope, false);
        assert!(!sample(scope).allows_underlying_geometry());
    });
}

fn snapshot_params() -> v8::CreateParams {
    match std::env::var_os("TURBO_DOM_SNAPSHOT_PATH").and(browser_env::snapshot_blob()) {
        Some(blob) => blob.create_params(),
        None => Default::default(),
    }
}

#[test]
fn shadow_and_foreign_realm_ownership_and_declaration_changes_fail_closed() {
    with_dom(|scope| {
        sample(scope);
        frame(scope, 16);
        frame(scope, 400);
        assert!(sample(scope).allows_underlying_geometry());
        // A CSSOM timing mutation on the same name must invalidate completion.
        eval(
            scope,
            "document.styleSheets[0].cssRules[2].style.animationDuration='10s'",
        );
        assert!(!sample(scope).allows_underlying_geometry());
        for mode in ["open", "closed"] {
            eval(
                scope,
                &format!(
                    r#"{{const host=document.createElement('div');document.body.append(host);
              const root=host.attachShadow({{mode:{mode:?}}});
              root.innerHTML='<style>@keyframes local {{from {{opacity:0}} to {{opacity:1}}}}</style><div style="animation:local .3s"></div>';
              globalThis.shadowTarget=root.lastChild;}}"#
                ),
            );
            let target = element(scope, "shadowTarget");
            let snapshot = css_effect_lifecycle(scope, target).unwrap();
            assert!(!snapshot.allows_underlying_geometry());
            assert_eq!(snapshot.element, handle_of(scope, target).unwrap());
        }
        let previous = scope.get_current_context();
        let context = v8::Context::new(scope, Default::default());
        let (target, document_id) = {
            let scope = &mut v8::ContextScope::new(scope, context);
            browser_env::install_independent_html(
                scope,
                "<div id='foreign' style='animation:missing .3s'></div>",
            );
            let target = element(scope, "document.getElementById('foreign')");
            let snapshot = css_effect_lifecycle(scope, target).unwrap();
            (target, snapshot.document_id)
        };
        assert_eq!(scope.get_current_context(), previous);
        let snapshot = css_effect_lifecycle(scope, target).unwrap();
        assert_eq!(snapshot.document_id, document_id);
        assert!(!snapshot.allows_underlying_geometry());
        assert_ne!(snapshot.document_id, sample(scope).document_id);
        assert_eq!(scope.get_current_context(), previous);
    });
}

#[test]
fn clock_reads_do_not_drive_frames_and_reentry_or_exceptions_are_unavailable() {
    with_dom(|scope| {
        browser_env::run_engine_script(scope,r#"
          globalThis.__nested='not-called';
          __tsNs.noteCssUpdatePass=()=>{globalThis.__nested=__tsNs.inspectElementCssEffects(message)};
        "#).unwrap();
        sample(scope);
        assert!(eval(scope, "__nested===null").is_true());
        frame(scope, 16);
        eval(scope, "__jump(400)");
        let elapsed = sample(scope);
        assert_eq!(elapsed.effects[0].play_state, "finished");
        assert!(!elapsed.allows_underlying_geometry());
        frame(scope, 0);
        assert!(sample(scope).allows_underlying_geometry());
        browser_env::run_engine_script(
            scope,
            r#"
          __tsNs.refreshCssEffectInputs=()=>{throw new Error('unavailable sync')};
        "#,
        )
        .unwrap();
        let target = element(scope, "message");
        assert!(css_effect_lifecycle(scope, target).is_none());
        assert!(eval(scope, "1+1===2").is_true());
    });
}

#[test]
fn producer_css_inputs_do_not_call_guest_iterators_or_numeric_setters() {
    with_dom(|scope| {
        eval(
            scope,
            r#"
          globalThis.__prototypeCalls=0;
          globalThis.__arrayIterator=Object.getOwnPropertyDescriptor(Array.prototype,Symbol.iterator);
          Object.defineProperty(Array.prototype,Symbol.iterator,{configurable:true,get(){__prototypeCalls++;throw Error('guest iterator')}});
        "#,
        );
        let target = element(scope, "message");
        assert!(css_effect_lifecycle(scope, target).is_none());
        eval(
            scope,
            r#"
          Object.defineProperty(Array.prototype,Symbol.iterator,__arrayIterator);
        "#,
        );
        assert!(eval(scope, "__prototypeCalls===0").is_true());
        // Indexed prototype accessors can affect unrelated cascade arrays. Reject
        // the shape before synchronization, without ever invoking the accessor.
        eval(
            scope,
            r#"
          Object.defineProperty(Array.prototype,'0',{configurable:true,get(){__prototypeCalls++;return undefined},set(){__prototypeCalls++}});
        "#,
        );
        let target = element(scope, "message");
        assert!(css_effect_lifecycle(scope, target).is_none());
        eval(scope, "delete Array.prototype[0]");
        assert!(eval(scope, "__prototypeCalls===0").is_true());
    });
}

#[test]
fn promise_assimilation_hooks_and_proxy_prototypes_are_unavailable_without_reentry() {
    for prototype in [
        "CSSAnimation.prototype",
        "Animation.prototype",
        "Object.prototype",
    ] {
        with_dom(|scope| {
            eval(
                scope,
                &format!(
                    r#"globalThis.__thenCalls=0;Object.defineProperty({prototype},'then',{{configurable:true,get(){{__thenCalls++;throw Error('guest then')}}}})"#
                ),
            );
            let target = element(scope, "message");
            assert!(css_effect_lifecycle(scope, target).is_none());
            assert!(eval(scope, "__thenCalls===0").is_true());
        });
    }
    with_dom(|scope| {
        sample(scope);
        eval(
            scope,
            r#"
          globalThis.__thenCalls=0;
          Object.defineProperty(message.getAnimations()[0],'then',{configurable:true,get(){__thenCalls++;throw Error('own guest then')}});
          message.style.animation='other .3s';
        "#,
        );
        let target = element(scope, "message");
        assert!(css_effect_lifecycle(scope, target).is_none());
        assert!(eval(scope, "__thenCalls===0").is_true());
    });
    with_dom(|scope| {
        eval(
            scope,
            r#"
          globalThis.__proxyCalls=0;
          const proxy=new Proxy({}, {getPrototypeOf(){__proxyCalls++;throw Error('guest proxy')},getOwnPropertyDescriptor(){__proxyCalls++;throw Error('guest proxy')}});
          Object.setPrototypeOf(CSSAnimation.prototype,proxy);
        "#,
        );
        let target = element(scope, "message");
        assert!(css_effect_lifecycle(scope, target).is_none());
        assert!(eval(scope, "__proxyCalls===0").is_true());
    });
}

#[test]
fn shared_object_prototype_hooks_fail_closed_without_executing_guest_code() {
    for name in [
        "css",
        "hostTrap",
        "inspectionModified",
        "scriptPlayState",
        "easingFn",
        "computed",
        "insertion",
        "toJSON",
    ] {
        with_dom(|scope| {
            eval(
                scope,
                &format!(
                    r#"globalThis.__hookCalls=0;Object.defineProperty(Object.prototype,{name:?},{{configurable:true,enumerable:true,get(){{__hookCalls++;throw Error('guest shared getter')}},set(){{__hookCalls++;throw Error('guest shared setter')}}}})"#
                ),
            );
            let target = element(scope, "message");
            assert!(css_effect_lifecycle(scope, target).is_none());
            assert!(eval(scope, "__hookCalls===0").is_true());
        });
    }
}

#[test]
fn replaced_standard_globals_are_unavailable_without_calling_guest_constructors() {
    for name in ["String", "Map", "Proxy", "Object", "parseFloat"] {
        with_dom(|scope| {
            eval(
                scope,
                &format!(
                    r#"globalThis.__globalCalls=0;Object.defineProperty(globalThis,{name:?},{{configurable:true,get(){{__globalCalls++;throw Error('guest builtin')}}}})"#
                ),
            );
            let target = element(scope, "message");
            assert!(css_effect_lifecycle(scope, target).is_none());
            assert!(eval(scope, "__globalCalls===0").is_true());
        });
    }
}

#[test]
fn automatic_microtask_policy_is_unavailable_and_does_not_flush_guest_jobs() {
    with_dom(|scope| {
        let target = element(scope, "message");
        eval(
            scope,
            "globalThis.__jobRan=false;Promise.resolve().then(()=>{__jobRan=true})",
        );
        scope.set_microtasks_policy(v8::MicrotasksPolicy::Auto);
        assert!(css_effect_lifecycle(scope, target).is_none());
        scope.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
        assert!(eval(scope, "__jobRan===false").is_true());
        scope.perform_microtask_checkpoint();
        assert!(eval(scope, "__jobRan===true").is_true());
    });
}
