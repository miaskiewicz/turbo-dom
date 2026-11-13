#![cfg(feature = "host-internals")]
use std::sync::Once;
use turbo_dom_browser_env::{browser_env, host_internals::*};

const HTML: &str = "<!doctype html><div id=message></div>";

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

fn snapshot_params() -> v8::CreateParams {
    match std::env::var_os("TURBO_DOM_SNAPSHOT_PATH").and(browser_env::snapshot_blob()) {
        Some(blob) => blob.create_params(),
        None => Default::default(),
    }
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
fn scoped_shadow_sheet_effects_are_active_then_naturally_complete() {
    for mode in ["open", "closed"] {
        with_dom(|scope| {
            eval(
                scope,
                &format!(
                    r#"
              const host=document.createElement('div');document.body.appendChild(host);
              const root=host.attachShadow({{mode:{mode:?}}});
              root.innerHTML='<style>@keyframes local {{from {{opacity:0;transform:translateY(8px)}} to {{opacity:1;transform:translateY(0)}}}} div {{animation:local .3s}} button {{width:32px;height:32px}}</style><div><button>Vote</button></div>';
              globalThis.message=root.lastChild;
              globalThis.finishedCount=0;
            "#
                ),
            );
            assert!(eval(scope, "(()=>{const count=message.getAnimations().length;__layoutCalls=0;return count===1})()").is_true());
            let initial = sample(scope);
            assert_eq!(initial.animation_names, ["local"], "{mode}: {initial:?}");
            assert_eq!(initial.effects.len(), 1);
            assert!(!initial.allows_underlying_geometry());
            let identity = initial.effects[0].identity;
            assert!(eval(scope, "__layoutCalls===0").is_true());
            eval(
                scope,
                "message.getAnimations()[0].finished.then(()=>finishedCount++);__layoutCalls=0",
            );
            frame(scope, 16);
            let active = sample(scope);
            assert_eq!(active.effects[0].phase, CssEffectPhase::Active);
            assert!(active.effects[0].contributes);
            frame(scope, 400);
            let done = sample(scope);
            assert_eq!(done.effects[0].identity, identity);
            assert!(done.allows_underlying_geometry(), "{mode}: {done:?}");
            assert!(eval(scope, "finishedCount===1 && __layoutCalls===0").is_true());
            assert!(eval(scope, "message.getAnimations().length===0").is_true());
        });
    }
}

#[test]
fn scoped_important_names_cannot_be_waived_by_completed_inline_animation() {
    with_dom(|scope| {
        eval(
            scope,
            r#"
          const host=document.createElement('div');document.body.appendChild(host);
          const root=host.attachShadow({mode:'closed'});
          root.innerHTML='<style>@keyframes inlineName {from {opacity:0} to {opacity:1}} @keyframes scopedName {from {opacity:0} to {opacity:1}} div {animation:scopedName 2s !important}</style><div style="animation:inlineName .1s"></div>';
          globalThis.message=root.lastChild;
        "#,
        );
        assert_eq!(sample(scope).animation_names, ["scopedName"]);
        frame(scope, 16);
        frame(scope, 400);
        let active = sample(scope);
        assert_eq!(active.animation_names, ["scopedName"]);
        assert_eq!(active.effects[0].name, "scopedName");
        assert_eq!(active.effects[0].phase, CssEffectPhase::Active);
        assert!(!active.allows_underlying_geometry());
        frame(scope, 2000);
        assert!(sample(scope).allows_underlying_geometry());
    });
}

#[test]
fn sibling_shadow_keyframes_and_cssom_adoption_are_isolated_and_live() {
    with_dom(|scope| {
        eval(
            scope,
            r#"
          globalThis.roots=[];globalThis.targets=[];
          for (const duration of [.1,2]) {
            const host=document.createElement('div');document.body.appendChild(host);
            const root=host.attachShadow({mode:'closed'});
            const sheet=new CSSStyleSheet();
            sheet.replaceSync('@keyframes shared {from {opacity:'+(duration===.1?'0':'.5')+'} to {opacity:1}} div {animation:shared '+duration+'s}');
            root.adoptedStyleSheets=[sheet];
            root.appendChild(document.createElement('div'));
            roots.push(root);targets.push(root.lastChild);
          }
          globalThis.message=targets[0];
        "#,
        );
        let first = sample(scope);
        eval(scope, "globalThis.message=targets[1]");
        let second = sample(scope);
        assert_eq!(first.animation_names, ["shared"]);
        assert_eq!(second.animation_names, ["shared"]);
        assert_ne!(first.effects[0].identity, second.effects[0].identity);
        assert!(eval(scope,"targets[0].getAnimations()[0].effect.getKeyframes()[0].opacity==='0' && targets[1].getAnimations()[0].effect.getKeyframes()[0].opacity==='0.5'").is_true());
        frame(scope, 16);
        frame(scope, 400);
        assert!(!sample(scope).allows_underlying_geometry());
        eval(scope, "globalThis.message=targets[0]");
        assert!(sample(scope).allows_underlying_geometry());
        eval(
            scope,
            r#"
          const replacement=new CSSStyleSheet();
          replacement.replaceSync('@keyframes changed {from {opacity:0} to {opacity:1}} div {animation:changed 1s}');
          roots[0].adoptedStyleSheets=[replacement];
        "#,
        );
        let changed = sample(scope);
        assert_eq!(changed.animation_names, ["changed"]);
        assert_ne!(changed.effects[0].identity, first.effects[0].identity);
        assert!(!changed.allows_underlying_geometry());
        eval(scope, "roots[0].adoptedStyleSheets[0].deleteRule(1)");
        let removed = sample(scope);
        assert!(removed.animation_names.is_empty());
        frame(scope, 16);
        assert!(sample(scope).allows_underlying_geometry());
    });
}

#[test]
fn scoped_lifecycle_never_enters_guest_sheet_root_or_animation_getters() {
    with_dom(|scope| {
        eval(
            scope,
            r#"
          const host=document.createElement('div');document.body.appendChild(host);
          const root=host.attachShadow({mode:'closed'});
          root.innerHTML='<style>@keyframes local {from {opacity:0} to {opacity:1}} div {animation:local .3s}</style><div></div>';
          globalThis.message=root.lastChild;globalThis.guestReads=0;
          const poison=()=>{guestReads++;throw new Error('guest getter')};
          Object.defineProperty(message,'getRootNode',{get:poison});
          Object.defineProperty(message,'matches',{get:poison});
          Object.defineProperty(root,'styleSheets',{get:poison});
          Object.defineProperty(root,'adoptedStyleSheets',{get:poison});
          Object.defineProperty(root.firstChild,'sheet',{get:poison});
          Object.defineProperty(message,'getAnimations',{get:poison});
        "#,
        );
        let pending = sample(scope);
        assert_eq!(pending.animation_names, ["local"]);
        assert!(!pending.allows_underlying_geometry());
        frame(scope, 16);
        frame(scope, 400);
        assert!(sample(scope).allows_underlying_geometry());
        assert!(eval(scope, "guestReads===0 && __layoutCalls===0").is_true());
    });
}

#[test]
fn moved_shadow_targets_rebind_to_their_actual_root_and_owner_context() {
    with_dom(|scope| {
        eval(
            scope,
            r#"
          globalThis.roots=[];
          for (const name of ['first','second']) {
            const host=document.createElement('div');document.body.appendChild(host);
            const root=host.attachShadow({mode:'closed'});
            root.innerHTML='<style>@keyframes '+name+' {from {opacity:0} to {opacity:1}} div {animation:'+name+' .3s}</style>';
            roots.push(root);
          }
          globalThis.message=document.createElement('div');roots[0].appendChild(message);
        "#,
        );
        let first = sample(scope);
        assert_eq!(first.animation_names, ["first"]);
        eval(scope, "roots[1].appendChild(message)");
        let moved = sample(scope);
        assert_eq!(moved.animation_names, ["second"]);
        assert_ne!(first.effects[0].identity, moved.effects[0].identity);
        assert!(!moved.allows_underlying_geometry());
        let previous = scope.get_current_context();
        let context = v8::Context::new(scope, Default::default());
        let (target, document_id) = {
            let scope = &mut v8::ContextScope::new(scope, context);
            browser_env::install_independent_html(scope, "<!doctype html><div id=host></div>");
            eval(
                scope,
                r#"
              const root=document.getElementById('host').attachShadow({mode:'closed'});
              root.innerHTML='<style>@keyframes foreign {from {opacity:0} to {opacity:1}} div {animation:foreign .3s}</style><div></div>';
              globalThis.foreignTarget=root.lastChild;
            "#,
            );
            let target = element(scope, "foreignTarget");
            let observed = css_effect_lifecycle(scope, target).unwrap();
            assert_eq!(observed.animation_names, ["foreign"]);
            assert_eq!(observed.effects.len(), 1);
            (target, observed.document_id)
        };
        let observed = css_effect_lifecycle(scope, target).unwrap();
        assert_eq!(observed.document_id, document_id);
        assert_ne!(observed.document_id, moved.document_id);
        assert_eq!(observed.animation_names, ["foreign"]);
        assert_eq!(scope.get_current_context(), previous);
        assert_eq!(sample(scope).animation_names, ["second"]);
    });
}

#[test]
fn missing_local_keyframes_never_borrow_another_shadow_scope_or_prove_completion() {
    with_dom(|scope| {
        eval(
            scope,
            r#"
          const host=document.createElement('div');document.body.appendChild(host);
          const root=host.attachShadow({mode:'closed'});
          root.innerHTML='<style>@keyframes outerName {from {opacity:0} to {opacity:1}}</style><section></section>';
          const inner=root.lastChild.attachShadow({mode:'closed'});
          inner.innerHTML='<style>div {animation:outerName .01s}</style><div></div>';
          globalThis.message=inner.lastChild;
        "#,
        );
        let missing = sample(scope);
        assert_eq!(missing.animation_names, ["outerName"]);
        assert!(missing.effects.is_empty());
        assert!(!missing.allows_underlying_geometry());
        frame(scope, 1000);
        assert!(!sample(scope).allows_underlying_geometry());
    });
}
