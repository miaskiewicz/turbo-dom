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

/// A failure before `create_blob` (round-1 review #16): the snapshot creator must never be
/// dropped (rusty_v8 aborts the process on that, mid-unwind or not), so each failure leaks
/// it and yields no blob, leaving the thread able to build again. Installs then fall back to
/// running the engine scripts.
#[test]
fn a_failure_before_create_blob_yields_no_blob_without_aborting() {
    use startup_snapshot::{BuildFailure, FAIL_BUILD_FOR_TEST};
    crate::browser_env::tests::init_v8();
    for failure in [
        BuildFailure::IsolateData,
        BuildFailure::Panic,
        BuildFailure::RetainWrapper,
    ] {
        FAIL_BUILD_FOR_TEST.with(|at| at.set(Some(failure)));
        assert!(startup_snapshot::build().is_none(), "{failure:?}");
        FAIL_BUILD_FOR_TEST.with(|at| at.set(None));
        assert!(!startup_snapshot::building(), "{failure:?}");
        assert!(with_tree(|tree| tree.root()).is_none(), "{failure:?}");
    }
    assert_eq!(
        startup_snapshot::RETAINED_FOR_TEST.with(std::cell::Cell::get),
        Some(1),
        "the kept document.body is detected"
    );
    assert!(startup_snapshot::build().is_some());
}

/// The bootstrap stores engine helpers in context slots, whose annex is a heap pointer in
/// the context's embedder data. The blob must not carry that pointer: a context made from
/// it has no slots (reading one must not dereference the build isolate's freed annex).
#[test]
fn a_context_from_the_blob_carries_no_slot_annex() {
    crate::browser_env::tests::init_v8();
    let blob = startup_snapshot::build().expect("every callback is registered");
    let params = v8::CreateParams::default()
        .snapshot_blob(blob.data)
        .external_references(Cow::Borrowed(blob.references));
    let isolate = &mut v8::Isolate::new(params);
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let _scope = &mut v8::ContextScope::new(scope, context);
    assert!(context
        .get_slot::<event_listeners::WindowPhases>()
        .is_none());
    assert!(context.get_slot::<u8>().is_none());
    context.set_slot(std::rc::Rc::new(7u8));
    assert_eq!(context.get_slot::<u8>().as_deref(), Some(&7));
}
