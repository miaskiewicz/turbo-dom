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
    browser_env::run_engine_script(scope, "globalThis.__clockReads=0;globalThis.__steppingClock=false;__tsNs.installAnimationHost({now:()=>{__clockReads++;return 1000+(__steppingClock?__clockReads:0)},frame(){},task(){}});globalThis.__calls=0;__tsNs.layoutUsedSize=()=>{__calls++;return[200,100,200,100,24]}").unwrap();
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
fn discovery_skips_used_resolution_for_each_transform_kind() {
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
    assert_eq!(rows, [0, 0, 0]);
}

#[test]
fn lifecycle_composes_inheritance_without_contaminating_underlying_or_public_snapshots() {
    with_dom(|scope| {
        eval(scope,"document.head.innerHTML='<style>@keyframes probe{from{opacity:.5}to{opacity:1}}</style>';globalThis.ancestor=target.parentElement;ancestor.style.cssText='font-size:10px;font-weight:200;--token:red';target.style.cssText='font-size:inherit;font-weight:inherit;cursor:inherit;transform:translateX(50%);animation:probe 1000ms linear both paused';globalThis.fontMotion=ancestor.animate([{fontSize:'10px',fontWeight:'200','--token':'red'},{fontSize:'30px',fontWeight:'800','--token':'blue'}],{duration:1000,fill:'both'});fontMotion.pause();fontMotion.currentTime=750;globalThis.cursorMotion=document.body.animate([{cursor:'pointer'},{cursor:'wait'}],{duration:1000,fill:'both'});cursorMotion.pause();cursorMotion.currentTime=750;");
        browser_env::run_engine_script(scope,"const fields=['font-size','font-weight','--token','cursor'];const read=mode=>{const snapshot=__tsNs.refreshCssEffectInputs(target,__tsNs.createCssEffectInputBatch(mode));return fields.map(name=>__tsNs.computedStyleValue(snapshot,name))};globalThis.underlyingBefore=JSON.stringify(read(false));globalThis.composed=JSON.stringify(read(true));__tsNs.__testLifecycleRead=read;").unwrap();
        assert_eq!(
            eval(scope, "underlyingBefore").to_rust_string_lossy(scope),
            r#"["10px","200","red","auto"]"#
        );
        assert_eq!(
            eval(scope, "composed").to_rust_string_lossy(scope),
            r#"["25px","650","blue","wait"]"#
        );
        eval(
            scope,
            "__calls=0;globalThis.discovered=document.getAnimations().length",
        );
        assert!(eval(scope, "__calls===0&&discovered===3").is_true());
        browser_env::run_engine_script(
            scope,
            "globalThis.underlyingAfter=JSON.stringify(__tsNs.__testLifecycleRead(false));delete __tsNs.__testLifecycleRead",
        )
        .unwrap();
        assert!(eval(scope, "underlyingBefore===underlyingAfter").is_true());
        assert!(eval(scope,"getComputedStyle(target).fontSize==='25px'&&getComputedStyle(target).fontWeight==='650'&&getComputedStyle(target).getPropertyValue('--token')==='blue'&&getComputedStyle(target).cursor==='wait'").is_true());
        assert_eq!(
            eval(scope, "getComputedStyle(target).transform").to_rust_string_lossy(scope),
            "matrix(1, 0, 0, 1, 100, 0)"
        );
    });
}

#[test]
fn discovery_keeps_auto_and_explicit_microtask_policies_and_original_errors() {
    for policy in [v8::MicrotasksPolicy::Auto, v8::MicrotasksPolicy::Explicit] {
        with_dom(|scope| {
            scope.set_microtasks_policy(policy);
            eval(scope,"globalThis.ran=0;globalThis.flushedInline=false;Promise.resolve().then(()=>ran++);target.style.transform='translateX(50%)';document.getAnimations();flushedInline=ran===0");
            assert!(eval(scope, "flushedInline").is_true());
            scope.perform_microtask_checkpoint();
            assert_eq!(eval(scope, "ran").integer_value(scope), Some(1));
            browser_env::run_engine_script(scope,"__tsNs.__savedLifecycleRefresh=__tsNs.refreshCssLifecycleInputs;globalThis.originalDiscoveryError=new Error('original lifecycle callback');__tsNs.refreshCssLifecycleInputs=()=>{throw originalDiscoveryError}").unwrap();
            assert!(eval(scope,"(()=>{try{document.getAnimations();return false}catch(error){return error===originalDiscoveryError}})()").is_true());
            browser_env::run_engine_script(scope, "__tsNs.refreshCssLifecycleInputs=__tsNs.__savedLifecycleRefresh;delete __tsNs.__savedLifecycleRefresh")
                .unwrap();
            assert_eq!(
                eval(scope, "document.getAnimations().length").integer_value(scope),
                Some(0)
            );
            scope.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
        });
    }
}

