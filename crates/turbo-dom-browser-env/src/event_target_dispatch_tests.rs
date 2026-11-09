//! turbo-dom#381 + #382: non-Node EventTarget dispatch with a parent hook (phase-filtered
//! capture / target / bubble across a non-Node chain), and a host-dispatch microtask
//! checkpoint between listeners. The fixture `event_target_dispatch_chrome156.json` was
//! captured against Chrome 156 IndexedDB (IDBRequest -> IDBTransaction -> IDBDatabase on
//! the error event). The in-binding test here emulates that chain with three plain
//! EventTargets linked by the embedder-settable parent hook and asserts the same phase
//! sequence, Chrome's `composedPath()`-returns-`[currentTarget]` contract, and the
//! microtask-checkpoint-between-listeners observable.
use super::*;
use sha2::{Digest, Sha256};

const PROBE: &str = include_str!("../scripts/event-target-dispatch-chrome156-probe.js");

fn fixture_result() -> serde_json::Value {
    let json: serde_json::Value =
        serde_json::from_str(include_str!("event_target_dispatch_chrome156.json")).unwrap();
    assert!(json["browser"]
        .as_str()
        .unwrap()
        .starts_with("Chrome/156.0.8078."));
    let digest = Sha256::digest(PROBE.as_bytes());
    let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    assert_eq!(
        json["probeSha256"], hex,
        "control was captured from a different probe"
    );
    json["result"].clone()
}

/// A synthesized non-Node EventTarget chain `request -> tx -> db` with listeners mirroring
/// the Chrome 156 control. Captures phases + composedPath + microtask-between-listeners.
fn run_chain() -> serde_json::Value {
    super::tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    super::install(scope);
    let source = r#"(() => {
      const request = new EventTarget(), tx = new EventTarget(), db = new EventTarget();
      const levels = [['request', request], ['tx', tx], ['db', db]];
      // Parent hook: request -> tx -> db -> null. Bound by the Rust test harness; this is
      // test-only (the production API is `ns.setEventTargetParent`, not page-visible).
      globalThis.__testSetParent(request, () => tx);
      globalThis.__testSetParent(tx, () => db);
      const phases = [];
      const push = (level, obj, capture, event) => {
        const path = event.composedPath();
        phases.push({
          level, capture, phase: event.eventPhase,
          currentIsLevel: event.currentTarget === obj,
          targetIsReq: event.target === request,
          pathLen: path.length,
          pathFirst: path.length > 0 && path[0] === obj,
        });
      };
      for (const [name, obj] of levels) {
        for (const capture of [true, false]) {
          obj.addEventListener('error', (event) => push(name, obj, capture, event), { capture });
        }
      }
      const microtask = [];
      let microFlag = 'pending';
      request.addEventListener('error', () => {
        Promise.resolve().then(() => { microFlag = 'ran'; });
        microtask.push(['listenerA', microFlag]);
      }, { capture: true });
      request.addEventListener('error', () => {
        microtask.push(['listenerB', microFlag]);
      }, { capture: true });
      // Host-initiated dispatch exercises the microtask checkpoint path.
      globalThis.__hostDispatchTarget(new Event('error', { bubbles: true }), request);
      return JSON.stringify({ phases, microtask });
    })()"#;
    let global = context.global(scope);
    bind_method(scope, global, "__hostDispatchTarget", host_dispatch_target);
    // Test-only: the production parent-hook API is `ns.setEventTargetParent`; expose it to
    // the guest script through the engine namespace.
    super::crate_surface::run_engine_script(
        scope,
        "globalThis.__testSetParent = __tsNs.setEventTargetParent;",
    )
    .unwrap();
    let source = v8::String::new(scope, source).unwrap();
    let scope = std::pin::pin!(v8::TryCatch::new(scope));
    let scope = &mut scope.init();
    let result = v8::Script::compile(scope, source, None).and_then(|script| script.run(scope));
    if result.is_none() {
        panic!(
            "event_target_dispatch fixture: {:?}",
            scope
                .exception()
                .map(|value| value.to_rust_string_lossy(scope))
        );
    }
    let text = result.unwrap().to_rust_string_lossy(scope);
    reset();
    serde_json::from_str(&text).unwrap()
}

