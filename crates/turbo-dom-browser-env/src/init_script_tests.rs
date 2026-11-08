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
                        ""
                            | "text/javascript"
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
