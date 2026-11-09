//! `snapshot_blob()` blobs must stay compatible with later plain isolates in the same
//! process (turbo-dom#392): the process may hold V8's shared read-only heap live through
//! any default-snapshot isolate (a worker isolate, a leaked creator from a failed build).
//! A blob built with no isolate alive, then used while a plain isolate exists, used to
//! crash in `Deserializer::ReadReadOnlyHeapRef` (SharedHeapDeserializer::DeserializeStringTable
//! OOB on darwin-arm64).
//!
//! Its own test binary, so its own process (one plain isolate, one blob isolate, nothing
//! else running). The build keeps a default-snapshot isolate alive on the build thread so
//! the creator shares V8's read-only heap layout; the assertion here is that the blob
//! isolate instantiates (any V8 abort during deserialization would SIGSEGV this binary).
use turbo_dom_browser_env::browser_env::snapshot_blob;

#[test]
fn a_snapshot_blob_isolate_runs_alongside_a_plain_isolate() {
    let platform = v8::new_default_platform(0, false).make_shared();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();

    // Build the blob with no other isolate alive (the fix must anchor the shared read-only
    // heap on the build thread itself, independent of what callers do).
    let blob = snapshot_blob().expect("the process blob builds");

    // A default-snapshot isolate alive on another thread: holds the process-shared
    // read-only heap while the blob isolate below deserializes.
    let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
    let (ready_tx, ready_rx) = std::sync::mpsc::channel::<()>();
    let handle = std::thread::spawn(move || {
        let _plain = v8::Isolate::new(Default::default());
        ready_tx.send(()).unwrap();
        release_rx.recv().ok();
    });
    ready_rx.recv().unwrap();

    // Deserializing from the blob while the plain isolate is alive: before the fix this
    // aborted in ReadReadOnlyHeapRef; after the fix the blob's RO layout matches V8's
    // default, so the isolate instantiates and runs `typeof window`.
    {
        let isolate = &mut v8::Isolate::new(blob.create_params());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        let code = v8::String::new(scope, "typeof window").unwrap();
        let out = v8::Script::compile(scope, code, None)
            .unwrap()
            .run(scope)
            .unwrap()
            .to_rust_string_lossy(scope);
        assert_eq!(out, "object");
    }

    release_tx.send(()).ok();
    handle.join().unwrap();
}
