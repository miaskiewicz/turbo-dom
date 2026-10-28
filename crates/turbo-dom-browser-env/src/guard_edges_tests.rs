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
#[ignore = "turbo-dom#161: fails until the fix"]
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
