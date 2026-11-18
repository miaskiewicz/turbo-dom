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
const KNOWN_GAPS: &[(&str, &str)] = &[];

#[test]
fn guard_edges_match_chrome156() {
    let expected = control(include_str!("guard_edges_chrome156.json"), PROBE);
    assert_eq!(
        unexpected(&run(PROBE), &expected, KNOWN_GAPS),
        Vec::<String>::new()
    );
}

/// Run `source` (an expression) in a fresh page and return its JSON serialization.
pub(super) fn page_json(html: &str, source: &str) -> String {
    crate::browser_env::tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
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
    let isolate = &mut crate::browser_env::tests::plain_isolate();
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
    let isolate = &mut crate::browser_env::tests::new_isolate();
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

fn terminate_now(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    scope.terminate_execution();
}

/// Run `source` in a fresh page with `__terminate()` bound (and its microtasks); it must end
/// by termination. Then cancel it, run the microtasks and return `after`'s JSON.
fn terminated(html: &str, source: &str, after: &str) -> String {
    crate::browser_env::tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, html);
    let global = context.global(scope);
    bind_method(scope, global, "__terminate", terminate_now);
    {
        v8::tc_scope!(let scope, scope);
        let code = v8::String::new(scope, source).unwrap();
        if v8::Script::compile(scope, code, None)
            .unwrap()
            .run(scope)
            .is_some()
        {
            // The termination lands in a microtask (an observer callback).
            scope.perform_microtask_checkpoint();
        }
        assert!(
            scope.is_execution_terminating() || scope.has_terminated(),
            "{source} did not terminate"
        );
        scope.cancel_terminate_execution();
        scope.reset();
    }
    scope.perform_microtask_checkpoint();
    let code = v8::String::new(scope, &format!("JSON.stringify({after})")).unwrap();
    let value = v8::Script::compile(scope, code, None)
        .unwrap()
        .run(scope)
        .unwrap()
        .to_rust_string_lossy(scope);
    reset();
    value
}

/// A beforetoggle listener that terminates while a popover hides: the popover stays open and
/// is no longer busy, so it hides normally once the termination is cancelled.
#[test]
fn termination_in_a_hiding_beforetoggle_keeps_the_popover_open() {
    assert_eq!(
        terminated(
            "<!doctype html><div id=p popover=manual></div>",
            "globalThis.p = document.getElementById('p'); p.showPopover(); globalThis.log = []; \
             p.addEventListener('beforetoggle', event => { log.push(event.newState); if (log.length === 1) __terminate(); }); \
             p.hidePopover(); log.push('end')",
            "[log.slice(), p.matches(':popover-open'), (p.hidePopover(), p.matches(':popover-open')), log]"
        ),
        r#"[["closed"],true,false,["closed","closed"]]"#
    );
}

/// A mutation observer callback that terminates ends the notification: the next observer
/// is not notified in that checkpoint.
#[test]
fn termination_in_an_observer_callback_skips_later_observers() {
    assert_eq!(
        terminated(
            "<!doctype html><div id=t></div>",
            "globalThis.log = []; const t = document.getElementById('t'); \
             new MutationObserver(() => { log.push('first'); __terminate(); }).observe(t, { attributes: true }); \
             new MutationObserver(() => log.push('second')).observe(t, { attributes: true }); \
             t.id = 'u'; Promise.resolve().then(() => log.push('microtask'))",
            "log"
        ),
        r#"["first"]"#
    );
}

/// A slotchange listener that terminates ends the slot signals of that checkpoint.
#[test]
fn termination_in_a_slotchange_listener_skips_later_slots() {
    assert_eq!(
        terminated(
            "<!doctype html><div id=h></div>",
            "globalThis.log = []; const h = document.getElementById('h'); const root = h.attachShadow({ mode: 'open' }); \
             const a = root.appendChild(document.createElement('slot')); a.name = 'a'; \
             const b = root.appendChild(document.createElement('slot')); b.name = 'b'; \
             a.addEventListener('slotchange', () => { log.push('a'); __terminate(); }); \
             b.addEventListener('slotchange', () => log.push('b')); \
             const x = document.createElement('i'); x.slot = 'a'; const y = document.createElement('i'); y.slot = 'b'; h.append(x, y)",
            "log"
        ),
        r#"["a"]"#
    );
}

/// turbo-dom#380: Chrome 156 compiles an `on*` content attribute's handler body lazily.
/// Its CSP `script-src-attr` hook runs twice for a report-only policy — once at
/// attribute set and once again when the handler is first compiled. An eager compile at
/// attribute-set time collapses these into one policy call, producing one CSP report
/// where Chrome produces two.
#[test]
fn inline_handler_policy_runs_at_set_and_at_first_compile() {
    crate::browser_env::tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, "<!doctype html><div id=d></div>");
    let policy = function(
        scope,
        "((doc, el, kind, code) => { if (kind === 'script-src-attr') \
         globalThis.calls = (globalThis.calls||0) + 1; return true; })",
    );
    install_inline_event_handlers(scope, policy, "https://test/");
    let code = v8::String::new(
        scope,
        "(() => { globalThis.calls = 0; globalThis.ran = 0; \
         const d = document.getElementById('d'); \
         d.setAttribute('onclick', 'globalThis.ran = (globalThis.ran||0) + 1'); \
         const afterSet = calls; \
         d.click(); const afterClick = calls; \
         d.click(); const afterSecondClick = calls; \
         return JSON.stringify({ afterSet, afterClick, afterSecondClick, ran: globalThis.ran }); \
         })()",
    )
    .unwrap();
    let value = v8::Script::compile(scope, code, None)
        .and_then(|script| script.run(scope))
        .unwrap()
        .to_rust_string_lossy(scope);
    assert_eq!(
        value,
        r#"{"afterSet":1,"afterClick":2,"afterSecondClick":2,"ran":2}"#
    );
    reset();
}

