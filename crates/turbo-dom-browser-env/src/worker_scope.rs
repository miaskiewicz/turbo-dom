//! DedicatedWorkerGlobalScope install (turbo-dom#385, for turbo-surf #203).
//!
//! The ECMAScript surface of a Chrome-style dedicated worker:
//! - `self` -> `globalThis`;
//! - `importScripts(...urls)` (synchronous script loader hook — surf wraps);
//! - `structuredClone(value)` (a JSON round-trip stub; surf wires its real
//!   structured-clone machinery through the message-port path);
//! - `postMessage(message, transfer?)` and `close()` (embedder hooks);
//! - `addEventListener` / `removeEventListener` / `dispatchEvent` for the three
//!   worker-scope event kinds (`message`, `messageerror`, `error`);
//! - event-handler slots `onmessage` / `onmessageerror` / `onerror`;
//! - the `DedicatedWorkerGlobalScope` -> `WorkerGlobalScope` -> `EventTarget`
//!   interface brands stamped onto `globalThis`.
//!
//! The install is a short JS bootstrap because the page it runs on has no DOM
//! template (`mark_worker_context` skipped it) and no heavy engine state —
//! keeping the surface in JS makes the three hook wire-up points (`postMessage`,
//! `close`, `importScripts`) small and auditable. The Rust side only exposes
//! helpers that reach into the bootstrap's `__turboDomWorker` object to register
//! embedder hooks and deliver incoming messages.

const WORKER_BOOTSTRAP: &str = include_str!("worker_scope.js");

/// Install the worker surface onto the current context's `globalThis`.
pub(super) fn install(scope: &mut v8::PinScope) {
    let code = v8::String::new(scope, WORKER_BOOTSTRAP).unwrap();
    if let Some(script) = v8::Script::compile(scope, code, None) {
        let _ = script.run(scope);
    }
}

/// Install `hook` as the context's `__turboDomWorker.setXxx(f)`.
fn set_hook(
    scope: &mut v8::PinScope,
    context: v8::Local<v8::Context>,
    setter: &str,
    hook: Option<v8::Local<v8::Function>>,
) {
    let scope = &mut v8::ContextScope::new(scope, context);
    let global = context.global(scope);
    let ns_key = v8::String::new(scope, "__turboDomWorker").unwrap();
    let Some(ns_value) = global.get(scope, ns_key.into()) else {
        return;
    };
    let Ok(ns) = v8::Local::<v8::Object>::try_from(ns_value) else {
        return;
    };
    let setter_key = v8::String::new(scope, setter).unwrap();
    let Some(setter_value) = ns.get(scope, setter_key.into()) else {
        return;
    };
    let Ok(setter_fn) = v8::Local::<v8::Function>::try_from(setter_value) else {
        return;
    };
    let arg: v8::Local<v8::Value> = match hook {
        Some(f) => f.into(),
        None => v8::null(scope).into(),
    };
    let recv: v8::Local<v8::Value> = ns.into();
    let _ = setter_fn.call(scope, recv, &[arg]);
}

pub(super) fn set_post_message_hook(
    scope: &mut v8::PinScope,
    context: v8::Local<v8::Context>,
    hook: Option<v8::Local<v8::Function>>,
) {
    set_hook(scope, context, "setPostMessage", hook);
}

pub(super) fn set_close_hook(
    scope: &mut v8::PinScope,
    context: v8::Local<v8::Context>,
    hook: Option<v8::Local<v8::Function>>,
) {
    set_hook(scope, context, "setClose", hook);
}

pub(super) fn set_import_scripts_hook(
    scope: &mut v8::PinScope,
    context: v8::Local<v8::Context>,
    hook: Option<v8::Local<v8::Function>>,
) {
    set_hook(scope, context, "setImportScripts", hook);
}

