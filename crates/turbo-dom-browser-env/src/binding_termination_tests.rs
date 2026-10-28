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

/// An install that runs while the isolate is terminating (an embedder's watchdog fired just
/// before a frame was set up) leaves no projected document and does not panic; once the
/// termination is cancelled, installing again yields a working document.
#[test]
fn install_while_terminating_unwinds() {
    crate::browser_env::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    {
        v8::tc_scope!(let scope, scope);
        scope.terminate_execution();
        install_with_html(scope, "<!doctype html><p id=p>x</p>");
        scope.cancel_terminate_execution();
        scope.reset();
    }
    reset();
    install_with_html(scope, "<!doctype html><p id=p>x</p>");
    let code = v8::String::new(scope, "document.getElementById('p').textContent").unwrap();
    let value = v8::Script::compile(scope, code, None)
        .unwrap()
        .run(scope)
        .unwrap()
        .to_rust_string_lossy(scope);
    assert_eq!(value, "x");
    reset();
}

/// A realm whose binding namespace was frozen before install (an embedder or an earlier
/// script defined `globalThis[Symbol.for('ts')]`) cannot receive the node projector: the
/// install still completes without panicking and the document is reachable.
#[test]
fn install_into_a_frozen_namespace_degrades_without_panicking() {
    crate::browser_env::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    let code = v8::String::new(
        scope,
        "Object.defineProperty(globalThis, Symbol.for('ts'), { value: Object.freeze({}), configurable: false, writable: false })",
    )
    .unwrap();
    v8::Script::compile(scope, code, None)
        .unwrap()
        .run(scope)
        .unwrap();
    {
        v8::tc_scope!(let scope, scope);
        install_with_html(scope, "<!doctype html><p id=p>x</p>");
        scope.reset();
    }
    let code = v8::String::new(
        scope,
        "[typeof globalThis[Symbol.for('ts')].projectNativeNode, typeof document].join()",
    )
    .unwrap();
    let value = v8::Script::compile(scope, code, None)
        .unwrap()
        .run(scope)
        .unwrap()
        .to_rust_string_lossy(scope);
    assert_eq!(value, "undefined,object");
    reset();
}

