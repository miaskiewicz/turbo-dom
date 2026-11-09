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
///
/// turbo-dom#435: compile with a `ScriptOrigin` naming `worker_scope.js` so a
/// stack trace from inside the bootstrap (e.g. an event-listener error) names
/// the source file and the line within it — the default `compile(scope, code,
/// None)` leaves the frame as the anonymous `<anonymous>` placeholder.
pub(super) fn install(scope: &mut v8::PinScope) {
    // turbo-dom#428: expose a native `__turboDomStructuredClone` BEFORE the JS
    // bootstrap runs so the bootstrap can capture it, delete it from the
    // global, and freeze a reference inside the namespace. This routes
    // `structuredClone` through V8's own ValueSerializer / ValueDeserializer
    // so Date / Map / Set / RegExp / BigInt / typed arrays / cycles
    // round-trip faithfully with HTML-spec semantics.
    let context = scope.get_current_context();
    let global = context.global(scope);
    super::bind_method(scope, global, "__turboDomStructuredClone", structured_clone);
    let code = v8::String::new(scope, WORKER_BOOTSTRAP).unwrap();
    let resource = v8::String::new(scope, "worker_scope.js").unwrap();
    let origin = v8::ScriptOrigin::new(
        scope,
        resource.into(),
        0,
        0,
        false,
        0,
        None,
        false,
        false,
        false,
        None,
    );
    if let Some(script) = v8::Script::compile(scope, code, Some(&origin)) {
        let _ = script.run(scope);
    }
}

/// A `ValueSerializerImpl` that rethrows cloneability failures as
/// `DataCloneError` (matching HTML spec and Chrome worker behaviour).
struct StructuredCloneSerializerDelegate;

impl v8::ValueSerializerImpl for StructuredCloneSerializerDelegate {
    fn throw_data_clone_error<'s>(
        &self,
        scope: &mut v8::PinScope<'s, '_>,
        message: v8::Local<'s, v8::String>,
    ) {
        let exc = v8::Exception::error(scope, message);
        if let Ok(obj) = v8::Local::<v8::Object>::try_from(exc) {
            // invariants: engine-built strings, well below V8's limits
            let key = v8::String::new(scope, "name").unwrap();
            let name = v8::String::new(scope, "DataCloneError").unwrap();
            obj.set(scope, key.into(), name.into());
        }
        scope.throw_exception(exc);
    }
}

/// A `ValueDeserializerImpl` with default behaviour (no host objects, no
/// transferables — structured clone of an in-process payload only).
struct StructuredCloneDeserializerDelegate;

impl v8::ValueDeserializerImpl for StructuredCloneDeserializerDelegate {}

