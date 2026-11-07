//! Minimum worker opt-out hook (miaskiewicz/turbo-dom#385, for turbo-surf #203):
//! a context tagged with [`mark_worker_context`] gets no DOM template installed.
use super::*;

/// Evaluate `source` in the current context and return its stringified result.
fn eval(scope: &mut v8::PinScope, source: &str) -> String {
    let code = v8::String::new(scope, source).unwrap();
    v8::Script::compile(scope, code, None)
        .unwrap()
        .run(scope)
        .unwrap()
        .to_rust_string_lossy(scope)
}

/// `install_with_html` into a context marked as a worker leaves `globalThis` free of the
/// DOM template — `window`, `document`, `Element` and every other DOM-side global are
/// absent. A plain context in the same isolate still gets them, so we are testing the
/// per-context opt-out and not a disabled binding.
#[test]
fn mark_worker_context_skips_dom_install() {
    crate::browser_env::tests::init_v8();
    // A plain isolate on every path: a snapshot_blob() isolate's contexts already hold the
    // window (see `a_snapshot_blob_context_is_not_a_worker_context`).
    let isolate = &mut crate::browser_env::tests::plain_isolate();
    v8::scope!(let scope, isolate);

    // A fresh worker-marked context: install must not add window/document/Element.
    let worker_context = v8::Context::new(scope, Default::default());
    assert_eq!(mark_worker_context(scope, worker_context), Ok(()));
    {
        let scope = &mut v8::ContextScope::new(scope, worker_context);
        install_with_html(scope, "<!doctype html><p></p>");
        // Each name is probed individually so a leak of any one of them fails loudly.
        for name in [
            "window",
            "document",
            "Element",
            "HTMLElement",
            "Node",
            "Document",
            "Window",
            "HTMLDocument",
            "navigator",
        ] {
            let source = format!("typeof globalThis.{name}");
            assert_eq!(
                eval(scope, &source),
                "undefined",
                "{name} must be absent on a worker-marked context"
            );
            // Bare lookups also miss (a ReferenceError the page script can't catch is what
            // the embedder is preventing by opting out).
            let bare = format!(
                "(() => {{ try {{ return typeof {name}; }} catch (e) {{ return 'threw'; }} }})()"
            );
            assert_eq!(
                eval(scope, &bare),
                "undefined",
                "bare {name} must be absent on a worker-marked context"
            );
        }
        // The worker's globalThis is otherwise usable — turbo-surf's worker scope still
        // needs plain ECMAScript globals.
        assert_eq!(eval(scope, "typeof Array"), "function");
    }
    reset();

    // A sibling context without the mark still gets the full DOM install: the gate is
    // per-context, not isolate-wide.
    let page_context = v8::Context::new(scope, Default::default());
    {
        let scope = &mut v8::ContextScope::new(scope, page_context);
        install_with_html(scope, "<!doctype html><p></p>");
        assert_eq!(eval(scope, "typeof window"), "object");
        assert_eq!(eval(scope, "typeof document"), "object");
        assert_eq!(eval(scope, "typeof Element"), "function");
    }
    reset();
}

/// A context made from a `snapshot_blob()` isolate already holds the bootstrapped window, so
/// `mark_worker_context` refuses it and leaves it an ordinary window; a context that an install
/// already ran in is refused the same way (turbo-dom#389 review).
#[test]
fn a_snapshot_blob_context_is_not_a_worker_context() {
    crate::browser_env::tests::init_v8();
    let blob = snapshot_blob().expect("the process blob builds");
    let isolate = &mut v8::Isolate::new(blob.create_params());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    assert_eq!(
        mark_worker_context(scope, context),
        Err(WindowAlreadyInstalled)
    );
    assert!(!is_worker_context(context));
    assert!(WindowAlreadyInstalled
        .to_string()
        .contains("snapshot_blob_with"));
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, "<!doctype html><p></p>");
    assert_eq!(eval(scope, "typeof window"), "object");
    assert_eq!(eval(scope, "document.querySelector('p') !== null"), "true");
    reset();
}

#[test]
fn an_installed_context_is_not_a_worker_context() {
    crate::browser_env::tests::init_v8();
    let isolate = &mut crate::browser_env::tests::plain_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, "<!doctype html><p></p>");
    assert_eq!(
        mark_worker_context(scope, context),
        Err(WindowAlreadyInstalled)
    );
    assert!(!is_worker_context(context));
    reset();
}
