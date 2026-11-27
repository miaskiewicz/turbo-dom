#![cfg(feature = "host-internals")]
use sha2::{Digest, Sha256};
use std::sync::Once;
use turbo_dom_browser_env::{browser_env, host_internals as host};

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
    let owner = host::enter_context_document(context).unwrap();
    test(scope);
    drop(owner);
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
            eprintln!(
                "renderedness case={id} phase={} targets={targets}",
                step["id"]
            );
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

#[test]
fn manual_assignment_changes_only_the_existing_live_revision_and_deduplicated_noop_does_not() {
    with_dom(|scope| {
        eval(scope,"globalThis.host=document.createElement('div');document.body.appendChild(host);globalThis.root=host.attachShadow({mode:'closed',slotAssignment:'manual'});root.innerHTML='<slot id=a></slot><slot id=b></slot>';globalThis.a=root.querySelector('#a');globalThis.b=root.querySelector('#b');globalThis.first=document.createElement('i');globalThis.second=document.createElement('i');host.append(first,second)");
        let tree = host::with_tree(|tree| tree.version).unwrap();
        let before = host::style_revisions();
        eval(scope, "a.assign(first,second,first)");
        let changed = host::style_revisions();
        assert_eq!(host::with_tree(|tree| tree.version), Some(tree));
        assert!(changed.live_state > before.live_state);
        assert_eq!(changed.cssom, before.cssom);
        eval(scope, "a.assign(first,second)");
        assert_eq!(host::style_revisions().live_state, changed.live_state);
        eval(scope, "a.assign(first,second,first)");
        assert_eq!(host::style_revisions().live_state, changed.live_state);
        eval(scope, "a.assign(second,first)");
        let reordered = host::style_revisions();
        assert!(reordered.live_state > changed.live_state);
        eval(scope, "b.assign(first)");
        assert!(host::style_revisions().live_state > reordered.live_state);
        assert!(eval(scope,"a.assignedNodes().length===1&&a.assignedNodes()[0]===second&&b.assignedNodes()[0]===first").is_true());
        assert_eq!(host::with_tree(|tree| tree.version), Some(tree));
    });
}

#[test]
fn stale_private_assignment_during_display_read_is_unproved_without_canceling_css() {
    with_dom(|scope| {
        eval(scope,"globalThis.host=document.createElement('div');document.body.appendChild(host);globalThis.root=host.attachShadow({mode:'closed',slotAssignment:'manual'});root.innerHTML='<slot id=a></slot><slot id=b style=display:none></slot>';globalThis.a=root.querySelector('#a');globalThis.b=root.querySelector('#b');globalThis.target=document.createElement('div');target.className='animated';host.appendChild(target);a.assign(target);globalThis.oldCss=target.getAnimations().find(effect=>effect instanceof CSSAnimation);globalThis.canceled=0;oldCss.finished.catch(()=>canceled++);target.setAttribute('data-probe','fresh')");
        browser_env::run_engine_script(scope,"__tsNs.__displayOriginal=__tsNs.computedDisplay;globalThis.armed=true;__tsNs.computedDisplay=function(element,snapshots){const value=__tsNs.__displayOriginal(element,snapshots);if(armed&&element===a){armed=false;globalThis.treeBefore=__tsNs.computedStyleStamp(target)[0];b.assign(target);globalThis.treeAfter=__tsNs.computedStyleStamp(target)[0]}return value}").unwrap();
        let document = v8::Local::<v8::Object>::try_from(eval(scope, "document")).unwrap();
        let target = v8::Local::<v8::Object>::try_from(eval(scope, "target")).unwrap();
        let handle = host::handle_of(scope, target).unwrap();
        assert!(host::sample_presented_effects(scope, document, &[handle]).is_none());
        scope.perform_microtask_checkpoint();
        assert!(eval(scope, "!armed&&treeBefore===treeAfter&&canceled===0").is_true());
        browser_env::run_engine_script(
            scope,
            "__tsNs.computedDisplay=__tsNs.__displayOriginal;delete __tsNs.__displayOriginal",
        )
        .unwrap();
        assert_eq!(
            eval(
                scope,
                "target.getAnimations().filter(effect=>effect instanceof CSSAnimation).length"
            )
            .integer_value(scope),
            Some(0)
        );
        scope.perform_microtask_checkpoint();
        assert_eq!(eval(scope, "canceled").integer_value(scope), Some(1));
    });
}

#[test]
fn presented_display_important() {
    control("presented-display-important");
}
#[test]
fn display_exit_transition() {
    control("display-exit-transition");
}
#[test]
fn foreign_manual_assignment() {
    control("foreign-manual-assignment");
}

#[test]
fn rendering_display_callback_preserves_original_errors_and_adoption_refusal() {
    for adoption in [false, true] {
        with_dom(|scope| {
            eval(scope,"globalThis.host=document.createElement('div');document.body.appendChild(host);globalThis.root=host.attachShadow({mode:'closed'});root.innerHTML='<style>@keyframes probe{from{opacity:.5}to{opacity:1}}.animated{animation:probe 1s linear both paused}</style><div id=target class=animated></div>';globalThis.target=root.querySelector('#target');target.getAnimations();target.setAttribute('data-probe','fresh');globalThis.sentinel=new Error('original rendering callback');globalThis.other=document.implementation.createHTMLDocument('other')");
            let action = if adoption {
                "other.body.appendChild(target)"
            } else {
                "throw sentinel"
            };
            browser_env::run_engine_script(scope,&format!("__tsNs.__displayOriginal=__tsNs.computedDisplay;globalThis.armed=true;__tsNs.computedDisplay=function(element,snapshots){{const value=__tsNs.__displayOriginal(element,snapshots);if(armed&&element===host){{armed=false;{action}}}return value}} ")).unwrap();
            let document = v8::Local::<v8::Object>::try_from(eval(scope, "document")).unwrap();
            let target = v8::Local::<v8::Object>::try_from(eval(scope, "target")).unwrap();
            let handle = host::handle_of(scope, target).unwrap();
            let sentinel = eval(scope, "sentinel");
            {
                v8::tc_scope!(let catch,scope);
                assert!(host::sample_presented_effects(catch, document, &[handle]).is_none());
                if adoption {
                    assert!(!catch.has_caught());
                } else {
                    assert!(catch.has_caught());
                    assert!(catch.exception().unwrap().strict_equals(sentinel));
                    catch.reset();
                }
            }
            assert!(eval(scope, "!armed").is_true());
            browser_env::run_engine_script(
                scope,
                "__tsNs.computedDisplay=__tsNs.__displayOriginal;delete __tsNs.__displayOriginal",
            )
            .unwrap();
            if adoption {
                assert!(eval(
                    scope,
                    "target.ownerDocument===other&&target.parentNode===other.body"
                )
                .is_true());
            } else {
                assert_eq!(
                    eval(scope, "target.getAnimations().length").integer_value(scope),
                    Some(1)
                );
            }
        });
    }
}