fn host_dispatch_target(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let event = v8::Local::<v8::Object>::try_from(args.get(0)).expect("event");
    let target = v8::Local::<v8::Object>::try_from(args.get(1)).expect("target");
    dispatch_host_event(scope, event, target);
}

/// turbo-dom#381: phase records across the parent chain match Chrome 156 IDB.
#[test]
fn non_node_event_target_phases_match_chrome156() {
    let expected = fixture_result();
    let actual = run_chain();
    assert_eq!(actual["phases"], expected["phases"], "phase records differ");
}

/// turbo-dom#382: a microtask queued inside listener A has run by the time listener B is
/// invoked (IDB transaction deactivation observable).
#[test]
fn host_dispatch_checkpoints_microtasks_between_listeners() {
    let expected = fixture_result();
    let actual = run_chain();
    assert_eq!(
        actual["microtask"], expected["microtask"],
        "microtask observability differs"
    );
}

/// turbo-dom R1 (expert H1): `collectPath` must dispatch across chains deeper than
/// Chrome's defactor-nonexistent cap. The pre-R1 implementation silently truncated
/// the parent walk at 100 hops; Chrome 156 has no such ceiling, so a 101+-deep chain
/// (an IDB-style tree an embedder could legitimately build, or a surf non-Node
/// hierarchy) dropped listeners past hop 100 without diagnostic. The fix raises the
/// cap to 10 000 (so an adversarial infinite chain throws loudly instead of DoS-ing
/// the dispatch), and asserts the walk reaches every real ancestor.
#[test]
fn collect_path_reaches_ancestors_beyond_the_old_100_hop_cap() {
    super::tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    super::install(scope);
    // Build a chain of 150 EventTargets A0 -> A1 -> ... -> A149, attach a bubble
    // listener to the deepest ancestor, dispatch a bubbling Event on A0. Chrome
    // 156 fires the A149 listener; the pre-fix 100-hop cap stops the walk at
    // A99 and the deep listener never sees the event.
    let source = r#"(() => {
      const DEPTH = 150;
      const nodes = [];
      for (let i = 0; i < DEPTH; i++) nodes.push(new EventTarget());
      for (let i = 0; i < DEPTH - 1; i++) {
        globalThis.__testSetParent(nodes[i], () => nodes[i + 1]);
      }
      let deepSaw = false;
      nodes[DEPTH - 1].addEventListener('x', () => { deepSaw = true; });
      nodes[0].dispatchEvent(new Event('x', { bubbles: true }));
      return JSON.stringify({ deepSaw, depth: DEPTH });
    })()"#;
    super::crate_surface::run_engine_script(
        scope,
        "globalThis.__testSetParent = __tsNs.setEventTargetParent;",
    )
    .unwrap();
    let source = v8::String::new(scope, source).unwrap();
    let scope = std::pin::pin!(v8::TryCatch::new(scope));
    let scope = &mut scope.init();
    let result = v8::Script::compile(scope, source, None)
        .and_then(|script| script.run(scope))
        .unwrap_or_else(|| {
            panic!(
                "deep-chain dispatch script: {:?}",
                scope
                    .exception()
                    .map(|value| value.to_rust_string_lossy(scope))
            )
        })
        .to_rust_string_lossy(scope);
    assert_eq!(
        result, r#"{"deepSaw":true,"depth":150}"#,
        "deepest ancestor saw the bubbled event (no silent truncation at 100)",
    );
    reset();
}

