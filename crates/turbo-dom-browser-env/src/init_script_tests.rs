//! turbo-dom#394: the embedder-supplied init script must run BEFORE the parser's
//! inline `<script>` elements, like CDP's `Page.addScriptToEvaluateOnNewDocument` /
//! Playwright's `addInitScript`. Surfaced by turbo-surf#240 (page.route fetch
//! interception missing inline-script fetches).
use super::*;
use crate::browser_env::{add_init_script, clear_init_scripts, install_with_html, reset};
use turbo_dom_parser::rtdom::Tree;

/// Compile+run one inline parser script's source in the current context, as the
/// embedder's parser lifecycle does (see `script_state::prepare_script`).
fn run_inline_parser_scripts(scope: &mut v8::PinScope) {
    let scripts = {
        let root = with_tree(Tree::root).unwrap();
        let handles = DOM.with(|dom| {
            dom.borrow()
                .as_ref()
                .map(|dom| dom.parser_scripts.clone())
                .unwrap_or_default()
        });
        let _ = root;
        handles
            .into_iter()
            .filter_map(|handle| {
                let node = wrap(scope, handle);
                let prepared = prepare_script(scope, node)?;
                (prepared.src.is_none()
                    && matches!(
                        prepared.kind.as_str(),
                        "" | "text/javascript"
                            | "application/javascript"
                            | "text/ecmascript"
                            | "application/ecmascript"
                    ))
                .then_some(prepared.source)
            })
            .collect::<Vec<_>>()
    };
    for source in scripts {
        let source = v8::String::new(scope, &source).unwrap();
        if let Some(script) = v8::Script::compile(scope, source, None) {
            script.run(scope);
        }
    }
}

fn eval_string(scope: &mut v8::PinScope, source: &str) -> Option<String> {
    let s = v8::String::new(scope, source).unwrap();
    let script = v8::Script::compile(scope, s, None)?;
    let value = script.run(scope)?;
    Some(value.to_rust_string_lossy(scope))
}

#[test]
fn init_script_runs_before_parser_inline_scripts() {
    super::tests::init_v8();
    clear_init_scripts();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);

    // Embedder registers an init script BEFORE install (like page.route's fetch
    // monkey-patch). The script publishes a sentinel on globalThis.
    add_init_script("globalThis.__initSentinel = 'installed';");

    // Document whose inline script reads the sentinel at parse-time. If the init
    // script ran AFTER the inline parser script, __seen would be `undefined`.
    install_with_html(
        scope,
        "<!doctype html><html><head><script>globalThis.__seen = globalThis.__initSentinel;</script></head><body></body></html>",
    );
    run_inline_parser_scripts(scope);

    assert_eq!(
        eval_string(scope, "String(globalThis.__seen)").as_deref(),
        Some("installed"),
        "inline parser script saw the init-script sentinel (turbo-dom#394)",
    );

    clear_init_scripts();
    reset();
}

#[test]
fn init_scripts_run_in_registration_order_and_survive_across_installs() {
    super::tests::init_v8();
    clear_init_scripts();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);

    add_init_script("globalThis.__trace = (globalThis.__trace || '') + 'a';");
    add_init_script("globalThis.__trace = (globalThis.__trace || '') + 'b';");

    install_with_html(
        scope,
        "<!doctype html><html><head><script>globalThis.__trace = (globalThis.__trace || '') + 'parser';</script></head><body></body></html>",
    );
    run_inline_parser_scripts(scope);

    assert_eq!(
        eval_string(scope, "String(globalThis.__trace)").as_deref(),
        Some("abparser"),
        "init scripts run in registration order, and before inline parser scripts",
    );

    clear_init_scripts();
    reset();
}

#[test]
fn reset_clears_init_scripts_like_other_per_thread_state() {
    // Review R1 M6: `reset()` historically cleared every per-thread cell
    // except `INIT_SCRIPTS`, so an embedder that teardown-reset without
    // also calling `clear_init_scripts()` leaked init scripts into the
    // next isolate's installs. A `reset()` is the "clear per-thread
    // state" boundary; the init-script registry belongs there.
    super::tests::init_v8();
    clear_init_scripts();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);

    add_init_script("globalThis.__shouldNotLeak = 'leaked';");
    reset();
    install_with_html(
        scope,
        "<!doctype html><html><head><script>\
         globalThis.__after = typeof globalThis.__shouldNotLeak;\
         </script></head><body></body></html>",
    );
    run_inline_parser_scripts(scope);

    assert_eq!(
        eval_string(scope, "String(globalThis.__after)").as_deref(),
        Some("undefined"),
        "init scripts registered before reset() must not survive it",
    );
    reset();
}

/// turbo-dom#394 R1 (expert H4): a throwing init script must not swallow its
/// error silently. `run_init_scripts` wraps each run in a `v8::TryCatch` so the
/// exception is reported through the window's error channel (an `ErrorEvent`
/// dispatched to `window.onerror`, like every other reported engine error), and
/// so the pending V8 exception does not leak into later init scripts or into
/// the embedder's inline parser scripts.
#[test]
fn init_script_exceptions_are_reported_and_isolated_from_later_scripts() {
    super::tests::init_v8();
    clear_init_scripts();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);

    // init 1: an onerror handler collects every reported error's message.
    add_init_script(
        "globalThis.__initErrors = [];\
         globalThis.addEventListener('error', (event) => __initErrors.push(String(event.message)));",
    );
    // init 2 throws; init 3 must still run and publish its sentinel
    // (turbo-dom#394 R1 — a pending V8 exception from init 2 would otherwise
    // abort the compile/run of init 3 and reach init 1's listener as nothing).
    add_init_script("throw new Error('init2-boom');");
    add_init_script("globalThis.__initThird = 'ran';");

    install_with_html(
        scope,
        "<!doctype html><html><head><script>\
         globalThis.__inlineSentinel = String(globalThis.__initThird);\
         globalThis.__inlineSawError = JSON.stringify(globalThis.__initErrors || []);\
         </script></head><body></body></html>",
    );
    run_inline_parser_scripts(scope);

    // The throw must be reported to the window's error channel. Without the
    // TryCatch isolation added for R1 H4, the exception escapes to V8 without
    // a report and `__initErrors` stays empty.
    let errors = eval_string(scope, "JSON.stringify(globalThis.__initErrors)")
        .unwrap_or_default();
    assert!(
        errors.contains("init2-boom"),
        "init-script throw was reported to window error channel (got {errors:?})",
    );
    // init 3 still runs even though init 2 threw.
    assert_eq!(
        eval_string(scope, "String(globalThis.__initThird)").as_deref(),
        Some("ran"),
        "third init script runs after the second throws (turbo-dom#394 R1)",
    );
    // The inline parser script runs with a clean scope: it sees init 3's
    // sentinel and the caught error.
    assert_eq!(
        eval_string(scope, "String(globalThis.__inlineSentinel)").as_deref(),
        Some("ran"),
        "inline parser script runs with a clean scope (turbo-dom#394 R1)",
    );
    assert!(
        eval_string(scope, "String(globalThis.__inlineSawError)")
            .as_deref()
            .is_some_and(|s| s.contains("init2-boom")),
        "inline parser script sees the earlier init-script throw in window errors",
    );

    clear_init_scripts();
    reset();
}
