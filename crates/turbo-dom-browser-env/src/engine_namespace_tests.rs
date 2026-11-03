//! The engine namespace is held in a V8 private (miaskiewicz/turbo-dom#222) and `window` has
//! Chrome 156's Window interface chain (miaskiewicz/turbo-dom#223).
use super::*;

fn with_page(body: impl FnOnce(&mut v8::PinScope)) {
    crate::browser_env::tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, "<!doctype html><p id=p></p>");
    body(scope);
    reset();
}

/// Run `body` as page script (a plain function body, no namespace parameter).
fn page(scope: &mut v8::PinScope, body: &str) -> String {
    let code = v8::String::new(scope, &format!("(function(){{{body}\n}})()")).unwrap();
    v8::Script::compile(scope, code, None)
        .unwrap()
        .run(scope)
        .unwrap()
        .to_rust_string_lossy(scope)
}

#[test]
fn page_script_cannot_reach_the_engine_namespace() {
    with_page(|scope| {
        assert_eq!(
            page(scope, include_str!("engine_namespace_probe.js")),
            r#"{"symbolFor":[true,true,true,true],"inOperator":false,"ownSymbols":[],"windowOwnSymbols":[],"reflectSymbols":[],"descriptor":true,"found":[]}"#
        );
    });
}

#[test]
fn page_defined_ts_symbol_is_inert() {
    with_page(|scope| {
        page(
            scope,
            "globalThis[Symbol.for('ts')] = { makeNativeFunction: 1 }; return ''",
        );
        let value = run_engine_script(scope, "return typeof __tsNs.makeNativeFunction").unwrap();
        assert_eq!(value.to_rust_string_lossy(scope), "function");
    });
}

#[test]
fn engine_scripts_and_rust_share_one_namespace() {
    with_page(|scope| {
        let value = run_engine_script(
            scope,
            "return [typeof __tsNs.makeNativeFunction, typeof __tsNs.makeNamedObject, typeof __tsNs.intrinsics, typeof __tsNs.DOMException, this === globalThis].join()",
        )
        .unwrap();
        assert_eq!(
            value.to_rust_string_lossy(scope),
            "function,function,object,function,true"
        );
        let namespace = engine_namespace(scope).unwrap();
        let script = run_engine_script(scope, "return __tsNs").unwrap();
        assert!(script.strict_equals(namespace.into()));
        assert!(engine_namespace(scope)
            .unwrap()
            .strict_equals(namespace.into()));
        let value = engine_expression(scope, "  (function (a) { return a + 1; });\n").unwrap();
        let function = v8::Local::<v8::Function>::try_from(value).unwrap();
        let undefined = v8::undefined(scope);
        let one = v8::Number::new(scope, 1.0);
        let two = function
            .call(scope, undefined.into(), &[one.into()])
            .unwrap();
        assert_eq!(two.number_value(scope), Some(2.0));
    });
}

#[test]
fn run_engine_script_reports_compile_and_runtime_failures_as_none() {
    with_page(|scope| {
        let tc = std::pin::pin!(v8::TryCatch::new(scope));
        let tc = &mut tc.init();
        assert!(run_engine_script(tc, "return (").is_none());
        assert!(tc.has_caught());
        tc.reset();
        assert!(run_engine_script(tc, "throw new RangeError('x')").is_none());
        assert!(tc.has_caught());
    });
}

#[test]
fn window_has_chromes_window_interface_chain() {
    with_page(|scope| {
        let mut actual: serde_json::Value =
            serde_json::from_str(&page(scope, include_str!("window_interface_probe.js"))).unwrap();
        let mut expected: serde_json::Value =
            serde_json::from_str(include_str!("window_interface_chrome156.json")).unwrap();
        // EventTarget.prototype's own count differs by Chrome's Observable `when`, which the
        // binding does not ship (outside the Window chain this test pins).
        assert_eq!(expected["chain"][2]["own"], 6);
        actual["chain"][2]["own"] = 6.into();
        assert_eq!(actual, expected);
    });
}