/// turbo-dom#423: a per-level `composedPath()` must be immutable — a listener that stashes
/// the returned array into a closure must still read `[its own currentTarget]` after a
/// later listener up the parent chain runs. The pre-#423 implementation reassigned
/// `record.path = [level]` per invocation; the fix freezes each invocation's path so an
/// internal re-reader or an external caller cannot observe a cross-listener mutation.
#[test]
fn per_level_composed_path_is_frozen_per_invocation() {
    super::tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    super::install(scope);
    // A -> B -> C parent chain. Listener on A (AT_TARGET) stashes composedPath into a
    // closure; listener on C (bubble) re-reads the stashed array and its own composedPath
    // and asserts: stashed is frozen, stashed is still [A], attempting to push to either
    // array throws (strict-mode on frozen target).
    let source = r#"(() => {
      const a = new EventTarget(), b = new EventTarget(), c = new EventTarget();
      globalThis.__testSetParent(a, () => b);
      globalThis.__testSetParent(b, () => c);
      let stashed = null;
      a.addEventListener('x', (event) => { stashed = event.composedPath(); });
      const observed = {};
      c.addEventListener('x', (event) => {
        const own = event.composedPath();
        observed.stashedFrozen = Object.isFrozen(stashed);
        observed.ownFrozen = Object.isFrozen(own);
        observed.stashedLen = stashed.length;
        observed.stashedIsA = stashed[0] === a;
        observed.ownLen = own.length;
        observed.ownIsC = own[0] === c;
        try { stashed.push('nope'); observed.stashedMutable = true; }
        catch (_) { observed.stashedMutable = false; }
      });
      a.dispatchEvent(new Event('x', { bubbles: true }));
      return JSON.stringify(observed);
    })()"#;
    super::crate_surface::run_engine_script(
        scope,
        "globalThis.__testSetParent = __tsNs.setEventTargetParent;",
    )
    .unwrap();
    let source = v8::String::new(scope, source).unwrap();
    let scope = std::pin::pin!(v8::TryCatch::new(scope));
    let scope = &mut scope.init();
    let result = v8::Script::compile(scope, source, None)
        .and_then(|script| script.run(scope))
        .unwrap_or_else(|| {
            panic!(
                "per-level composedPath script: {:?}",
                scope
                    .exception()
                    .map(|value| value.to_rust_string_lossy(scope))
            )
        })
        .to_rust_string_lossy(scope);
    let actual: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(
        actual["stashedFrozen"],
        serde_json::Value::Bool(true),
        "listener A's stashed composedPath() must be frozen (immutable per invocation)"
    );
    assert_eq!(
        actual["ownFrozen"],
        serde_json::Value::Bool(true),
        "listener C's own composedPath() must be frozen"
    );
    assert_eq!(actual["stashedLen"], serde_json::json!(1));
    assert_eq!(
        actual["stashedIsA"],
        serde_json::Value::Bool(true),
        "stashed path's first element must still be A after bubble reaches C"
    );
    assert_eq!(actual["ownLen"], serde_json::json!(1));
    assert_eq!(actual["ownIsC"], serde_json::Value::Bool(true));
    assert_eq!(
        actual["stashedMutable"],
        serde_json::Value::Bool(false),
        "frozen array must reject .push() (strict-mode throw)"
    );
    reset();
}

/// Review R2 V8 M3: the per-listener microtask checkpoint
/// (turbo-dom#382) runs INSIDE `dispatchTarget`'s try/finally while
/// `record.dispatching` is still true, so a microtask that
/// synchronously re-dispatches the same event throws
/// `InvalidStateError` — Chrome 156 permits the re-dispatch because
/// its dispatch flag is per-dispatch, not per-event. Save / restore
/// the dispatch state around the drain so a microtask's re-dispatch
/// of the same event runs in its own scope.
#[test]
fn microtask_may_re_dispatch_the_same_event_without_invalid_state_error() {
    super::tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    super::install(scope);
    // One non-Node EventTarget with one listener. Listener queues a
    // microtask that re-dispatches the same event object. If the
    // microtask throws InvalidStateError the re-dispatch's count
    // stays at 1; otherwise we see 2.
    let source = r#"(() => {
      const target = new EventTarget();
      // The target has a parent hook so dispatchTarget routes through
      // dispatchMulti -> invokeAt (the R1 M3 microtask drain path).
      // Without a parent hook the path is dispatchSingle -> invokeAt, which
      // also drains, but the explicit parent hook mirrors the real IDB
      // consumer that motivated #382.
      globalThis.__testSetParent(target, () => null);
      let count = 0, error = null, ran = false;
      target.addEventListener('ping', (event) => {
        count++;
        if (count === 1) {
          Promise.resolve().then(() => {
            ran = true;
            try { target.dispatchEvent(event); }
            catch (e) { error = (e && e.name) || String(e); }
          });
        }
      });
      target.dispatchEvent(new Event('ping'));
      // The microtask drains inside the dispatch's per-listener finally
      // (turbo-dom#382). After dispatchTarget returns, the microtask has
      // already run — count reflects whether it was allowed through.
      return JSON.stringify({ count, error, ran });
    })()"#;
    super::crate_surface::run_engine_script(
        scope,
        "globalThis.__testSetParent = __tsNs.setEventTargetParent;",
    )
    .unwrap();
    let source = v8::String::new(scope, source).unwrap();
    let scope = std::pin::pin!(v8::TryCatch::new(scope));
    let scope = &mut scope.init();
    let result = v8::Script::compile(scope, source, None)
        .and_then(|script| script.run(scope))
        .unwrap_or_else(|| {
            panic!(
                "re-dispatch script failed: {:?}",
                scope.exception().map(|value| value.to_rust_string_lossy(scope))
            )
        })
        .to_rust_string_lossy(scope);
    assert_eq!(
        result, r#"{"count":2,"error":null,"ran":true}"#,
        "microtask-driven re-dispatch of the same event must not throw",
    );
    reset();
}

