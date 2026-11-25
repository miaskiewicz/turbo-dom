#![cfg(feature = "host-internals")]
use std::sync::Once;
use turbo_dom_browser_env::browser_env;

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
    browser_env::install_independent_html(
        scope,
        "<!doctype html><div style='font-size:24px'><div id='target'></div></div>",
    );
    browser_env::run_engine_script(scope, "__tsNs.installAnimationHost({now:()=>1000,frame(){},task(){}});globalThis.__calls=0;__tsNs.layoutUsedSize=()=>{__calls++;return[200,100,200,100,24]}").unwrap();
    eval(scope, "globalThis.target=document.getElementById('target')");
    test(scope);
    browser_env::reset();
}

#[test]
fn absolute_transform_style_reads_do_not_enter_the_used_size_provider() {
    for (transform, expected) in [
        ("matrix(1,0,0,1,14,16)", "matrix(1, 0, 0, 1, 14, 16)"),
        (
            "rotate(45deg)",
            "matrix(0.707107, 0.707107, -0.707107, 0.707107, 0, 0)",
        ),
        (
            "translate(14px,16px) scale(2)",
            "matrix(2, 0, 0, 2, 14, 16)",
        ),
    ] {
        with_dom(|scope| {
            eval(
                scope,
                &format!("target.style.transform='{transform}';getComputedStyle(target).opacity"),
            );
            assert_eq!(
                eval(scope, "__calls").integer_value(scope),
                Some(0),
                "{transform}"
            );
            assert_eq!(
                eval(scope, "getComputedStyle(target).transform").to_rust_string_lossy(scope),
                expected
            );
            assert_eq!(eval(scope, "__calls").integer_value(scope), Some(0));
        });
    }
}

#[test]
fn animation_discovery_on_absolute_transforms_does_not_need_layout() {
    with_dom(|scope| {
        browser_env::run_engine_script(scope, "__tsNs.layoutUsedSize=()=>{__calls++;throw Error('absolute transform requested layout')}").unwrap();
        eval(scope,"document.head.innerHTML='<style>@keyframes probe{from{opacity:.5}to{opacity:1}}</style>';target.style.cssText='transform:matrix(1,0,0,1,14,16);animation:probe 1000ms linear both'");
        assert_eq!(
            eval(scope, "document.getAnimations().length").integer_value(scope),
            Some(1)
        );
        assert_eq!(eval(scope, "__calls").integer_value(scope), Some(0));
    });
}

#[test]
fn relative_transform_reads_keep_the_real_border_box_and_inherited_font_basis() {
    for (transform, expected) in [
        ("translate(50%,25%)", "matrix(1, 0, 0, 1, 100, 25)"),
        ("translate(2em,1em)", "matrix(1, 0, 0, 1, 48, 24)"),
        ("translate(2rem,1rem)", "matrix(1, 0, 0, 1, 32, 16)"),
    ] {
        with_dom(|scope| {
            eval(scope, &format!("target.style.transform='{transform}'"));
            assert_eq!(
                eval(scope, "getComputedStyle(target).transform").to_rust_string_lossy(scope),
                expected
            );
            assert!(eval(scope, "__calls").integer_value(scope).unwrap() > 0);
        });
    }
}

#[test]
fn unresolved_and_nonfinite_lists_do_not_acquire_a_zero_reference_basis() {
    with_dom(|scope| {
        browser_env::run_engine_script(scope, "__tsNs.layoutUsedSize=()=>{__calls++;return null};globalThis.__unknown=__tsNs.resolveTransform('translate(50%,25%)',target,undefined);globalThis.__overflow=__tsNs.resolveTransform('translate(1e309px)',target,undefined);globalThis.__matrixOverflow=__tsNs.resolveTransform('matrix(1e309,0,0,1,0,0)',target,undefined)").unwrap();
        assert!(eval(
            scope,
            "__unknown===null&&__overflow===null&&__matrixOverflow===null"
        )
        .is_true());
        assert_eq!(eval(scope, "__calls").integer_value(scope), Some(3));
    });
}

#[test]
fn local_absolute_matrix_is_not_composed_with_its_transformed_ancestor() {
    with_dom(|scope| {
        eval(scope,"target.parentElement.style.transform='scale(3)';target.style.transform='translate(14px,16px)'");
        assert_eq!(
            eval(scope, "getComputedStyle(target).transform").to_rust_string_lossy(scope),
            "matrix(1, 0, 0, 1, 14, 16)"
        );
        assert_eq!(eval(scope, "__calls").integer_value(scope), Some(0));
    });
}

