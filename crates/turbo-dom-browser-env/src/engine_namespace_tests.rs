//! The engine namespace is held in a V8 private (miaskiewicz/turbo-dom#222) and `window` has
//! Chrome 156's Window interface chain (miaskiewicz/turbo-dom#223).
use super::*;

fn with_page(body: impl FnOnce(&mut v8::PinScope)) {
    crate::browser_env::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
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
        page(scope, "globalThis[Symbol.for('ts')] = { makeNativeFunction: 1 }; return ''");
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
        assert!(engine_namespace(scope).unwrap().strict_equals(namespace.into()));
        let value = engine_expression(scope, "  (function (a) { return a + 1; });\n").unwrap();
        let function = v8::Local::<v8::Function>::try_from(value).unwrap();
        let undefined = v8::undefined(scope);
        let one = v8::Number::new(scope, 1.0);
        let two = function.call(scope, undefined.into(), &[one.into()]).unwrap();
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
        let actual: serde_json::Value =
            serde_json::from_str(&page(scope, include_str!("window_interface_probe.js"))).unwrap();
        let expected: serde_json::Value =
            serde_json::from_str(include_str!("window_interface_chrome156.json")).unwrap();
        assert_eq!(actual, expected);
    });
}