/// turbo-dom#382 regression: the per-listener microtask checkpoint
/// (`drainMicrotasksAcrossDispatch`) runs after every listener of
/// `invokeAt`, including listeners of a NESTED dispatch that a host-run
/// listener initiates. Chrome 156 only checkpoints "clean up after
/// running a callback" when the realm is at the top of the script
/// settings stack — a page-script `dispatchEvent` (or
/// `AbortController.abort()`) inside a host-run listener is nested and
/// does not drain its own listeners' microtasks between calls.
///
/// Repro (the surf `abort_signal_any` oracle's `timeoutNestedAbort`
/// scenario, hoisted to a plain non-Node EventTarget so the binding
/// test can exercise it without `AbortSignal.timeout`): a host-run
/// outer listener synchronously dispatches `inner` on a second target;
/// the inner listener queues a microtask, then control returns to the
/// outer listener which pushes `after-inner`. Chrome records
/// `["inner","after-inner","inner-microtask"]`; pre-fix the inner's
/// drain fires the `inner-microtask` before `after-inner`.
#[test]
fn nested_dispatch_inside_host_listener_defers_microtasks_to_outer_drain() {
    super::tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    super::install(scope);
    let global = context.global(scope);
    bind_method(scope, global, "__hostDispatchTarget", host_dispatch_target);
    super::crate_surface::run_engine_script(
        scope,
        "globalThis.__testSetParent = __tsNs.setEventTargetParent;",
    )
    .unwrap();
    let source = r#"(() => {
      const outer = new EventTarget();
      const log = [];
      outer.addEventListener('timeout', () => {
        const inner = new EventTarget();
        inner.addEventListener('abort', () => {
          log.push('inner');
          // Promise.resolve().then stands in for `queueMicrotask` (the bare
          // V8 test isolate has no window-scope polyfill); both enqueue a
          // real microtask.
          Promise.resolve().then(() => log.push('inner-microtask'));
        });
        inner.dispatchEvent(new Event('abort'));
        log.push('after-inner');
      });
      globalThis.__hostDispatchTarget(new Event('timeout'), outer);
      return JSON.stringify(log);
    })()"#;
    let source = v8::String::new(scope, source).unwrap();
    let scope = std::pin::pin!(v8::TryCatch::new(scope));
    let scope = &mut scope.init();
    let result = v8::Script::compile(scope, source, None)
        .and_then(|script| script.run(scope))
        .unwrap_or_else(|| {
            panic!(
                "nested dispatch script: {:?}",
                scope.exception().map(|value| value.to_rust_string_lossy(scope))
            )
        })
        .to_rust_string_lossy(scope);
    assert_eq!(
        result, r#"["inner","after-inner","inner-microtask"]"#,
        "nested page-script dispatch inside a host-run listener must not drain \
         microtasks between its own listeners (Chrome 156: the inner listener's \
         `queueMicrotask` fires only after the outer listener's synchronous \
         tail, not between the inner listener and its outer continuation).",
    );
    reset();
}