/// Arm the test-only watchdog at `checkpoint`, run `source` (which must end by termination),
/// cancel the termination and return `after`'s JSON.
fn at_checkpoint(checkpoint: &'static str, nested: bool, source: &str, after: &str) -> String {
    crate::browser_env::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, "<!doctype html><p id=p class=c>x</p>");
    {
        v8::tc_scope!(let scope, scope);
        TERMINATE_AT.with(|at| at.set(Some((checkpoint, nested))));
        let code = v8::String::new(scope, source).unwrap();
        let result = v8::Script::compile(scope, code, None).unwrap().run(scope);
        let armed = TERMINATE_AT.with(|at| at.replace(None));
        assert!(armed.is_none(), "{checkpoint} was never reached");
        assert!(result.is_none(), "{checkpoint}: {source} completed");
        assert!(scope.has_terminated(), "{checkpoint}: no termination");
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
fn watchdog_checkpoints_unwind_and_leave_a_working_document() {
    let wrap = "globalThis.made = document.createElement('aside'); globalThis.done = true";
    let wrap_after = "[typeof made, typeof done, document.createElement('aside').localName, document.getElementById('p') === document.getElementById('p')]";
    let method = "const p = document.getElementById('p'); p.setAttribute('data-a', '1'); globalThis.done = true";
    let method_after = "[typeof done, document.getElementById('p').getAttribute('data-a'), document.getElementById('p').hasAttribute('class')]";
    let setter = "const p = document.getElementById('p'); p.id = 'q'; globalThis.done = true";
    let setter_after = "[typeof done, document.querySelector('p').id]";
    let mut rows = Vec::new();
    for (checkpoint, nested, source, after) in [
        ("wrap:instance", false, wrap, wrap_after),
        ("wrap:cache", false, wrap, wrap_after),
        ("wrap:project", false, wrap, wrap_after),
        ("method:pinned", false, method, method_after),
        ("method:pinned", true, method, method_after),
        ("method:called", false, method, method_after),
        ("method:called", true, method, method_after),
        ("method:flushed", false, method, method_after),
        ("method:reconciled", false, method, method_after),
        ("setter:pinned", false, setter, setter_after),
        ("setter:pinned", true, setter, setter_after),
        ("setter:called", false, setter, setter_after),
        ("setter:called", true, setter, setter_after),
        ("setter:flushed", false, setter, setter_after),
        ("setter:reconciled", false, setter, setter_after),
    ] {
        rows.push(format!(
            "{checkpoint}/{nested}: {}",
            at_checkpoint(checkpoint, nested, source, after)
        ));
    }
    // The script never finishes; a setter terminated after it ran keeps its change (the
    // method rows terminate in the first native method the setAttribute path calls, before the
    // attribute is set); wrapper identity and fresh wrapping keep working.
    let expected = [
        r#"wrap:instance/false: ["undefined","undefined","aside",true]"#,
        r#"wrap:cache/false: ["undefined","undefined","aside",true]"#,
        r#"wrap:project/false: ["undefined","undefined","aside",true]"#,
        r#"method:pinned/false: ["undefined",null,true]"#,
        r#"method:pinned/true: ["undefined",null,true]"#,
        r#"method:called/false: ["undefined",null,true]"#,
        r#"method:called/true: ["undefined",null,true]"#,
        r#"method:flushed/false: ["undefined",null,true]"#,
        r#"method:reconciled/false: ["undefined",null,true]"#,
        r#"setter:pinned/false: ["undefined","p"]"#,
        r#"setter:pinned/true: ["undefined","p"]"#,
        r#"setter:called/false: ["undefined","q"]"#,
        r#"setter:called/true: ["undefined","q"]"#,
        r#"setter:flushed/false: ["undefined","q"]"#,
        r#"setter:reconciled/false: ["undefined","q"]"#,
    ];
    assert_eq!(rows, expected);
}

/// Listener records hold their callback and AbortSignal weakly. When the collector has taken
/// a callback, dispatch skips that listener; when a signal was aborted without its abort
/// hook running (the hook's realm was collected), dispatch drops the listener on sight.
#[test]
fn dispatch_skips_collected_callbacks_and_drops_listeners_of_aborted_signals() {
    crate::browser_env::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, "<!doctype html><p id=p>x</p>");
    let run = |scope: &mut v8::PinScope, source: &str| -> String {
        let code = v8::String::new(scope, source).unwrap();
        v8::Script::compile(scope, code, None)
            .unwrap()
            .run(scope)
            .unwrap()
            .to_rust_string_lossy(scope)
    };
    run(
        scope,
        "globalThis.log = []; globalThis.p = document.getElementById('p'); p.addEventListener('gone', () => log.push('gone')); p.addEventListener('gone', () => log.push('kept')); p.addEventListener('stale', () => log.push('stale')); globalThis.aborted = new AbortController(); aborted.abort();",
    );
    let signal = {
        let code = v8::String::new(scope, "aborted.signal").unwrap();
        let value = v8::Script::compile(scope, code, None)
            .unwrap()
            .run(scope)
            .unwrap();
        v8::Local::<v8::Object>::try_from(value).unwrap()
    };
    let signal = v8::Weak::new(scope, signal);
    LISTENERS.with(|listeners| {
        for ((_, kind), list) in listeners.borrow_mut().iter_mut() {
            match kind.as_str() {
                // The collector took the first 'gone' callback.
                "gone" => list[0].callback = v8::Weak::empty(scope),
                "stale" => list[0].signal = Some(signal.clone()),
                _ => {}
            }
        }
    });
    assert_eq!(
        run(
            scope,
            "p.dispatchEvent(new Event('gone')); p.dispatchEvent(new Event('stale')); p.dispatchEvent(new Event('stale')); JSON.stringify(log)"
        ),
        r#"["kept"]"#
    );
    let stale = LISTENERS.with(|listeners| {
        listeners
            .borrow()
            .iter()
            .filter(|((_, kind), _)| kind == "stale")
            .map(|(_, list)| {
                list.iter()
                    .filter(|listener| !listener.removed.get())
                    .count()
            })
            .sum::<usize>()
    });
    assert_eq!(stale, 0);
    reset();
}
