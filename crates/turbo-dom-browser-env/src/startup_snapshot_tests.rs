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

/// An embedder that bakes its own startup context (turbo-test's framework layer) gets the
/// window into the same blob: the default context is the embedder's alone (no window, for
/// its non-DOM files), and context [`SNAPSHOT_WINDOW_CONTEXT`] is the embedder's prelude, then
/// the engine bootstrap (the order of a prelude, then an install). With snapshot installs on,
/// every window context the isolate makes (an embedder reusing its isolate makes several)
/// installs its document without running an engine script.
#[test]
fn an_embedder_prelude_shares_the_blob_and_contexts_install_from_it() {
    fn prelude(scope: &mut v8::PinScope) {
        let code = v8::String::new(scope, "globalThis.__prelude = typeof document;").unwrap();
        v8::Script::compile(scope, code, None).unwrap().run(scope).unwrap();
    }
    fn eval(scope: &mut v8::PinScope, code: &str) -> String {
        let code = v8::String::new(scope, code).unwrap();
        let value = v8::Script::compile(scope, code, None).unwrap().run(scope).unwrap();
        value.to_rust_string_lossy(scope)
    }
    crate::browser_env::tests::init_v8();
    let blob = snapshot_blob_with(prelude).expect("every callback is registered");
    let isolate = &mut v8::Isolate::new(blob.create_params());
    {
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        assert_eq!(eval(scope, "[__prelude, typeof document, typeof Node].join()"), "undefined,undefined,undefined");
    }
    set_snapshot_install(true);
    for page in ["<p id=x>one</p>", "<p id=x>two</p>"] {
        {
            v8::scope!(let scope, isolate);
            let context =
                v8::Context::from_snapshot(scope, SNAPSHOT_WINDOW_CONTEXT, Default::default())
                    .expect("the blob holds the window context");
            let scope = &mut v8::ContextScope::new(scope, context);
            let before = ENGINE_SCRIPTS_RUN.with(std::cell::Cell::get);
            install_with_html(scope, page);
            assert_eq!(ENGINE_SCRIPTS_RUN.with(std::cell::Cell::get), before, "{page}");
            let text = if page.contains("one") { "one" } else { "two" };
            assert_eq!(
                eval(scope, "[__prelude, document.getElementById('x').textContent, document.body.firstChild instanceof HTMLParagraphElement].join()"),
                format!("undefined,{text},true")
            );
        }
        reset();
    }
    set_snapshot_install(false);
}

/// An embedder can keep a [`snapshot_blob_with`] blob across processes (V8 rejects bytecode
/// caches made against another blob, and a rebuilt blob is not byte-identical): the table a
/// later process records for it ([`snapshot_references`]) is the one the blob was built with,
/// with a fingerprint that is stable for the binary, so saved bytes deserialize and install.
#[test]
fn saved_blob_bytes_install_with_the_recorded_reference_table() {
    fn prelude(_scope: &mut v8::PinScope) {}
    crate::browser_env::tests::init_v8();
    let blob = snapshot_blob_with(prelude).expect("every callback is registered");
    let saved: &'static [u8] = Box::leak(blob.data.to_vec().into_boxed_slice());
    let (references, fingerprint) = snapshot_references();
    let (_, again) = snapshot_references();
    assert_eq!(fingerprint, again, "stable within the process");
    assert!(references.len() >= blob.references.len());
    for (recorded, built) in references.iter().zip(blob.references) {
        assert_eq!(unsafe { recorded.pointer }, unsafe { built.pointer });
    }
    let params = v8::CreateParams::default()
        .snapshot_blob(v8::StartupData::from(saved))
        .external_references(Cow::Borrowed(references));
    let isolate = &mut v8::Isolate::new(params);
    set_snapshot_install(true);
    {
        v8::scope!(let scope, isolate);
        let context = v8::Context::from_snapshot(scope, SNAPSHOT_WINDOW_CONTEXT, Default::default())
            .expect("the saved blob holds the window context");
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(scope, "<b id=b>saved</b>");
        let code = v8::String::new(scope, "document.getElementById('b').textContent").unwrap();
        let text = v8::Script::compile(scope, code, None).unwrap().run(scope).unwrap();
        assert_eq!(text.to_rust_string_lossy(scope), "saved");
    }
    reset();
    set_snapshot_install(false);
}
