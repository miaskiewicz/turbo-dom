//! Execution termination (an embedder's watchdog calling `terminate_execution`) while the
//! binding is inside a DOM call: the call unwinds without running further guest code, the
//! DOM change the call made stays made, and once the embedder cancels the termination the
//! realm keeps working. Guest code ends the script through `__terminate()`.
use crate::browser_env::*;

fn terminate(scope: &mut v8::PinScope, _args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue) {
    scope.terminate_execution();
}

thread_local! {
    static WRAPPED: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
}

/// Terminates, lets the interrupt land in a guest call, then wraps a node that has no
/// wrapper yet: wrapping while terminating yields the global object and caches nothing.
fn wrap_while_terminating(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    scope.terminate_execution();
    let callback = v8::Local::<v8::Function>::try_from(args.get(0)).unwrap();
    let receiver = v8::undefined(scope).into();
    assert!(callback.call(scope, receiver, &[]).is_none());
    assert!(scope.is_execution_terminating());
    let handle = with_tree_mut(|tree| tree.create_element("aside")).unwrap();
    let object = wrap_inner(scope, handle);
    let global = scope.get_current_context().global(scope);
    let cached = DOM.with(|dom| dom.borrow().as_ref().unwrap().cache.contains_key(&handle));
    WRAPPED.with(|wrapped| wrapped.set(Some(object == global && !cached)));
}

/// Run `source` in a fresh page; it must end by termination. Then cancel it and return
/// `after`'s JSON.
fn terminated(html: &str, source: &str, after: &str) -> String {
    crate::browser_env::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, html);
    let global = context.global(scope);
    bind_method(scope, global, "__terminate", terminate);
    bind_method(
        scope,
        global,
        "__wrapWhileTerminating",
        wrap_while_terminating,
    );
    {
        v8::tc_scope!(let scope, scope);
        let code = v8::String::new(scope, source).unwrap();
        let result = v8::Script::compile(scope, code, None).unwrap().run(scope);
        assert!(result.is_none(), "{source} completed");
        assert!(scope.has_terminated(), "{source} did not terminate");
        scope.cancel_terminate_execution();
        scope.reset();
    }
    let code = v8::String::new(scope, &format!("JSON.stringify({after})")).unwrap();
    let value = v8::Script::compile(scope, code, None)
        .unwrap()
        .run(scope)
        .unwrap()
        .to_rust_string_lossy(scope);
    reset();
    value
}

#[test]
fn wrapping_while_terminating_returns_the_global_and_caches_nothing() {
    WRAPPED.with(|wrapped| wrapped.set(None));
    assert_eq!(
        terminated(
            "<!doctype html><p id=p></p>",
            "globalThis.ran = 0; __wrapWhileTerminating(() => { ran++; }); ran = 99",
            "[ran, document.getElementById('p').id]"
        ),
        r#"[0,"p"]"#
    );
    assert_eq!(WRAPPED.with(|wrapped| wrapped.get()), Some(true));
}

/// A listener that terminates inside dispatchEvent: later listeners do not run.
#[test]
fn termination_inside_dispatch_stops_the_remaining_listeners() {
    assert_eq!(
        terminated(
            "<!doctype html><p id=p></p>",
            "globalThis.log = []; const p = document.getElementById('p'); p.addEventListener('x', () => { log.push(1); __terminate(); log.push('after'); }); p.addEventListener('x', () => log.push(2)); document.body.addEventListener('x', () => log.push(3)); p.dispatchEvent(new Event('x', { bubbles: true })); log.push('end')",
            "log"
        ),
        "[1]"
    );
}

/// A custom element reaction that terminates when a method's [CEReactions] scope ends: the
/// attribute is set, nothing after the reaction runs.
#[test]
fn termination_in_a_method_reaction_keeps_the_mutation() {
    assert_eq!(
        terminated(
            "<!doctype html>",
            "globalThis.log = []; customElements.define('x-term', class extends HTMLElement { static observedAttributes = ['a']; attributeChangedCallback(name, old, value) { log.push(value); if (value === 'stop') __terminate(); } }); globalThis.el = document.body.appendChild(document.createElement('x-term')); el.setAttribute('a', 'stop'); log.push('end')",
            "[log, el.getAttribute('a')]"
        ),
        r#"[["stop"],"stop"]"#
    );
}

/// The same for a setter's reaction (an upgrade the innerHTML setter queued).
#[test]
fn termination_in_a_setter_reaction_keeps_the_mutation() {
    assert_eq!(
        terminated(
            "<!doctype html><div id=host></div>",
            "globalThis.log = []; customElements.define('x-stop', class extends HTMLElement { connectedCallback() { log.push('connected'); __terminate(); } }); globalThis.host = document.getElementById('host'); host.innerHTML = '<x-stop></x-stop><x-stop></x-stop>'; log.push('end')",
            "[log, host.children.length]"
        ),
        r#"[["connected"],2]"#
    );
}