#[test]
fn lifecycle_discovery_preserves_shadow_own_display_and_quoted_names() {
    with_dom(|scope| {
        eval(scope,"globalThis.host=document.createElement('div');document.body.appendChild(host);globalThis.shadow=host.attachShadow({mode:'closed'});shadow.innerHTML='<style>@keyframes \"wide card\"{from{opacity:.5}to{opacity:1}}#inside{transform:translateX(50%);animation:\"wide card\" 1000ms linear both paused}</style><div id=inside></div>';globalThis.inside=shadow.querySelector('#inside');__calls=0");
        assert!(eval(scope,"shadow.getAnimations().length===1&&inside.getAnimations()[0].animationName==='wide card'&&document.getAnimations().length===0&&__calls===0").is_true());
        eval(scope, "inside.style.display='none'");
        assert_eq!(
            eval(scope, "shadow.getAnimations().length").integer_value(scope),
            Some(0)
        );
    });
}

#[test]
fn lifecycle_discovery_keeps_child_media_and_document_receiver_ownership() {
    with_dom(|scope| {
        browser_env::run_engine_script(scope,"__tsNs.layoutUsedSize=element=>{__calls++;const width=element.tagName==='IFRAME'?+element.getAttribute('width'):100;return[width,200,width,200,24]}").unwrap();
        eval(scope,"globalThis.frame=document.createElement('iframe');frame.width='300';frame.height='200';document.body.appendChild(frame);globalThis.child=frame.contentDocument;child.head.innerHTML='<style>@keyframes narrow{from{opacity:.5}to{opacity:1}}@keyframes wide{from{opacity:.1}to{opacity:1}}#s{transform:translateX(50%)}@media(max-width:400px){#s{animation:narrow 1s linear both paused}}@media(min-width:401px){#s{animation:wide 1s linear both paused}}</style>';child.body.innerHTML='<div id=s></div>'");
        assert_eq!(
            eval(scope, "frame.contentWindow.innerWidth").integer_value(scope),
            Some(300)
        );
        eval(scope, "globalThis.childElement=child.getElementById('s')");
        browser_env::run_engine_script(scope,"globalThis.childLifecycleName=__tsNs.computedStyleValue(__tsNs.refreshCssLifecycleInputs(childElement,__tsNs.createCssEffectInputBatch(true)),'animation-name')").unwrap();
        assert_eq!(
            eval(scope, "childLifecycleName").to_rust_string_lossy(scope),
            "narrow"
        );
        assert_eq!(
            eval(
                scope,
                "frame.contentWindow.getComputedStyle(childElement).animationName"
            )
            .to_rust_string_lossy(scope),
            "narrow"
        );
        assert!(eval(scope,"(()=>{try{Document.prototype.getAnimations.call(child);return false}catch(error){return error instanceof TypeError}})()").is_true());
        assert_eq!(
            eval(scope, "childElement.getAnimations().length").integer_value(scope),
            Some(0)
        );
        eval(scope, "frame.width='600'");
        browser_env::run_engine_script(scope,"globalThis.childLifecycleName=__tsNs.computedStyleValue(__tsNs.refreshCssLifecycleInputs(childElement,__tsNs.createCssEffectInputBatch(true)),'animation-name')").unwrap();
        assert_eq!(
            eval(scope, "childLifecycleName").to_rust_string_lossy(scope),
            "wide"
        );
        assert_eq!(
            eval(
                scope,
                "frame.contentWindow.getComputedStyle(childElement).animationName"
            )
            .to_rust_string_lossy(scope),
            "wide"
        );
    });
}

#[test]
fn lifecycle_discovery_holds_one_clock_and_refuses_nested_flush_without_poisoning_it() {
    with_dom(|scope| {
        browser_env::run_engine_script(scope,"globalThis.clockWitness=[];globalThis.nestedRefusals=0;__clockReads=0;__steppingClock=true;__tsNs.__clockOriginal=__tsNs.refreshCssLifecycleInputs;__tsNs.refreshCssLifecycleInputs=function(element,batch){clockWitness.push(document.timeline.currentTime);try{document.getAnimations()}catch(error){if(error instanceof TypeError)nestedRefusals++;else throw error}return __tsNs.__clockOriginal(element,batch)}").unwrap();
        eval(scope,"document.head.innerHTML='<style>@keyframes probe{from{opacity:0}to{opacity:1}}</style>';target.style.cssText='transform:translateX(50%);animation:probe 1s linear both paused';__clockReads=0;document.getAnimations()");
        assert!(eval(scope,"__clockReads===1&&clockWitness.length>0&&nestedRefusals===clockWitness.length&&clockWitness.every(value=>value===1001)").is_true());
        browser_env::run_engine_script(
            scope,
            "__tsNs.refreshCssLifecycleInputs=__tsNs.__clockOriginal;delete __tsNs.__clockOriginal",
        )
        .unwrap();
        assert_eq!(
            eval(scope, "document.getAnimations().length").integer_value(scope),
            Some(1)
        );
    });
}
