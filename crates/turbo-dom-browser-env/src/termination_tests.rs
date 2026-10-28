//! Native walks under a terminating isolate (crate-owned). When page script is being
//! terminated (an embedder's watchdog calls `TerminateExecution`), every native traversal the
//! host or a callback can enter must stop without touching the tree: no projection for the
//! embedder, no template realization, no normalization, no slot events. Each operation runs
//! once while terminating and once after the termination is cancelled, which proves the
//! guard (and not the input) made the difference. The host CSS policy's cascade view, which
//! only an embedder reads, is checked here too.
use crate::browser_env::*;

fn with_page(html: &str, body: impl FnOnce(&mut v8::PinScope)) {
    crate::browser_env::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, html);
    // A pending termination stays observable while a TryCatch is on the stack.
    v8::tc_scope!(let scope, scope);
    body(scope);
    reset();
}

fn eval<'s>(scope: &mut v8::PinScope<'s, '_>, source: &str) -> v8::Local<'s, v8::Value> {
    let code = v8::String::new(scope, source).unwrap();
    v8::Script::compile(scope, code, None)
        .unwrap()
        .run(scope)
        .unwrap()
}

fn json(scope: &mut v8::PinScope, source: &str) -> String {
    let value = eval(scope, &format!("JSON.stringify({source})"));
    value.to_rust_string_lossy(scope)
}

fn object<'s>(scope: &mut v8::PinScope<'s, '_>, source: &str) -> v8::Local<'s, v8::Object> {
    v8::Local::<v8::Object>::try_from(eval(scope, source)).unwrap()
}

/// Make the isolate's termination pending (as a watchdog's `TerminateExecution` does).
fn terminate(scope: &mut v8::PinScope) {
    let code = v8::String::new(scope, "0").unwrap();
    let script = v8::Script::compile(scope, code, None).unwrap();
    scope.terminate_execution();
    assert!(script.run(scope).is_none());
    assert!(scope.is_execution_terminating());
}

fn resume(scope: &mut v8::PinScope) {
    scope.cancel_terminate_execution();
    assert!(!scope.is_execution_terminating());
}

#[test]
fn slot_projection_and_slotchange_stop_while_terminating() {
    with_page("<div id=h><b id=c></b></div>", |scope| {
        let host = object(
            scope,
            "globalThis.h = document.getElementById('h'); globalThis.events = 0; \
             const root = h.attachShadow({mode: 'open'}); globalThis.s = root.appendChild(document.createElement('slot')); \
             s.addEventListener('slotchange', () => events++); h",
        );
        let child = object(scope, "document.getElementById('c')");
        let slot = object(scope, "s");
        scope.perform_microtask_checkpoint();
        // The parsed child's assignment already fired one slotchange.
        assert_eq!(json(scope, "events"), "1");
        eval(
            scope,
            "globalThis.seen = 0; s.addEventListener('ping', () => seen++)",
        );
        terminate(scope);
        shadow_slots::refresh(scope);
        shadow_slots::fire(scope, slot);
        assert!(shadow_slots::composed_children(scope, host).is_none());
        assert!(shadow_slots::composed_parent(scope, child).is_none());
        resume(scope);
        assert_eq!(json(scope, "[events, seen]"), "[1,0]");
        let ping = object(scope, "new Event('ping', {bubbles: true})");
        dispatch_host_event(scope, ping, child);
        assert_eq!(json(scope, "seen"), "1");
        // Cancelled: the same calls project the slot and fire the event.
        let children = shadow_slots::composed_children(scope, host).unwrap();
        assert_eq!(children.len(), 1);
        assert!(children[0].strict_equals(slot.into()));
        let parent = shadow_slots::composed_parent(scope, child).unwrap();
        assert!(parent.strict_equals(slot.into()));
        shadow_slots::fire(scope, slot);
        assert_eq!(json(scope, "events"), "2");
    });
}

#[test]
fn template_walks_stop_while_terminating() {
    with_page(
        "<template id=t><b>x</b><template><i>y</i></template></template><div id=d><p>z</p></div>",
        |scope| {
            let template = object(scope, "globalThis.t = document.getElementById('t')");
            let div = object(scope, "document.getElementById('d')");
            let document = object(scope, "document");
            let copy = object(scope, "document.createElement('template')");
            // Each walk is entered with the termination pending and leaves it pending.
            macro_rules! stopped {
                ($walk:expr) => {{
                    terminate(scope);
                    let result = $walk;
                    assert!(scope.is_execution_terminating());
                    resume(scope);
                    result
                }};
            }
            stopped!(template_contents::realize(scope, document));
            stopped!(template_contents::mark_parsed(scope, document, true));
            stopped!(template_contents::relocate(scope, div));
            stopped!(template_contents::copy_contents(
                scope, template, copy, true
            ));
            assert!(stopped!(
                template_contents::ensure(scope, template).is_none()
            ));
            assert!(stopped!(template_contents::parser_policy_nodes(
                scope, document
            )
            .is_empty()));
            let serialized = template_contents::serialize(scope, div, true);
            assert_eq!(
                String::from_utf16(&serialized).unwrap(),
                "<div id=\"d\"><p>z</p></div>"
            );
            assert!(template_contents::ensure(scope, template).is_some());
            assert_eq!(
                json(scope, "[t.content.childNodes.length, t.innerHTML]"),
                r#"[2,"<b>x</b><template><i>y</i></template>"]"#
            );
        },
    );
}