/// `__turboDomStructuredClone(value)` — a V8-backed structured clone of
/// `value`. Throws `DataCloneError` for values the HTML spec declines
/// (functions, Symbols, WeakMap, etc.). Transferables are not supported
/// (callers that need them route through the message-port path).
fn structured_clone<'a, 'i>(
    scope: &mut v8::PinScope<'a, 'i>,
    args: v8::FunctionCallbackArguments<'a>,
    mut rv: v8::ReturnValue<'a>,
) {
    let value = args.get(0);
    let context = scope.get_current_context();
    use v8::ValueDeserializerHelper;
    use v8::ValueSerializerHelper;
    let bytes = {
        let serializer =
            v8::ValueSerializer::new(scope, Box::new(StructuredCloneSerializerDelegate));
        serializer.write_header();
        match serializer.write_value(context, value) {
            Some(true) => serializer.release(),
            // Exception (DataCloneError) or false: either way the serializer's
            // throw_data_clone_error has already scheduled the exception; just
            // return so V8 propagates it.
            _ => return,
        }
    };
    let deserializer =
        v8::ValueDeserializer::new(scope, Box::new(StructuredCloneDeserializerDelegate), &bytes);
    if deserializer.read_header(context) != Some(true) {
        return;
    }
    if let Some(cloned) = deserializer.read_value(context) {
        rv.set(cloned);
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
    // turbo-dom#438: construct the event with a null prototype so inherited
    // lookups through `Object.prototype` cannot observe or alter the native
    // event payload. `type` and `data` remain own properties of the event.
    let event = v8::Object::new(scope);
    let null_proto: v8::Local<v8::Value> = v8::null(scope).into();
    event.set_prototype(scope, null_proto);
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

    /// Review R2 V8 M1: worker bootstrap must capture its own intrinsics so a
    /// page that mutates `globalThis.Set` / `Array.prototype[@@iterator]`
    /// before the first `addEventListener` cannot intercept listener
    /// bookkeeping or the dispatch's iteration. The main-realm binding
    /// already does this (captured_intrinsics.js); the worker had the gap.
    #[test]
    fn listener_bookkeeping_uses_captured_set_through_prototype_poisoning() {
        browser_env::tests::init_v8();
        let isolate = &mut browser_env::tests::plain_isolate();
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        browser_env::install_worker_global_scope(scope, context).unwrap();
        let scope = &mut v8::ContextScope::new(scope, context);
        // Poison BOTH globalThis.Set (constructor replacement) and
        // Array.prototype[@@iterator] (spread-iteration). The first
        // addEventListener has not been called yet, so the worker scope
        // has not materialised its per-type Set. A bootstrap that
        // re-reads `Set` or does `[...list]` would now use the poisoned
        // builtins. A bootstrap that captured its intrinsics at install
        // time is untouched.
        eval(
            scope,
            "globalThis.__poisonAdded = false; \
             globalThis.Set = class { add() { globalThis.__poisonAdded = true; } has() { return false; } }; \
             Array.prototype[Symbol.iterator] = function () { throw new Error('poisoned iterator'); };",
        );
        // Register a listener and dispatch a message — both paths touch
        // Set (on first add) and the dispatch iteration ([...list]).
        eval(
            scope,
            "globalThis.__log = []; \
             addEventListener('message', (e) => __log.push(e.data));",
        );
        let data = v8::String::new(scope, "poisoned-but-delivered").unwrap();
        let delivered = browser_env::dispatch_worker_message(scope, data.into());
        // Clean the prototype poisoning so other tests on this isolate are
        // unaffected.
        eval(
            scope,
            "delete Array.prototype[Symbol.iterator]; delete globalThis.__poisonAdded;",
        );
        assert!(delivered, "dispatch must still report delivery");
        assert_eq!(
            eval(scope, "JSON.stringify(__log)"),
            r#"["poisoned-but-delivered"]"#,
            "listener must fire even with poisoned builtins",
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

    /// turbo-dom#428: `structuredClone` must round-trip the HTML spec types
    /// (Date, Map, Set, RegExp, typed arrays, cycles, BigInt) instead of
    /// rejecting them. The previous JSON-based stub threw DataCloneError on
    /// all of these; a page feature-detection then disagreed with Chrome.
    #[test]
    fn structured_clone_round_trips_html_spec_types() {
        browser_env::tests::init_v8();
        let isolate = &mut browser_env::tests::plain_isolate();
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        browser_env::install_worker_global_scope(scope, context).unwrap();
        let scope = &mut v8::ContextScope::new(scope, context);
        // A single safe-eval probe: every assertion runs inside try/catch
        // (the current JSON-stub throws DataCloneError eagerly, which would
        // otherwise abort the host-side eval helper on the first failure).
        let probe = r#"(() => {
          const rows = {};
          function safe(label, fn) { try { rows[label] = fn(); } catch (e) { rows[label] = 'threw:' + (e && e.name || String(e)); } }
          safe('date', () => {
            const d = new Date(1700000000000); const c = structuredClone(d);
            return [c instanceof Date, c !== d, c.getTime() === d.getTime()];
          });
          safe('map', () => {
            const m = new Map([['a',1],['b',2]]); const c = structuredClone(m);
            return [c instanceof Map, c !== m, c.get('a'), c.get('b'), c.size];
          });
          safe('set', () => {
            const s = new Set([1,2,3]); const c = structuredClone(s);
            return [c instanceof Set, c !== s, c.has(1), c.has(2), c.has(3), c.size];
          });
          safe('regexp', () => {
            const r = /foo/gi; const c = structuredClone(r);
            return [c instanceof RegExp, c.source === 'foo', c.flags === r.flags];
          });
          safe('bigint', () => {
            const n = 123456789012345678901234567890n; const c = structuredClone(n);
            return [typeof c === 'bigint', c === n];
          });
          safe('cycle', () => {
            const o = {}; o.self = o; const c = structuredClone(o);
            return [c !== o, c.self === c];
          });
          safe('u8', () => {
            const u = new Uint8Array([1,2,3,255]); const c = structuredClone(u);
            return [c instanceof Uint8Array, c !== u, Array.from(c).join()];
          });
          safe('fn', () => {
            let name = null;
            try { structuredClone(() => {}); }
            catch (e) { name = e && e.name; }
            return name;
          });
          return JSON.stringify(rows);
        })()"#;
        let result = eval(scope, probe);
        let expected = r#"{"date":[true,true,true],"map":[true,true,1,2,2],"set":[true,true,true,true,true,3],"regexp":[true,true,true],"bigint":[true,true],"cycle":[true,true],"u8":[true,true,"1,2,3,255"],"fn":"DataCloneError"}"#;
        assert_eq!(
            result, expected,
            "structuredClone must round-trip HTML-spec types (Date, Map, \
             Set, RegExp, BigInt, cycles, typed arrays) and still reject \
             non-cloneable values (functions) with DataCloneError"
        );
        browser_env::reset();
    }

    /// turbo-dom#438: native-side event payloads must not inherit
    /// `Object.prototype`. A worker-side script that stamps a getter on
    /// `Object.prototype.leak` would otherwise be able to observe the native
    /// event through the inherited slot. The event is constructed with a null
    /// prototype so inherited lookups return `undefined` and the event shape
    /// is exactly what the Rust side wrote (`type`, `data`).
    #[test]
    fn dispatch_message_event_has_null_prototype() {
        browser_env::tests::init_v8();
        let isolate = &mut browser_env::tests::plain_isolate();
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        browser_env::install_worker_global_scope(scope, context).unwrap();
        let scope = &mut v8::ContextScope::new(scope, context);
        // Plant a getter on `Object.prototype.leak` and record the event's
        // prototype and whether the inherited getter fires on the event.
        eval(
            scope,
            "globalThis.__leakCalls = 0; \
             Object.defineProperty(Object.prototype, 'leak', { \
               configurable: true, get() { globalThis.__leakCalls++; return 'leaked'; } \
             }); \
             globalThis.__probe = null; \
             addEventListener('message', (e) => { \
               globalThis.__probe = { \
                 protoNull: Object.getPrototypeOf(e) === null, \
                 ownType: Object.prototype.hasOwnProperty.call(e, 'type'), \
                 ownData: Object.prototype.hasOwnProperty.call(e, 'data'), \
                 leak: e.leak === undefined ? 'undef' : String(e.leak), \
               }; \
             });",
        );
        let data = v8::String::new(scope, "payload").unwrap();
        let delivered = browser_env::dispatch_worker_message(scope, data.into());
        assert!(delivered);
        assert_eq!(
            eval(scope, "JSON.stringify(__probe)"),
            r#"{"protoNull":true,"ownType":true,"ownData":true,"leak":"undef"}"#,
            "dispatch_message event must have a null prototype so \
             Object.prototype getters cannot observe or alter it",
        );
        // Belt and suspenders: the inherited getter must never have fired
        // against the event object.
        assert_eq!(
            eval(scope, "String(__leakCalls)"),
            "0",
            "no inherited Object.prototype getter may fire against the event",
        );
        // Cleanup for shared test context.
        eval(scope, "delete Object.prototype.leak;");
        browser_env::reset();
    }

    #[test]
    fn turbo_dom_worker_namespace_is_locked_against_page_tampering() {
        // Review R1 M4: page JS inside a worker must not be able to
        // overwrite the `__turboDomWorker` object (whole-slot takeover) nor
        // any of its hook methods (per-method intercept of incoming
        // messages). The namespace is installed non-writable +
        // non-configurable + non-enumerable, and the object itself is
        // frozen. Chrome's real DedicatedWorkerGlobalScope has no such
        // namespace — this is extra attack surface and must be inert from
        // the page.
        browser_env::tests::init_v8();
        let isolate = &mut browser_env::tests::plain_isolate();
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        browser_env::install_worker_global_scope(scope, context).unwrap();
        let scope = &mut v8::ContextScope::new(scope, context);
        // The descriptor on `globalThis` is non-writable, non-configurable
        // and non-enumerable.
        assert_eq!(
            eval(
                scope,
                "const d = Object.getOwnPropertyDescriptor(globalThis, '__turboDomWorker'); \
                 d && d.writable === false && d.configurable === false && \
                 d.enumerable === false ? 'ok' : 'no'"
            ),
            "ok",
            "__turboDomWorker must be locked on globalThis"
        );
        // The namespace object itself is frozen.
        assert_eq!(
            eval(scope, "Object.isFrozen(__turboDomWorker) ? 'ok' : 'no'"),
            "ok",
            "__turboDomWorker must be frozen"
        );
        // A page-side write to `dispatch` is a no-op (strict mode would
        // throw; the bootstrap runs non-strict, so it silently fails).
        assert_eq!(
            eval(
                scope,
                "const original = __turboDomWorker.dispatch; \
                 try { __turboDomWorker.dispatch = () => 'EVIL'; } catch (_) {} \
                 __turboDomWorker.dispatch === original ? 'ok' : 'no'"
            ),
            "ok",
            "page JS must not be able to replace dispatch"
        );
        // A page-side whole-slot overwrite is a no-op.
        assert_eq!(
            eval(
                scope,
                "try { globalThis.__turboDomWorker = { dispatch: () => 'EVIL' }; } catch (_) {} \
                 typeof __turboDomWorker.dispatch === 'function' && \
                 __turboDomWorker.dispatch !== undefined ? 'ok' : 'no'"
            ),
            "ok",
            "page JS must not be able to overwrite __turboDomWorker"
        );
        browser_env::reset();
    }

    #[test]
    fn structured_clone_throws_on_unsupported_inputs() {
        // turbo-dom#428: the real structured-clone implementation (V8
        // ValueSerializer / ValueDeserializer) accepts Date, Map, Set,
        // BigInt, Infinity, NaN, `undefined` (as a property value) and
        // cyclic object graphs; it still throws `DataCloneError` for
        // genuinely non-cloneable values — functions, symbols, WeakMap,
        // WeakSet, Promise, Proxy-wrapped instances of host objects.
        browser_env::tests::init_v8();
        let isolate = &mut browser_env::tests::plain_isolate();
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        browser_env::install_worker_global_scope(scope, context).unwrap();
        let scope = &mut v8::ContextScope::new(scope, context);
        // POJOs still clone.
        assert_eq!(
            eval(
                scope,
                "const c = structuredClone({ a: 1, b: [2, 3] }); \
                 c.a === 1 && c.b[1] === 3 ? 'ok' : 'no'"
            ),
            "ok",
        );
        for src in [
            "structuredClone(() => {})",
            "structuredClone(Symbol('s'))",
            "structuredClone(new WeakMap())",
            "structuredClone(new WeakSet())",
            "structuredClone(Promise.resolve())",
        ] {
            assert_eq!(
                eval(
                    scope,
                    &format!(
                        "try {{ {src}; 'no-throw' }} \
                         catch (e) {{ e && e.name === 'DataCloneError' ? 'ok' : (e && e.name) }}"
                    ),
                ),
                "ok",
                "structuredClone({src}) must throw DataCloneError",
            );
        }
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
        type Setter =
            fn(&mut v8::PinScope, v8::Local<v8::Context>, Option<v8::Local<v8::Function>>);
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
    /// turbo-dom#435: the worker bootstrap must compile with a `ScriptOrigin`
    /// naming `worker_scope.js`, so an exception that unwinds through a
    /// bootstrap frame has a readable stack trace instead of the default
    /// anonymous placeholder.
    #[test]
    fn worker_bootstrap_frames_carry_the_worker_scope_js_resource_name() {
        browser_env::tests::init_v8();
        let isolate = &mut browser_env::tests::plain_isolate();
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        browser_env::install_worker_global_scope(scope, context).unwrap();
        let scope = &mut v8::ContextScope::new(scope, context);
        // `importScripts` with no registered loader throws from the bootstrap
        // body — the top frame of the resulting stack must name worker_scope.js.
        let probe = v8::String::new(
            scope,
            "try { importScripts('x.js'); '' } catch (e) { e.stack || String(e) }",
        )
        .unwrap();
        let stack = v8::Script::compile(scope, probe, None)
            .unwrap()
            .run(scope)
            .unwrap()
            .to_rust_string_lossy(scope);
        assert!(
            stack.contains("worker_scope.js"),
            "stack must name worker_scope.js, got: {stack}"
        );
        browser_env::reset();
    }

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
