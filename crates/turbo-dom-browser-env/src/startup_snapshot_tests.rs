//! turbo-dom#249 Phase B: a startup snapshot of the bootstrapped shell builds and
//! deserializes. A callback missing from the external reference table aborts V8 in
//! `create_blob` or in deserialization, which fails this test.
use super::*;
use std::borrow::Cow;

#[test]
fn the_bootstrapped_shell_snapshots_and_deserializes() {
    crate::browser_env::tests::init_v8();
    let blob = startup_snapshot::build().expect("every callback is registered");
    assert!(blob.references.len() > 100, "{}", blob.references.len());
    let params = v8::CreateParams::default()
        .snapshot_blob(blob.data)
        .external_references(Cow::Borrowed(blob.references));
    let isolate = &mut v8::Isolate::new(params);
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    let code = v8::String::new(
        scope,
        "[typeof document.createElement, typeof HTMLElement, typeof Node.prototype.appendChild, Object.getPrototypeOf(window) === Window.prototype].join()",
    )
    .unwrap();
    let result = v8::Script::compile(scope, code, None)
        .unwrap()
        .run(scope)
        .unwrap()
        .to_rust_string_lossy(scope);
    assert_eq!(result, "function,function,function,true");
}

/// A bootstrap that handed V8 a callback outside the table yields no blob instead of
/// aborting V8 in create_blob.
#[test]
fn a_callback_outside_the_table_yields_no_blob() {
    crate::browser_env::tests::init_v8();
    external_refs::UNTABLED_FOR_TEST.with(|extra| extra.set(Some(1)));
    assert!(startup_snapshot::build().is_none());
    assert!(!startup_snapshot::building());
}