/// turbo-dom R1 (expert H3): a JS-assigned `el.on<type> = fn` wins over the
/// earlier `setAttribute('on<type>', '…')` source, even after a sequence of
/// property writes that bypass the setter's first-assignment `handlerAdd`
/// path. The invariant: once `note()` has recorded the JS-side assignment,
/// a subsequent refresh on dispatch must NOT recompile the content-attribute
/// body and clobber the JS-assigned function.
///
/// The current setter path reaches `refresh` through `handlerAdd` on the
/// first non-null JS assignment (which inserts `compiled` and short-circuits
/// later refreshes). This test covers the full contract — JS wins after a
/// `setAttribute` seed AND the attribute's source survives on the element —
/// plus the `fn1 -> fn2` reassignment path where `handlerAdd` is NOT called
/// again. A `note()` that marks `compiled` protects both.
#[test]
fn js_assignment_after_set_attribute_wins_over_content_attribute_body() {
    crate::browser_env::tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, "<!doctype html><div id=d></div>");
    let code = v8::String::new(
        scope,
        "(() => { globalThis.seen = []; \
         const d = document.getElementById('d'); \
         d.setAttribute('onclick', 'seen.push(\"A\")'); \
         d.onclick = () => seen.push('B1'); \
         /* fn1 -> fn2: setter's handlerAdd is skipped (entry.value is non-null). */ \
         d.onclick = () => seen.push('B2'); \
         d.dispatchEvent(new Event('click')); \
         /* The content-attribute source is still on the element. */ \
         const attr = d.getAttribute('onclick'); \
         return JSON.stringify({ seen, attr }); \
         })()",
    )
    .unwrap();
    let value = v8::Script::compile(scope, code, None)
        .and_then(|script| script.run(scope))
        .unwrap()
        .to_rust_string_lossy(scope);
    assert_eq!(
        value, r#"{"seen":["B2"],"attr":"seen.push(\"A\")"}"#,
        "JS assignment fires (not the content-attribute body), attribute survives",
    );
    reset();
}

/// turbo-dom#164: a keyframe selector outside 0%..100% drops its keyframe rule (Chrome 156).
#[test]
fn out_of_range_keyframe_selectors_drop_the_rule() {
    assert_eq!(
        page_json(
            "<!doctype html>",
            "['-1%', '101%', '-0%', '100%', '0%, 101%', '100.0001%', '1e2%', 'to', 'FROM, TO', 'top'].map(key => { const sheet = new CSSStyleSheet(); sheet.replaceSync(`@keyframes k { ${key} {} }`); return sheet.cssRules[0].cssRules.length; })",
        ),
        "[0,0,1,1,0,0,1,1,1,0]"
    );
}

/// Re-recording an unchanged parser handler preserves its compiled slot and does
/// not report the set-phase policy again (for example after host reconciliation).
#[test]
fn inline_handler_repeated_source_preserves_compiled_function_and_policy_reports() {
    crate::browser_env::tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, "<!doctype html><div id=d onclick='return 7'></div>");
    let policy = function(
        scope,
        "((doc, el, kind) => { if (kind === 'script-src-attr') globalThis.calls++; return true; })",
    );
    let initialize = v8::String::new(scope, "globalThis.calls = 0").unwrap();
    v8::Script::compile(scope, initialize, None)
        .unwrap()
        .run(scope)
        .unwrap();
    install_inline_event_handlers(scope, policy, "https://test/");
    let initialize = v8::String::new(
        scope,
        "globalThis.first = document.getElementById('d').onclick",
    )
    .unwrap();
    v8::Script::compile(scope, initialize, None)
        .unwrap()
        .run(scope)
        .unwrap();
    let node = with_tree(|tree| tree.query_selector_all("#d")[0]).unwrap();
    event_handlers::record_set_phase(scope, node, "onclick");
    let check = v8::String::new(
        scope,
        "JSON.stringify([document.getElementById('d').onclick === first, calls, first()])",
    )
    .unwrap();
    let value = v8::Script::compile(scope, check, None)
        .unwrap()
        .run(scope)
        .unwrap();
    assert_eq!(value.to_rust_string_lossy(scope), "[true,2,7]");
    reset();
}

/// The host URL drives form action resolution even without an action attribute;
/// clearing it restores about:blank instead of retaining a stale host base.
#[test]
fn inline_handler_host_base_is_used_by_default_form_actions() {
    crate::browser_env::tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, "<!doctype html><form id=f></form>");
    let policy = function(scope, "(() => true)");
    for (base, expected) in [
        (
            "https://example.test/path/page",
            "https://example.test/path/page",
        ),
        ("", "about:blank"),
    ] {
        install_inline_event_handlers(scope, policy, base);
        let code = v8::String::new(scope, "document.getElementById('f').action").unwrap();
        let value = v8::Script::compile(scope, code, None)
            .unwrap()
            .run(scope)
            .unwrap();
        assert_eq!(value.to_rust_string_lossy(scope), expected);
    }
    reset();
}
