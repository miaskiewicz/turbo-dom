//! Guard edges of the binding (crate-owned): hostile selectors, declarations, keyframe values
//! and view-transition roots against their Chrome 156 control (`guard_edges_probe.js` →
//! `guard_edges_chrome156.json`, recorded by `scripts/trusted-input-chrome-control.mjs`), and
//! the regression tests of the bugs they found.
use crate::browser_env::crate_surface::harness_tests::thrown;
use crate::browser_env::crate_surface::member_edges_tests::unexpected;
use crate::browser_env::reference_target::probe_harness::{control, run};
use crate::browser_env::*;

const PROBE: &str = include_str!("guard_edges_probe.js");

/// KNOWN_GAP rows `(path, binding value)`: leaves where the binding still differs from
/// Chrome 156. Each must still differ exactly so; a fix removes its row.
const KNOWN_GAPS: &[(&str, &str)] = &[
    // miaskiewicz/turbo-dom#162: deferred zoom values are dropped.
    ("result.declarations.zoom: var(--z) !important[0]", "\"\""),
    ("result.declarations.zoom: var(--z) !important[1]", "\"\""),
    ("result.declarations.zoom: var(--z)[0]", "\"\""),
    ("result.declarations.zoom: var(--z)[1]", "\"\""),
    ("result.declarations.zoom: env(x)[0]", "\"\""),
    ("result.declarations.zoom: env(x)[1]", "\"\""),
    // miaskiewicz/turbo-dom#134: @keyframes cssText.
    (
        "result.declarations.@keyframes k { 0% {} }",
        "\"@keyframes k { 0% {  } }\"",
    ),
    (
        "result.declarations.@keyframes k { x {} }",
        "\"@keyframes k { x {  } }\"",
    ),
    (
        "result.declarations.@keyframes k { 10%, 20% {} from {} }",
        "\"@keyframes k { 10%, 20% {  } from {  } }\"",
    ),
    (
        "result.declarations.@keyframes k { -1% {} }",
        "\"@keyframes k { -1% {  } }\"",
    ),
    (
        "result.declarations.@keyframes k { 101% {} }",
        "\"@keyframes k { 101% {  } }\"",
    ),
];

#[test]
fn guard_edges_match_chrome156() {
    let expected = control(include_str!("guard_edges_chrome156.json"), PROBE);
    assert_eq!(
        unexpected(&run(PROBE), &expected, KNOWN_GAPS),
        Vec::<String>::new()
    );
}

/// Run `source` (an expression) in a fresh page and return its JSON serialization.
fn page_json(html: &str, source: &str) -> String {
    crate::browser_env::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, html);
    v8::tc_scope!(let scope, scope);
    let code = v8::String::new(scope, &format!("JSON.stringify({source})")).unwrap();
    let value = v8::Script::compile(scope, code, None)
        .and_then(|script| script.run(scope))
        .map(|value| value.to_rust_string_lossy(scope));
    let value = value.unwrap_or_else(|| panic!("page threw: {}", thrown(scope.exception(), scope)));
    reset();
    value
}

/// turbo-dom#160: an invalid `:popover-open(...)` inside a forgiving `:is()` / `:where()`
/// list drops only its branch (Chrome 156); outside one it is still a SyntaxError.
#[test]
fn forgiving_lists_drop_invalid_popover_open_branches() {
    assert_eq!(
        page_json(
            "<!doctype html><div id=p popover=manual></div><div id=d></div>",
            "(() => { const p = document.getElementById('p'), d = document.getElementById('d'); \
             const name = run => { try { return run(); } catch (error) { return error.name; } }; \
             const before = [document.querySelector(':is(:popover-open(x))'), d.matches(':where(:popover-open(x), div)'), p.matches(':is(:popover-open, :popover-open(x))')]; \
             p.showPopover(); \
             return [...before, p.matches(':is(:popover-open, :popover-open(x))'), d.matches(':is(:popover-open(x))'), \
               document.querySelectorAll(':where(:popover-open(), :popover-open)').length, name(() => d.matches(':popover-open(x)'))]; })()",
        ),
        r#"[null,true,false,true,false,1,"SyntaxError"]"#
    );
}

fn function<'s>(scope: &mut v8::PinScope<'s, '_>, source: &str) -> v8::Local<'s, v8::Function> {
    let code = v8::String::new(scope, source).unwrap();
    let value = v8::Script::compile(scope, code, None)
        .and_then(|script| script.run(scope))
        .unwrap();
    v8::Local::<v8::Function>::try_from(value).unwrap()
}

/// Install the document and every host hook that retains a helper, with the test-only
/// watchdog armed to terminate at the `at`-th retained helper. Returns `None` when fewer
/// helpers were retained (the watchdog never fired), else whether the realm still runs
/// script once the termination is cancelled.
fn install_terminated_at_helper(at: usize) -> Option<String> {
    crate::browser_env::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    let fired = {
        v8::tc_scope!(let scope, scope);
        dom_ownership::TERMINATE_AT_HELPER.with(|helper| helper.set(Some(at)));
        install_with_html(scope, "<!doctype html><p id=p>x</p>");
        let allow = function(scope, "(() => true)");
        install_inline_event_handlers(scope, allow, "");
        install_css_policy(scope, allow);
        install_form_value_reader(scope, allow);
        install_form_submission_handler(scope, allow);
        install_trusted_types_policy(scope, allow);
        let armed = dom_ownership::TERMINATE_AT_HELPER.with(|helper| helper.replace(None));
        let fired = armed.is_none();
        scope.cancel_terminate_execution();
        scope.reset();
        fired
    };
    let after = fired.then(|| {
        let code = v8::String::new(scope, "String(1 + 1)").unwrap();
        v8::Script::compile(scope, code, None)
            .and_then(|script| script.run(scope))
            .map_or("threw".to_owned(), |value| {
                value.to_rust_string_lossy(scope)
            })
    });
    reset();
    after
}

/// An embedder's watchdog firing while the binding retains any of its helpers (during
/// install or a host hook's install) unwinds the install without panicking, and once the
/// termination is cancelled the realm runs script again.
#[test]
fn termination_at_every_install_helper_unwinds() {
    let mut count = 0;
    while let Some(after) = install_terminated_at_helper(count) {
        assert_eq!(after, "2", "helper {count}");
        count += 1;
    }
    // The install and the five host hooks retain dozens of helpers.
    assert!(count > 30, "{count} helpers");
}

/// Inline event handlers installed without a document base compile with Chrome's
/// about:blank resource name.
#[test]
fn inline_handlers_without_a_base_compile_as_about_blank() {
    crate::browser_env::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(
        scope,
        "<!doctype html><div id=d onclick=\"globalThis.where = new Error().stack.split('\\n')[1].trim()\"></div>",
    );
    let allow = function(scope, "(() => true)");
    install_inline_event_handlers(scope, allow, "");
    let code = v8::String::new(scope, "document.getElementById('d').click(); where").unwrap();
    let value = v8::Script::compile(scope, code, None)
        .and_then(|script| script.run(scope))
        .unwrap()
        .to_rust_string_lossy(scope);
    assert_eq!(value, "at HTMLDivElement.onclick (about:blank:1:20)");
    reset();
}