#[test]
fn normalize_and_listener_records_stop_while_terminating() {
    with_page("<p id=p></p>", |scope| {
        eval(
            scope,
            "globalThis.p = document.getElementById('p'); p.append('a', 'b', document.createElement('i'), 'c', ''); \
             p.lastElementChild.append('d', 'e')",
        );
        let paragraph = object(scope, "p");
        let handle = with_tree(|tree| tree.query_selector("#p"))
            .flatten()
            .unwrap();
        let target = object(scope, "new EventTarget()");
        terminate(scope);
        range_live::normalize(scope, handle);
        assert!(!event_brands::register_native_event_target(scope, target));
        assert!(!event_brands::native_event_target_has_listener(
            scope, paragraph, "click"
        ));
        resume(scope);
        assert_eq!(json(scope, "p.childNodes.length"), "5");
        range_live::normalize(scope, handle);
        assert_eq!(
            json(
                scope,
                "[p.childNodes.length, p.firstChild.data, p.lastElementChild.childNodes.length]"
            ),
            r#"[3,"ab",1]"#
        );
        assert!(event_brands::register_native_event_target(scope, target));
    });
}

/// The host CSS policy's cascade view (turbo-surf's native layout): with a callback that
/// blocks one `<style>` and every inline style, the policy lists the blocked sheet and the
/// inline override (the applied text: empty for the blocked declaration), and is cached
/// until the tree version changes.
#[test]
fn host_css_policy_lists_blocked_sheets_and_inline_overrides() {
    with_page(
        "<style id=ok>p{color:red}</style><style id=no>p{color:blue}</style><p id=a style='opacity:0.5'>x</p><p id=b>y</p>",
        |scope| {
            let callback = v8::Local::<v8::Function>::try_from(eval(
                scope,
                "globalThis.calls = []; (document, element, kind, text) => { calls.push([kind, element.id, String(text)]); return kind === 'style-src-attr' ? false : element.id !== 'no'; }",
            ))
            .unwrap();
            install_css_policy(scope, callback);
            let handle = |id: &str| {
                with_tree(|tree| tree.query_selector(&format!("#{id}")))
                    .flatten()
                    .unwrap()
            };
            let (no, a, b) = (handle("no"), handle("a"), handle("b"));
            let policy = css_policy::cascade_policy(scope);
            assert_eq!(policy.blocked_stylesheets.iter().copied().collect::<Vec<_>>(), [no]);
            assert_eq!(policy.inline_overrides.len(), 1);
            assert_eq!(policy.inline_overrides.get(&a).map(String::as_str), Some(""));
            assert_eq!(json(scope, "document.getElementById('a').style.opacity"), r#""""#);
            let calls = json(scope, "calls.length");
            // Cached per tree version: a second read asks the callback nothing.
            let again = css_policy::cascade_policy(scope);
            assert_eq!(again.inline_overrides, policy.inline_overrides);
            assert_eq!(json(scope, "calls.length"), calls);
            // A new version recomputes: a second blocked inline style joins the overrides.
            eval(scope, "document.getElementById('b').setAttribute('style', 'color: green')");
            let next = css_policy::cascade_policy(scope);
            assert_eq!(next.inline_overrides.get(&b).map(String::as_str), Some(""));
            assert_eq!(next.inline_overrides.len(), 2);
        },
    );
}

/// The embedder's mutation consumer registers only on its own document node with a callback
/// of that document's realm; anything else is refused and receives nothing.
#[test]
fn mutation_consumer_registration_rejects_foreign_receivers_and_callbacks() {
    with_page("<p id=p></p>", |scope| {
        let document = object(scope, "document");
        let callback = v8::Local::<v8::Function>::try_from(eval(
            scope,
            "globalThis.records = 0; () => records++",
        ))
        .unwrap();
        let plain = object(scope, "({})");
        let element = object(scope, "document.getElementById('p')");
        assert!(!install_mutation_consumer(scope, plain, callback));
        assert!(!install_mutation_consumer(scope, element, callback));
        let other = v8::Context::new(scope, Default::default());
        let foreign = {
            let scope = &mut v8::ContextScope::new(scope, other);
            v8::Local::<v8::Function>::try_from(eval(scope, "() => 0")).unwrap()
        };
        assert!(!install_mutation_consumer(scope, document, foreign));
        eval(scope, "document.getElementById('p').append('x')");
        scope.perform_microtask_checkpoint();
        assert_eq!(json(scope, "records"), "0");
        assert!(install_mutation_consumer(scope, document, callback));
        eval(scope, "document.getElementById('p').append('y')");
        scope.perform_microtask_checkpoint();
        assert_eq!(json(scope, "records"), "1");
    });
}
