//! Shared failure-message helpers of the binding's test harnesses (crate-owned). Each
//! harness keeps its failure branch on the line that runs, so a passing suite executes it;
//! the helpers' own tests drive the failure inputs.
use crate::browser_env::*;

/// The text of a pending exception for a harness failure message, `"no exception"` if the
/// script failed without one (termination).
pub(in crate::browser_env) fn thrown(
    error: Option<v8::Local<v8::Value>>,
    scope: &mut v8::PinScope,
) -> String {
    error.map_or_else(
        || "no exception".to_owned(),
        |error| error.to_rust_string_lossy(scope),
    )
}

#[test]
fn thrown_reports_the_exception_or_its_absence() {
    crate::browser_env::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    v8::tc_scope!(let scope, scope);
    assert_eq!(thrown(scope.exception(), scope), "no exception");
    let code = v8::String::new(scope, "throw new TypeError('boom')").unwrap();
    assert!(v8::Script::compile(scope, code, None)
        .unwrap()
        .run(scope)
        .is_none());
    assert_eq!(thrown(scope.exception(), scope), "TypeError: boom");
}
