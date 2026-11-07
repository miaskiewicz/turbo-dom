//! `mark_worker_context` refuses a context made from a `snapshot_blob()` isolate
//! (turbo-dom#385, #389 review): the blob's default context IS the bootstrapped window, so
//! the context stays an ordinary window and an install into it still works.
//!
//! Its own test binary, so its own process: the lib suite's failed-build tests leak a
//! snapshot creator, which keeps V8's process-shared read-only heap of the default snapshot
//! alive, and an isolate deserialized from a process blob built before that leak resolves
//! its read-only references against the wrong heap (SIGSEGV under the serial coverage run;
//! turbo-dom#392). One test, so no other isolate is alive in this process.
use turbo_dom_browser_env::browser_env::{install_with_html, reset, snapshot_blob};
use turbo_dom_browser_env::{mark_worker_context, WindowAlreadyInstalled};

fn eval(scope: &mut v8::PinScope, source: &str) -> String {
    let code = v8::String::new(scope, source).unwrap();
    v8::Script::compile(scope, code, None)
        .unwrap()
        .run(scope)
        .unwrap()
        .to_rust_string_lossy(scope)
}

#[test]
fn a_snapshot_blob_context_is_not_a_worker_context() {
    let platform = v8::new_default_platform(0, false).make_shared();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();
    let blob = snapshot_blob().expect("the process blob builds");
    let isolate = &mut v8::Isolate::new(blob.create_params());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    assert_eq!(
        mark_worker_context(scope, context),
        Err(WindowAlreadyInstalled)
    );
    assert!(WindowAlreadyInstalled
        .to_string()
        .contains("snapshot_blob_with"));
    // Left unmarked: an install into it still binds the page's document (a worker-marked
    // context would get no `window` / `document`).
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, "<!doctype html><p></p>");
    assert_eq!(eval(scope, "typeof window"), "object");
    assert_eq!(eval(scope, "document.querySelector('p') !== null"), "true");
    reset();
}