#[test]
fn reference_transform_animation_discovery_does_not_need_used_geometry() {
    let mut rows = Vec::new();
    for operation in ["document.getAnimations()", "target.getAnimations()"] {
        for throws in [false, true] {
            with_dom(|scope| {
                let provider = if throws {
                    "__calls++;throw Error('discovery provider sentinel')"
                } else {
                    "__calls++;return[100,20,100,20,24]"
                };
                browser_env::run_engine_script(
                    scope,
                    &format!("__tsNs.layoutUsedSize=()=>{{{provider}}}"),
                )
                .unwrap();
                eval(scope,"document.head.innerHTML='<style>@keyframes probe{from{opacity:.5}to{opacity:1}}</style>';target.style.cssText='width:100px;height:20px;transform:translateX(50%);animation:probe 1000ms linear both paused';__calls=0");
                eval(scope, &format!("globalThis.discovered={operation}"));
                let calls = eval(scope, "__calls").integer_value(scope).unwrap();
                assert!(eval(scope,"discovered.length===1&&discovered[0] instanceof CSSAnimation&&discovered[0].animationName==='probe'&&discovered[0].playState==='paused'&&discovered[0].currentTime===0").is_true());
                assert!(eval(scope,"discovered[0].effect.getKeyframes()[0].opacity==='0.5'&&discovered[0].effect.getKeyframes()[1].opacity==='1'").is_true());
                let total_calls = eval(scope, "__calls").integer_value(scope).unwrap();
                eprintln!("reference-discovery operation={operation} provider_throws={throws} discovery_provider_calls={calls} model_inspection_calls={}",total_calls-calls);
                rows.push((operation, throws, calls));
            });
        }
    }
    assert!(
        rows.iter().all(|(_, _, calls)| *calls == 0),
        "lifecycle discovery must not resolve used transform geometry: {rows:?}"
    );
}

#[test]
fn explicit_reference_transform_reads_still_require_the_current_native_box() {
    with_dom(|scope| {
        browser_env::run_engine_script(scope,"globalThis.__referenceWidth=100;__tsNs.layoutUsedSize=()=>{__calls++;return[__referenceWidth,20,__referenceWidth,20,24]}").unwrap();
        eval(
            scope,
            "target.style.cssText='width:100px;height:20px;transform:translateX(50%)';__calls=0",
        );
        assert_eq!(
            eval(scope, "getComputedStyle(target).transform").to_rust_string_lossy(scope),
            "matrix(1, 0, 0, 1, 50, 0)"
        );
        assert!(eval(scope, "__calls>0").is_true());
        eval(
            scope,
            "__referenceWidth=200;target.style.width='200px';__calls=0",
        );
        assert_eq!(
            eval(scope, "getComputedStyle(target).transform").to_rust_string_lossy(scope),
            "matrix(1, 0, 0, 1, 100, 0)"
        );
        assert!(eval(scope, "__calls>0").is_true());
    });
}

#[test]
fn discovery_provider_attempts_are_specific_to_reference_transform_resolution() {
    let mut rows = Vec::new();
    for transform in ["none", "matrix(1,0,0,1,50,0)", "translateX(50%)"] {
        with_dom(|scope| {
            browser_env::run_engine_script(
                scope,
                "__tsNs.layoutUsedSize=()=>{__calls++;return[100,20,100,20,24]}",
            )
            .unwrap();
            eval(scope,"document.head.innerHTML='<style>@keyframes probe{from{opacity:.5}to{opacity:1}}</style>'");
            eval(scope,&format!("target.style.cssText='width:100px;height:20px;transform:{transform};animation:probe 1000ms linear both paused';__calls=0;globalThis.motionCount=document.getAnimations().length"));
            let calls = eval(scope, "__calls").integer_value(scope).unwrap();
            assert!(eval(scope, "motionCount===1").is_true());
            eprintln!("discovery-transform-inverse transform={transform} provider_calls={calls}");
            rows.push(calls);
        });
    }
    assert_eq!(&rows[..2], [0, 0]);
    assert!(rows[2] > 0);
}
