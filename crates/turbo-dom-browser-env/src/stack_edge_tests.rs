//! Natives entered with almost no stack left (`stack_edge_probe.js`): every V8 call a
//! native makes can then fail with a pending RangeError, and the native must return it to
//! the page instead of unwrapping (a panic in a V8 callback aborts the process). Control:
//! `stack_edge_chrome156.json`, where each operation only ever throws RangeError.
use crate::browser_env::*;

#[test]
fn natives_at_the_stack_edge_throw_range_error_like_chrome156() {
    crate::browser_env::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(
        scope,
        "<!doctype html><html><body><div id=a><p>x</p></div><pre id=out></pre></body></html>",
    );
    let code = v8::String::new(scope, include_str!("stack_edge_probe.js")).unwrap();
    let result = v8::Script::compile(scope, code, None)
        .and_then(|script| script.run(scope))
        .map(|value| value.to_rust_string_lossy(scope))
        .expect("the probe completes");
    let actual: serde_json::Value = serde_json::from_str(&result).unwrap();
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("stack_edge_chrome156.json")).unwrap();
    let differences: Vec<_> = expected
        .as_object()
        .unwrap()
        .iter()
        .filter(|(name, value)| actual.get(name.as_str()) != Some(value))
        .map(|(name, value)| format!("{name}: {:?} != {value}", actual.get(name.as_str())))
        .collect();
    assert!(differences.is_empty(), "{differences:#?}");
}