/// An embedder whose global template already makes the global a `Window` instance with an
/// immutable prototype (turbo-surf's WindowProxy) keeps its interface object, and the binding
/// completes the chain above `Window.prototype`.
#[test]
fn embedder_window_template_keeps_its_interface_and_gets_the_chain() {
    crate::browser_env::tests::init_v8();
    let isolate = &mut crate::browser_env::tests::plain_isolate();
    v8::scope!(let scope, isolate);
    let constructor = v8::FunctionTemplate::new(
        scope,
        |_: &mut v8::PinScope, _: v8::FunctionCallbackArguments, _: v8::ReturnValue| {},
    );
    let name = v8::String::new(scope, "Window").unwrap();
    constructor.set_class_name(name);
    let global = constructor.instance_template(scope);
    global.set_with_attr(
        name.into(),
        constructor.into(),
        v8::PropertyAttribute::DONT_ENUM,
    );
    global.set_immutable_proto();
    let context = v8::Context::new(
        scope,
        v8::ContextOptions {
            global_template: Some(global),
            ..Default::default()
        },
    );
    let scope = &mut v8::ContextScope::new(scope, context);
    page(scope, "globalThis.__embedderWindow = Window; return ''");
    install_with_html(scope, "<!doctype html>");
    assert_eq!(
        page(
            scope,
            "const P = Object.getPrototypeOf, tag = o => Object.prototype.hasOwnProperty.call(o, Symbol.toStringTag) ? o[Symbol.toStringTag] : null; \
             return JSON.stringify([Window === __embedderWindow, P(window) === Window.prototype, tag(P(window)), tag(P(P(window))), P(P(P(window))) === EventTarget.prototype, \
             window instanceof EventTarget, Window.PERSISTENT, Window.prototype.TEMPORARY, Object.hasOwn(window, 'addEventListener'), \
             (() => { let n = 0; addEventListener('x', () => n++); dispatchEvent(new Event('x')); return n; })()])"
        ),
        r#"[true,true,"Window","WindowProperties",true,true,1,0,false,1]"#
    );
    reset();
}

fn wrapped_engine_source(source: &str) -> String {
    format!("(function({ENGINE_NAMESPACE_PARAM}){{{source}\n}})")
}

fn run_and_consumes(source: &str) -> (String, Vec<bool>) {
    let mut value = String::new();
    engine_code_cache::CONSUMED.with(|consumed| consumed.borrow_mut().clear());
    with_page(|scope| {
        engine_code_cache::CONSUMED.with(|consumed| consumed.borrow_mut().clear());
        value = run_engine_script(scope, source)
            .unwrap()
            .to_rust_string_lossy(scope);
    });
    let consumed = engine_code_cache::CONSUMED.with(|consumed| consumed.take());
    (value, consumed)
}

// Every page runs the same engine scripts: the second page compiles them from the bytecode the
// first produced instead of parsing the source again.
#[test]
fn engine_scripts_reuse_bytecode_across_pages() {
    let source = "return 'cached ' + (() => 6 * 7)();";
    let key = engine_code_cache::key(&wrapped_engine_source(source));
    assert!(engine_code_cache::get(key).is_none());
    assert_eq!(run_and_consumes(source), ("cached 42".to_owned(), vec![]));
    assert!(engine_code_cache::get(key).is_some());
    assert_eq!(
        run_and_consumes(source),
        ("cached 42".to_owned(), vec![true])
    );
}

// Data V8 rejects (another build, a hash collision) still runs the script, and the cache is
// replaced by a usable one.
#[test]
fn rejected_engine_bytecode_is_replaced() {
    let source = "return 'rejected ' + 1;";
    let key = engine_code_cache::key(&wrapped_engine_source(source));
    engine_code_cache::insert(key, vec![0; 64]);
    assert_eq!(
        run_and_consumes(source),
        ("rejected 1".to_owned(), vec![false])
    );
    assert_ne!(*engine_code_cache::get(key).unwrap(), vec![0; 64]);
    assert_eq!(
        run_and_consumes(source),
        ("rejected 1".to_owned(), vec![true])
    );
}