/// Fire a message event: build `{ type: "message", data }` and call the context's
/// `__turboDomWorker.dispatch("message", event)`. Returns `true` when the
/// bootstrap reported a handler or listener fired.
pub(super) fn dispatch_message<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    data: v8::Local<'s, v8::Value>,
) -> bool {
    let context = scope.get_current_context();
    let global = context.global(scope);
    let ns_key = v8::String::new(scope, "__turboDomWorker").unwrap();
    let Some(ns_value) = global.get(scope, ns_key.into()) else {
        return false;
    };
    let Ok(ns) = v8::Local::<v8::Object>::try_from(ns_value) else {
        return false;
    };
    let dispatch_key = v8::String::new(scope, "dispatch").unwrap();
    let Some(dispatch_value) = ns.get(scope, dispatch_key.into()) else {
        return false;
    };
    let Ok(dispatch_fn) = v8::Local::<v8::Function>::try_from(dispatch_value) else {
        return false;
    };
    let event = v8::Object::new(scope);
    let type_key = v8::String::new(scope, "type").unwrap();
    let message = v8::String::new(scope, "message").unwrap();
    event.set(scope, type_key.into(), message.into());
    let data_key = v8::String::new(scope, "data").unwrap();
    event.set(scope, data_key.into(), data);
    let kind_arg: v8::Local<v8::Value> = message.into();
    let event_arg: v8::Local<v8::Value> = event.into();
    let recv: v8::Local<v8::Value> = ns.into();
    match dispatch_fn.call(scope, recv, &[kind_arg, event_arg]) {
        Some(ret) => ret.boolean_value(scope),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser_env;

    fn eval(scope: &mut v8::PinScope, src: &str) -> String {
        let code = v8::String::new(scope, src).unwrap();
        v8::Script::compile(scope, code, None)
            .unwrap()
            .run(scope)
            .unwrap()
            .to_rust_string_lossy(scope)
    }

    #[test]
    fn installs_dedicated_worker_global_scope_surface() {
        browser_env::tests::init_v8();
        let isolate = &mut browser_env::tests::plain_isolate();
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        assert_eq!(
            browser_env::install_worker_global_scope(scope, context),
            Ok(())
        );
        let scope = &mut v8::ContextScope::new(scope, context);
        assert_eq!(eval(scope, "typeof self"), "object");
        assert_eq!(eval(scope, "self === globalThis"), "true");
        assert_eq!(eval(scope, "typeof postMessage"), "function");
        assert_eq!(eval(scope, "typeof close"), "function");
        assert_eq!(eval(scope, "typeof importScripts"), "function");
        assert_eq!(eval(scope, "typeof structuredClone"), "function");
        assert_eq!(eval(scope, "typeof addEventListener"), "function");
        assert_eq!(eval(scope, "typeof removeEventListener"), "function");
        assert_eq!(eval(scope, "typeof dispatchEvent"), "function");
        assert_eq!(eval(scope, "typeof EventTarget"), "function");
        assert_eq!(eval(scope, "typeof WorkerGlobalScope"), "function");
        assert_eq!(eval(scope, "typeof DedicatedWorkerGlobalScope"), "function");
        assert_eq!(
            eval(
                scope,
                "Object.getPrototypeOf(WorkerGlobalScope.prototype) === EventTarget.prototype"
            ),
            "true"
        );
        assert_eq!(
            eval(
                scope,
                "Object.getPrototypeOf(DedicatedWorkerGlobalScope.prototype) === WorkerGlobalScope.prototype"
            ),
            "true"
        );
        assert_eq!(
            eval(
                scope,
                "DedicatedWorkerGlobalScope.prototype.PERSISTENT === 1 && \
                 DedicatedWorkerGlobalScope.prototype.TEMPORARY === 0"
            ),
            "true"
        );
        assert_eq!(eval(scope, "onmessage"), "null");
        assert_eq!(
            eval(
                scope,
                "let fn = () => 1; onmessage = fn; onmessage === fn ? 'ok' : 'no'"
            ),
            "ok"
        );
        assert_eq!(eval(scope, "onmessage = 'x'; onmessage"), "null");
        assert_eq!(
            eval(
                scope,
                "let o = { a: 1, b: [2, 3] }; let c = structuredClone(o); \
                 c !== o && c.a === 1 && c.b[1] === 3 ? 'ok' : 'no'"
            ),
            "ok"
        );
        assert_eq!(
            eval(
                scope,
                "try { importScripts('x.js'); 'none' } \
                 catch (e) { e instanceof TypeError ? 'typeerror' : 'other' }"
            ),
            "typeerror"
        );
        assert_eq!(
            eval(
                scope,
                "try { new DedicatedWorkerGlobalScope(); 'none' } \
                 catch (e) { e instanceof TypeError ? 'ok' : 'other' }"
            ),
            "ok"
        );
        browser_env::reset();
    }

    #[test]
    fn dispatches_messages_to_onmessage_and_listeners() {
        browser_env::tests::init_v8();
        let isolate = &mut browser_env::tests::plain_isolate();
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        browser_env::install_worker_global_scope(scope, context).unwrap();
        let scope = &mut v8::ContextScope::new(scope, context);
        eval(
            scope,
            "globalThis.__log = []; \
             onmessage = (e) => __log.push(['on', e.type, e.data]); \
             addEventListener('message', (e) => __log.push(['listen', e.type, e.data]));",
        );
        let data = v8::String::new(scope, "hello").unwrap();
        let delivered = browser_env::dispatch_worker_message(scope, data.into());
        assert!(delivered);
        assert_eq!(
            eval(scope, "JSON.stringify(__log)"),
            r#"[["on","message","hello"],["listen","message","hello"]]"#
        );
        browser_env::reset();
    }

    #[test]
    fn interface_parity_against_chrome_156() {
        // Shape check: the install stamps the same three-brand chain that Chrome
        // exposes (`DedicatedWorkerGlobalScope -> WorkerGlobalScope ->
        // EventTarget -> Object`) and installs the hook names recorded in the
        // control fixture.
        let fixture = include_str!("dedicated_worker_global_scope_chrome156.json");
        assert!(fixture.contains("\"DedicatedWorkerGlobalScope\""));
        assert!(fixture.contains("\"WorkerGlobalScope\""));
        assert!(fixture.contains("\"EventTarget\""));
        for name in [
            "importScripts",
            "structuredClone",
            "postMessage",
            "PERSISTENT",
            "TEMPORARY",
            "addEventListener",
            "removeEventListener",
            "dispatchEvent",
            "close",
            "onmessage",
            "onmessageerror",
            "onerror",
        ] {
            assert!(
                fixture.contains(&format!("\"{name}\"")),
                "control fixture missing {name}"
            );
        }
        browser_env::tests::init_v8();
        let isolate = &mut browser_env::tests::plain_isolate();
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        browser_env::install_worker_global_scope(scope, context).unwrap();
        let scope = &mut v8::ContextScope::new(scope, context);
        for name in [
            "self",
            "postMessage",
            "close",
            "importScripts",
            "structuredClone",
            "addEventListener",
            "removeEventListener",
            "dispatchEvent",
            "onmessage",
            "onmessageerror",
            "onerror",
        ] {
            assert_ne!(
                eval(scope, &format!("typeof {name}")),
                "undefined",
                "{name} must be installed by install_worker_global_scope"
            );
        }
        browser_env::reset();
    }

    /// A Rust callback the hook tests install. Flips a thread-local flag
    /// so the test can detect the hook firing without crossing V8 scopes.
    fn note_hook_called(
        _: &mut v8::PinScope,
        _: v8::FunctionCallbackArguments,
        mut result: v8::ReturnValue,
    ) {
        HOOK_FIRED.with(|f| f.set(true));
        result.set_null();
    }

    thread_local! {
        static HOOK_FIRED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }

    /// Install each hook, call its worker-side JS entry, verify the Rust
    /// callback fired. Then clear the hook (pass `None` — the `None` arm
    /// of `set_hook`'s `match`) and verify the entry no longer calls back.
    #[test]
    fn set_hook_registers_and_clears_each_embedder_hook() {
        browser_env::tests::init_v8();
        let isolate = &mut browser_env::tests::plain_isolate();
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        browser_env::install_worker_global_scope(scope, context).unwrap();
        let scope = &mut v8::ContextScope::new(scope, context);
        let hook = v8::Function::new(scope, ext!(FunctionCallback, note_hook_called)).unwrap();
        // `(setter, trigger)` — trigger is the JS that invokes the worker-scope
        // API the hook backs. `importScripts` throws by default, so the
        // trigger wraps it.
        type Setter = fn(
            &mut v8::PinScope,
            v8::Local<v8::Context>,
            Option<v8::Local<v8::Function>>,
        );
        let cases: &[(Setter, &str)] = &[
            (
                browser_env::set_worker_post_message_hook,
                "postMessage('x');",
            ),
            (browser_env::set_worker_close_hook, "close();"),
            (
                browser_env::set_worker_import_scripts_hook,
                "try { importScripts('a.js'); } catch (_) {}",
            ),
        ];
        for (setter, trigger) in cases {
            HOOK_FIRED.with(|f| f.set(false));
            setter(scope, context, Some(hook));
            let code = v8::String::new(scope, trigger).unwrap();
            if let Some(script) = v8::Script::compile(scope, code, None) {
                script.run(scope);
            }
            assert!(
                HOOK_FIRED.with(std::cell::Cell::get),
                "hook must fire ({trigger})",
            );
            // Clear the hook (None arm of set_hook's Some(f) / None match):
            // the trigger must not call back again.
            HOOK_FIRED.with(|f| f.set(false));
            setter(scope, context, None);
            let code = v8::String::new(scope, trigger).unwrap();
            if let Some(script) = v8::Script::compile(scope, code, None) {
                script.run(scope);
            }
            assert!(
                !HOOK_FIRED.with(std::cell::Cell::get),
                "cleared hook must not fire ({trigger})",
            );
        }
        browser_env::reset();
    }

    /// A context without the worker bootstrap has no `__turboDomWorker`
    /// namespace: `global.get` returns `undefined`, so
    /// `v8::Local::<v8::Object>::try_from(undefined)` fails in both
    /// `set_hook` and `dispatch_message` and each returns early. One bare
    /// context exercises every helper's early-return path.
    #[test]
    fn worker_helpers_are_a_no_op_without_the_bootstrap() {
        browser_env::tests::init_v8();
        let isolate = &mut browser_env::tests::plain_isolate();
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        // No install_worker_global_scope — this is a bare context.
        let cs = &mut v8::ContextScope::new(scope, context);
        let hook = v8::Function::new(cs, ext!(FunctionCallback, note_hook_called)).unwrap();
        browser_env::set_worker_post_message_hook(cs, context, Some(hook));
        browser_env::set_worker_close_hook(cs, context, Some(hook));
        browser_env::set_worker_import_scripts_hook(cs, context, Some(hook));
        // And dispatch_message returns `false` without the bootstrap (no
        // `dispatch` function to call).
        let data = v8::String::new(cs, "noop").unwrap();
        let delivered = browser_env::dispatch_worker_message(cs, data.into());
        assert!(
            !delivered,
            "dispatch_worker_message must return false without the bootstrap",
        );
        browser_env::reset();
    }
}
