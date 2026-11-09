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
    let text = v8::Script::compile(scope, source, None)
        .and_then(|script| script.run(scope))
        .expect("event_target_dispatch fixture compiled")
        .to_rust_string_lossy(scope);
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
      // turbo-dom#451: isolate two distinct properties.
      //   (1) a SYNC re-dispatch of the same event inside its own listener
      //       must throw InvalidStateError (Chrome's actual dispatch-flag
      //       contract; a mutant that drops the `dispatching` guard would
      //       silently recurse without this probe).
      //   (2) a MICROTASK re-dispatch of the same event must run exactly
      //       once more (count === 2), because the per-listener checkpoint
      //       has saved/restored the dispatch flag by then.
      let count = 0, syncError = null, asyncError = null, ran = false;
      let syncTried = false;
      target.addEventListener('ping', (event) => {
        count++;
        if (count === 1) {
          // Sync probe: must throw InvalidStateError while dispatching.
          syncTried = true;
          try { target.dispatchEvent(event); }
          catch (e) { syncError = (e && e.name) || String(e); }
          // Microtask probe: must not throw and must increment count.
          Promise.resolve().then(() => {
            ran = true;
            try { target.dispatchEvent(event); }
            catch (e) { asyncError = (e && e.name) || String(e); }
          });
        }
      });
      target.dispatchEvent(new Event('ping'));
      // The microtask drains inside the dispatch's per-listener finally
      // (turbo-dom#382). After dispatchTarget returns, the microtask has
      // already run — count reflects whether it was allowed through.
      return JSON.stringify({ count, syncError, asyncError, ran, syncTried });
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
                scope
                    .exception()
                    .map(|value| value.to_rust_string_lossy(scope))
            )
        })
        .to_rust_string_lossy(scope);
    assert_eq!(
        result,
        r#"{"count":2,"syncError":"InvalidStateError","asyncError":null,"ran":true,"syncTried":true}"#,
        "sync re-dispatch while dispatching must throw InvalidStateError; \
         microtask re-dispatch must run exactly once more (count=2)",
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
                scope
                    .exception()
                    .map(|value| value.to_rust_string_lossy(scope))
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

/// turbo-dom#484: a page-script `dispatchEvent` on a non-Node EventTarget (what
/// `controller.abort()` does internally for the 'abort' event, and what
/// `AbortSignal.any`'s subscription hook does to propagate to a dependent signal)
/// must NOT checkpoint microtasks BETWEEN its own listeners. Chrome 156 keeps the
/// microtask queued inside listener A pending until the dispatch unwinds back to
/// the calling script's natural yield (a later `await` / microtask / timer); the
/// synchronous tail of the surrounding script, including listener B, runs before
/// A's queued microtask. #382's per-listener drain fires only for HOST-initiated
/// dispatches (engine-fired, routed through `__captureHostEventDispatch`), not
/// for page-script dispatches. Source: the surf abort-bridge oracle
/// `official_abort_bridge_keeps_chrome_state_and_unadapted_reaction_order`
/// (`harness/detection/abort-bridge-order-probe.js`) — Chrome 156 records
/// `[signal, dependent, after-abort, ..., signal-microtask]`, with the
/// signal-microtask AFTER the dependent listener that ran inside the same
/// `dispatchTarget('abort', signal, true)` call.
#[test]
fn page_script_dispatch_does_not_checkpoint_between_listeners() {
    super::tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    super::install(scope);
    let source = r#"(() => {
      const target = new EventTarget();
      const trace = [];
      let microFlag = 'pending';
      target.addEventListener('abort', () => {
        trace.push('A:' + microFlag);
        Promise.resolve().then(() => { microFlag = 'ran'; trace.push('microtask'); });
      });
      target.addEventListener('abort', () => {
        trace.push('B:' + microFlag);
      });
      // Page-script dispatch, exactly what `controller.abort()`'s internal
      // `dispatchTarget('abort', signal, true)` reduces to (host=true sets
      // isTrusted but the call originates from JS, not from the engine).
      target.dispatchEvent(new Event('abort'));
      trace.push('after-dispatch:' + microFlag);
      return JSON.stringify(trace);
    })()"#;
    let source = v8::String::new(scope, source).unwrap();
    let scope = std::pin::pin!(v8::TryCatch::new(scope));
    let scope = &mut scope.init();
    let result = v8::Script::compile(scope, source, None)
        .and_then(|script| script.run(scope))
        .unwrap_or_else(|| {
            panic!(
                "abort-bridge script: {:?}",
                scope
                    .exception()
                    .map(|value| value.to_rust_string_lossy(scope))
            )
        })
        .to_rust_string_lossy(scope);
    // Chrome 156: listener B observes `microFlag === 'pending'` because the
    // per-listener microtask checkpoint doesn't run for page-script dispatches.
    // The microtask fires only AFTER the dispatch unwinds to the calling
    // script's natural yield. Pre-fix: B saw 'ran' because #382's drain fired
    // unconditionally at the outer listener boundary.
    assert_eq!(
        result,
        r#"["A:pending","B:pending","microtask","after-dispatch:ran"]"#,
        "page-script dispatchEvent must not checkpoint microtasks between \
         its own listeners (turbo-dom#484: the #382 drain is a HOST-dispatch \
         observable only; a page-script dispatch keeps A's queued microtask \
         behind B's synchronous invocation, matching Chrome 156's abort-bridge \
         reaction order)."
    );
    reset();
}

/// Review R3 H1: `drainMicrotasksAcrossDispatch` saves and restores the
/// dispatch-scoped fields (`dispatching` / `currentTarget` / `eventPhase` /
/// `path` / `target`) but not `stop` / `stopImmediate`. A listener on an
/// intermediate target that calls `event.stopPropagation()` and queues a
/// microtask which re-dispatches the same event would see the inner
/// dispatch's `finally` reset `record.stop=false`, and the outer bubble
/// phase run even though the listener asked to stop propagation.
#[test]
fn microtask_re_dispatch_preserves_outer_stop_propagation() {
    super::tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    super::install(scope);
    super::crate_surface::run_engine_script(
        scope,
        "globalThis.__testSetParent = __tsNs.setEventTargetParent;",
    )
    .unwrap();
    // Review R4 M4: tighten the R3 H1 assertion. The earlier version only
    // checked `!has_parent_bubble` — a test that passes for both correct
    // behaviour AND the R4 H1 leak (outer stop leaking INTO the inner
    // dispatch), because both make bubble stop before `parent` runs. Add
    // a `parent` capture listener: it must fire TWICE (once during the
    // outer dispatch's capture, once during the inner re-dispatch's
    // capture — each BEFORE `mid`'s capture calls stopPropagation). With
    // the R4 H1 leak, the inner's `dispatchMulti` bails on its capture
    // loop's first guard and `parent-capture` fires only once (outer).
    let source = r#"(() => {
      const child = new EventTarget(), mid = new EventTarget(), parent = new EventTarget();
      globalThis.__testSetParent(child, () => mid);
      globalThis.__testSetParent(mid, () => parent);
      const trace = [];
      let queued = false;
      parent.addEventListener('ping', (event) => {
        trace.push('parent-capture:' + event.eventPhase);
      }, { capture: true });
      mid.addEventListener('ping', (event) => {
        trace.push('mid-capture:' + event.eventPhase);
        event.stopPropagation();
        if (!queued) {
          queued = true;
          Promise.resolve().then(() => {
            trace.push('microtask-start');
            child.dispatchEvent(event);
            trace.push('microtask-end');
          });
        }
      }, { capture: true });
      parent.addEventListener('ping', (event) => {
        trace.push('parent-bubble:' + event.eventPhase);
      });
      child.addEventListener('ping', (event) => {
        trace.push('child-target:' + event.eventPhase);
      });
      child.dispatchEvent(new Event('ping', { bubbles: true }));
      return JSON.stringify(trace);
    })()"#;
    let source = v8::String::new(scope, source).unwrap();
    let scope = std::pin::pin!(v8::TryCatch::new(scope));
    let scope = &mut scope.init();
    let result = v8::Script::compile(scope, source, None)
        .and_then(|script| script.run(scope))
        .unwrap_or_else(|| {
            panic!(
                "re-dispatch stopPropagation script failed: {:?}",
                scope
                    .exception()
                    .map(|value| value.to_rust_string_lossy(scope))
            )
        })
        .to_rust_string_lossy(scope);
    let trace: serde_json::Value = serde_json::from_str(&result).unwrap();
    let array = trace.as_array().unwrap();
    let has_parent_bubble = array
        .iter()
        .any(|v| v.as_str().unwrap().starts_with("parent-bubble"));
    assert!(
        !has_parent_bubble,
        "parent-bubble must not fire: stopPropagation called on mid should \
         suppress bubble on both outer and inner dispatches. Trace: {trace}",
    );
    // R4 M4 tightening: `parent-capture:1` must fire BOTH during the
    // outer dispatch AND during the inner re-dispatch — each BEFORE
    // `mid`'s capture (which stops propagation). Exactly two
    // occurrences distinguishes correct behaviour from the R4 H1 leak
    // (which skips the inner's capture entirely).
    let parent_capture_count = array
        .iter()
        .filter(|v| v.as_str().unwrap() == "parent-capture:1")
        .count();
    assert_eq!(
        parent_capture_count, 2,
        "parent-capture:1 must fire TWICE — once in the outer dispatch \
         and once in the microtask-driven inner re-dispatch. The R4 H1 \
         leak makes the inner's `dispatchMulti` bail on its first \
         `if (record.stop || record.stopImmediate) return;`, so a \
         single-occurrence trace is the signature of that bug. \
         Trace: {trace}",
    );
    // Also assert the microtask actually ran — guards against a mutant
    // that drops the drain altogether.
    let has_microtask_start = array
        .iter()
        .any(|v| v.as_str().unwrap() == "microtask-start");
    let has_microtask_end = array.iter().any(|v| v.as_str().unwrap() == "microtask-end");
    assert!(
        has_microtask_start && has_microtask_end,
        "the microtask must have started and completed. Trace: {trace}",
    );
    reset();
}

/// Review R4 H1: `drainMicrotasksAcrossDispatch` added `stop` / `stopImmediate`
/// to the SAVE set (R3 H1) but did NOT CLEAR them before the
/// `microtaskCheckpoint`. A nested re-dispatch from a microtask therefore
/// enters `dispatchMulti` / `dispatchTarget` with the outer's `record.stop =
/// true`, and the inner's capture / bubble loops bail out on their first
/// `if (record.stop || record.stopImmediate) return;` check — Chrome 156
/// dispatches each nested dispatch as an independent `stop` scope and runs
/// every phase of the inner. Fix: clear `record.stop` / `record.stopImmediate`
/// after saving but before the checkpoint, so a nested re-dispatch starts
/// fresh and the outer's values restore on the drain's finally.
#[test]
fn nested_re_dispatch_sees_fresh_stop_propagation_state() {
    super::tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    super::install(scope);
    super::crate_surface::run_engine_script(
        scope,
        "globalThis.__testSetParent = __tsNs.setEventTargetParent;",
    )
    .unwrap();
    // Three-level chain child -> mid -> parent. Outer dispatches the SAME
    // event the inner will re-dispatch; the only listener that stops
    // propagation lives on mid (a `pingOnce` guard keeps it from recursing
    // in the inner dispatch). The outer's stopPropagation must NOT leak
    // INTO the inner's dispatch — Chrome 156 treats each nested dispatch as
    // an independent stop scope, so every capture / bubble phase on the
    // inner must still fire. With R4 H1's bug, the drain's checkpoint runs
    // with the outer's `record.stop = true` still on the shared record, and
    // the inner's `dispatchMulti` bails on its first
    // `if (record.stop || record.stopImmediate) return;` — inner capture
    // and bubble phases silently skipped, only AT_TARGET fires.
    let source = r#"(() => {
      const child = new EventTarget(), mid = new EventTarget(), parent = new EventTarget();
      globalThis.__testSetParent(child, () => mid);
      globalThis.__testSetParent(mid, () => parent);
      const trace = [];
      let stoppedOnce = false, queued = false;
      let depth = 0;
      // Capture listeners: path walks parent -> mid -> child, so the
      // parent's capture runs BEFORE mid's and records the outer phase.
      parent.addEventListener('ping', (event) => {
        trace.push('depth=' + depth + ':parent-capture:' + event.eventPhase);
      }, { capture: true });
      mid.addEventListener('ping', (event) => {
        trace.push('depth=' + depth + ':mid-capture:' + event.eventPhase);
        if (!stoppedOnce) {
          stoppedOnce = true;
          event.stopPropagation();
        }
        if (!queued) {
          queued = true;
          Promise.resolve().then(() => {
            trace.push('inner-start');
            depth = 1;
            try { child.dispatchEvent(event); }
            catch (e) { trace.push('inner-threw:' + (e && e.name || String(e))); }
            depth = 0;
            trace.push('inner-end');
          });
        }
      }, { capture: true });
      // AT_TARGET on child.
      child.addEventListener('ping', (event) => {
        trace.push('depth=' + depth + ':child-target:' + event.eventPhase);
      });
      // Bubble: mid, then parent.
      mid.addEventListener('ping', (event) => {
        trace.push('depth=' + depth + ':mid-bubble:' + event.eventPhase);
      });
      parent.addEventListener('ping', (event) => {
        trace.push('depth=' + depth + ':parent-bubble:' + event.eventPhase);
      });
      child.dispatchEvent(new Event('ping', { bubbles: true }));
      return JSON.stringify(trace);
    })()"#;
    let source = v8::String::new(scope, source).unwrap();
    let scope = std::pin::pin!(v8::TryCatch::new(scope));
    let scope = &mut scope.init();
    let result = v8::Script::compile(scope, source, None)
        .and_then(|script| script.run(scope))
        .unwrap_or_else(|| {
            panic!(
                "nested re-dispatch stop-state script: {:?}",
                scope
                    .exception()
                    .map(|value| value.to_rust_string_lossy(scope))
            )
        })
        .to_rust_string_lossy(scope);
    let trace: serde_json::Value = serde_json::from_str(&result).unwrap();
    let array = trace.as_array().unwrap();
    let strs: Vec<String> = array
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let has = |needle: &str| strs.iter().any(|s| s == needle);
    // Outer (depth=0) stop assertions: parent-capture fires BEFORE mid
    // (chain is parent -> mid -> child, capture walks top to target).
    // AT_TARGET's child-target DOES fire — stopPropagation does NOT stop
    // AT_TARGET (only stopImmediatePropagation does). Bubble is skipped.
    assert!(
        has("depth=0:parent-capture:1"),
        "depth=0:parent-capture must fire (capture walks top to target; \
         stopPropagation on mid comes after parent). Trace: {trace}",
    );
    assert!(
        has("depth=0:mid-capture:1"),
        "depth=0:mid-capture must fire (and calls stopPropagation). \
         Trace: {trace}",
    );
    assert!(
        has("depth=0:child-target:2"),
        "depth=0:child-target MUST fire — stopPropagation on a multi-level \
         path does NOT block AT_TARGET's own level (only \
         stopImmediatePropagation does). Trace: {trace}",
    );
    assert!(
        !has("depth=0:mid-bubble:3"),
        "depth=0:mid-bubble must NOT fire — outer stopPropagation. \
         Trace: {trace}",
    );
    assert!(
        !has("depth=0:parent-bubble:3"),
        "depth=0:parent-bubble must NOT fire — outer stopPropagation. \
         Trace: {trace}",
    );
    // Inner dispatch (depth=1) assertions: every phase runs — the outer's
    // stop state must NOT leak INTO the inner through the drain's unreset
    // `record.stop`. With R4 H1's bug, the inner's `dispatchMulti` bails
    // on its capture loop's first `if(record.stop||record.stopImmediate)
    // return;` check — AT_TARGET and bubble never run either (dispatchMulti
    // returns from the capture loop, not from a per-phase guard).
    assert!(
        has("depth=1:parent-capture:1"),
        "depth=1:parent-capture must fire — the outer's stopPropagation \
         must NOT leak into the microtask-driven re-dispatch of the same \
         event (R4 H1). Trace: {trace}",
    );
    assert!(
        has("depth=1:mid-capture:1"),
        "depth=1:mid-capture must fire — the outer's stopPropagation \
         must NOT leak (R4 H1). Trace: {trace}",
    );
    assert!(
        has("depth=1:child-target:2"),
        "depth=1:child-target must fire — with the R4 H1 leak, the inner \
         dispatchMulti bails on its capture loop before AT_TARGET runs. \
         Trace: {trace}",
    );
    assert!(
        has("depth=1:mid-bubble:3"),
        "depth=1:mid-bubble must fire — the outer's stopPropagation \
         must NOT leak (R4 H1). Trace: {trace}",
    );
    assert!(
        has("depth=1:parent-bubble:3"),
        "depth=1:parent-bubble must fire — the outer's stopPropagation \
         must NOT leak (R4 H1). Trace: {trace}",
    );
    reset();
}

/// turbo-dom#450: `record.defaultPrevented` leaks across a microtask-driven
/// nested re-dispatch. The per-listener microtask drain
/// (`drainMicrotasksAcrossDispatch`) saves and restores every other
/// dispatch-scoped field (dispatching / currentTarget / eventPhase / path /
/// target / stop / stopImmediate / trusted), but not `defaultPrevented`.
/// A microtask that re-dispatches the same cancelable event and whose
/// inner listener calls `event.preventDefault()` mutates the shared
/// record's `defaultPrevented = true`; the drain's finally does not
/// restore it, and the outer `dispatchTarget`'s own `finally` only
/// resets `stop` / `stopImmediate` / `passive`. The outer dispatch's
/// `return !record.defaultPrevented` then falsely reports canceled,
/// even though no outer listener called `preventDefault()`.
///
/// Chrome 156 control
/// (`src/event_defaultprevented_chrome156.json` scenario
/// `listenerQueuesMicrotask`): outer dispatch returns `true`, the
/// microtask's inner dispatch returns `false`. Chrome's checkpoint
/// runs the microtask AFTER the outer dispatch completes, so the
/// outer's return is independent of the inner's `preventDefault`.
///
/// Fix: add `defaultPrevented` to `drainMicrotasksAcrossDispatch`'s
/// save-set (same shape as R3 H1's `stop` / `stopImmediate` and R4 H2's
/// `trusted`). The outer's own `preventDefault` taken BEFORE the drain
/// is preserved; the inner's `preventDefault` stays confined to the
/// inner dispatch's return.
#[test]
fn microtask_re_dispatch_preserves_outer_default_prevented() {
    super::tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    super::install(scope);
    super::crate_surface::run_engine_script(
        scope,
        "globalThis.__testSetParent = __tsNs.setEventTargetParent;",
    )
    .unwrap();
    // Plain EventTarget with no parent hook triggers `dispatchSingle`, which
    // still routes callListener through `afterListener` (the drain path).
    // The outer listener queues a microtask whose inner dispatch's listener
    // calls preventDefault. No outer listener calls preventDefault, so the
    // outer `dispatchEvent` MUST return true.
    let source = r#"(() => {
      const target = new EventTarget();
      let step = 0;
      const out = {};
      target.addEventListener('e', (event) => {
        step++;
        if (step === 1) {
          Promise.resolve().then(() => {
            out.innerReturn = target.dispatchEvent(event);
            out.innerDefaultPreventedAfter = event.defaultPrevented;
          });
        } else {
          event.preventDefault();
        }
      });
      const event = new Event('e', { cancelable: true, bubbles: false });
      out.outerReturn = target.dispatchEvent(event);
      return JSON.stringify(out);
    })()"#;
    let source = v8::String::new(scope, source).unwrap();
    let scope = std::pin::pin!(v8::TryCatch::new(scope));
    let scope = &mut scope.init();
    let result = v8::Script::compile(scope, source, None)
        .and_then(|script| script.run(scope))
        .unwrap_or_else(|| {
            panic!(
                "defaultPrevented-leak script: {:?}",
                scope
                    .exception()
                    .map(|value| value.to_rust_string_lossy(scope))
            )
        })
        .to_rust_string_lossy(scope);
    let observed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(
        observed["outerReturn"],
        serde_json::Value::Bool(true),
        "outer dispatchEvent MUST return true — no outer listener called \
         preventDefault; the microtask's inner dispatch's preventDefault \
         must not leak into the outer's return value (Chrome 156 \
         scenario `listenerQueuesMicrotask`.outerReturn = true). \
         Observed: {observed}",
    );
    assert_eq!(
        observed["innerReturn"],
        serde_json::Value::Bool(false),
        "inner dispatchEvent MUST return false — the inner listener \
         called preventDefault (Chrome 156 innerReturn = false). \
         Observed: {observed}",
    );
    assert_eq!(
        observed["innerDefaultPreventedAfter"],
        serde_json::Value::Bool(true),
        "inside the microtask, after the inner dispatch returns, \
         event.defaultPrevented MUST be true (the inner's \
         preventDefault stuck). Observed: {observed}",
    );
    reset();
}

/// turbo-dom#450 — companion to the leak test: an outer listener that DID
/// call `preventDefault` before queueing the microtask must still see its
/// own preventDefault in the outer's return. The save/restore pattern
/// preserves `defaultPrevented = true` when it was set before the drain,
/// and restores `true` (its pre-drain value) after the drain — so the
/// inner's additional `preventDefault` is a no-op for the outer return
/// (which is already false), and an outer-cancelled dispatch cannot
/// silently flip back to uncancelled just because its microtask
/// re-dispatched.
#[test]
fn microtask_re_dispatch_preserves_outer_own_default_prevented() {
    super::tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    super::install(scope);
    super::crate_surface::run_engine_script(
        scope,
        "globalThis.__testSetParent = __tsNs.setEventTargetParent;",
    )
    .unwrap();
    let source = r#"(() => {
      const target = new EventTarget();
      let step = 0;
      const out = {};
      target.addEventListener('e', (event) => {
        step++;
        if (step === 1) {
          // Outer: cancel the outer dispatch FIRST, then queue the microtask.
          event.preventDefault();
          Promise.resolve().then(() => {
            out.innerReturn = target.dispatchEvent(event);
          });
        }
      });
      const event = new Event('e', { cancelable: true, bubbles: false });
      out.outerReturn = target.dispatchEvent(event);
      return JSON.stringify(out);
    })()"#;
    let source = v8::String::new(scope, source).unwrap();
    let scope = std::pin::pin!(v8::TryCatch::new(scope));
    let scope = &mut scope.init();
    let result = v8::Script::compile(scope, source, None)
        .and_then(|script| script.run(scope))
        .unwrap_or_else(|| {
            panic!(
                "defaultPrevented-outer-own script: {:?}",
                scope
                    .exception()
                    .map(|value| value.to_rust_string_lossy(scope))
            )
        })
        .to_rust_string_lossy(scope);
    let observed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(
        observed["outerReturn"],
        serde_json::Value::Bool(false),
        "outer dispatchEvent MUST return false — the outer listener \
         DID call preventDefault; the save/restore around the drain \
         must preserve defaultPrevented when it was true before the \
         drain. Observed: {observed}",
    );
    assert_eq!(
        observed["innerReturn"],
        serde_json::Value::Bool(false),
        "inner dispatchEvent returns false — the event's \
         defaultPrevented is still true at the start of the inner \
         dispatch (Chrome 156 scenario `outerListenerAlsoPrevents`). \
         Observed: {observed}",
    );
    reset();
}

/// Review R4 H2: `record.trusted` is dispatch-scoped — `dispatchTarget`'s
/// entry writes it from the `host` parameter, and neither its own
/// `finally` nor `drainMicrotasksAcrossDispatch`'s save-set restores it.
/// A trusted outer whose microtask re-dispatches the same event flips
/// `record.trusted = false` (untrusted inner), and the outer's
/// subsequent bubble listeners see `event.isTrusted === false` — a
/// trusted event spuriously becomes untrusted mid-dispatch.
///
/// Fix: add `trusted` to the drain's save-set alongside `stop` /
/// `stopImmediate`. (Clearing is not required: the inner's own
/// `dispatchTarget` entry overwrites `trusted` anyway.)
#[test]
fn trusted_flag_survives_nested_re_dispatch_from_microtask() {
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
    // Three-level chain child -> mid -> parent. Host-dispatch on child
    // (trusted). `child`'s capture listener queues a microtask that
    // synchronously re-dispatches the same event through
    // `child.dispatchEvent` (untrusted). After the microtask returns and
    // the outer's dispatch continues, every outer listener on the path
    // must still see `event.isTrusted === true` — the trusted flag of
    // the outer dispatch is not the inner's to toggle.
    let source = r#"(() => {
      const child = new EventTarget(), mid = new EventTarget(), parent = new EventTarget();
      globalThis.__testSetParent(child, () => mid);
      globalThis.__testSetParent(mid, () => parent);
      const trace = [];
      let queued = false;
      // capture on child (runs AT_TARGET on a multi-level — fires first
      // after parent + mid capture). Queue the microtask here; its
      // observable is BUBBLE listeners on mid + parent that follow.
      child.addEventListener('x', (event) => {
        trace.push('outer:child:isTrusted=' + event.isTrusted);
        if (!queued) {
          queued = true;
          Promise.resolve().then(() => {
            trace.push('inner-start:beforeTrusted=' + event.isTrusted);
            try { child.dispatchEvent(event); }
            catch (e) { trace.push('inner-threw:' + (e && e.name || String(e))); }
            trace.push('inner-end:afterTrusted=' + event.isTrusted);
          });
        }
      }, { capture: true });
      // Bubble listeners run AFTER the drain restores (if the save
      // works), so each must still see isTrusted = true on the outer.
      mid.addEventListener('x', (event) => {
        trace.push('outer:mid-bubble:isTrusted=' + event.isTrusted);
      });
      parent.addEventListener('x', (event) => {
        trace.push('outer:parent-bubble:isTrusted=' + event.isTrusted);
      });
      globalThis.__hostDispatchTarget(new Event('x', { bubbles: true }), child);
      return JSON.stringify(trace);
    })()"#;
    let source = v8::String::new(scope, source).unwrap();
    let scope = std::pin::pin!(v8::TryCatch::new(scope));
    let scope = &mut scope.init();
    let result = v8::Script::compile(scope, source, None)
        .and_then(|script| script.run(scope))
        .unwrap_or_else(|| {
            panic!(
                "trusted-leak script: {:?}",
                scope
                    .exception()
                    .map(|value| value.to_rust_string_lossy(scope))
            )
        })
        .to_rust_string_lossy(scope);
    let trace: serde_json::Value = serde_json::from_str(&result).unwrap();
    let array = trace.as_array().unwrap();
    let strs: Vec<String> = array
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let has = |needle: &str| strs.iter().any(|s| s == needle);
    // Outer AT_TARGET listener on child must see trusted (the host set it).
    assert!(
        has("outer:child:isTrusted=true"),
        "outer:child must see isTrusted=true (host dispatch set it). \
         Trace: {trace}",
    );
    // Inner re-dispatch via `child.dispatchEvent(event)` is NOT a host
    // call — the inner's `dispatchTarget` writes `record.trusted =
    // host = false`. Its finally does not restore trusted, so without
    // the save/restore the outer's bubble listeners would see false.
    assert!(
        has("outer:mid-bubble:isTrusted=true"),
        "outer:mid-bubble must see isTrusted=true — the inner \
         (untrusted) re-dispatch's trusted flag must NOT leak back into \
         the outer (R4 H2). Trace: {trace}",
    );
    assert!(
        has("outer:parent-bubble:isTrusted=true"),
        "outer:parent-bubble must see isTrusted=true — the inner \
         (untrusted) re-dispatch's trusted flag must NOT leak back into \
         the outer (R4 H2). Trace: {trace}",
    );
    reset();
}

/// turbo-dom#452: the `collectPath` 10 000-hop cap (R1 H1) was only exercised at 101 hops
/// (the pre-fix silent-truncation boundary). The band between 101 and 10 000 — where Chrome
/// still walks and the fix's raised ceiling matters — was untested, as was the 10 001-hop
/// loud-throw path. This test exercises both:
///   (1) a 9 999-hop chain with a listener on the deepest ancestor fires the listener
///       (walk reaches every real ancestor below the cap),
///   (2) a 10 001-hop chain throws a loud TypeError from `dispatchEvent` (cap enforced).
#[test]
fn collect_path_fires_at_9999_hops_and_throws_at_10001_hops() {
    super::tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    super::install(scope);
    super::crate_surface::run_engine_script(
        scope,
        "globalThis.__testSetParent = __tsNs.setEventTargetParent;",
    )
    .unwrap();
    // Both chains use a native-held array to avoid blowing the parser on 10 000 literals.
    let source = r#"(() => {
      function chain(depth) {
        const nodes = new Array(depth);
        for (let i = 0; i < depth; i++) nodes[i] = new EventTarget();
        for (let i = 0; i < depth - 1; i++) {
          const next = nodes[i + 1];
          globalThis.__testSetParent(nodes[i], () => next);
        }
        return nodes;
      }
      // (1) 9 999 hops: just under the cap. Deep ancestor sees the event.
      const BELOW = 9999;
      const below = chain(BELOW);
      let deepSaw = false;
      below[BELOW - 1].addEventListener('x', () => { deepSaw = true; });
      below[0].dispatchEvent(new Event('x', { bubbles: true }));
      // (2) 10 001 hops: over the cap. Dispatch throws loudly.
      const OVER = 10001;
      const over = chain(OVER);
      // Attach a listener so the fast-skip does not swallow the walk.
      over[OVER - 1].addEventListener('x', () => {});
      let overError = null;
      try { over[0].dispatchEvent(new Event('x', { bubbles: true })); }
      catch (e) { overError = (e && e.name) || String(e); }
      return JSON.stringify({ deepSaw, below: BELOW, overError, over: OVER });
    })()"#;
    let source = v8::String::new(scope, source).unwrap();
    let scope = std::pin::pin!(v8::TryCatch::new(scope));
    let scope = &mut scope.init();
    let result = v8::Script::compile(scope, source, None)
        .and_then(|script| script.run(scope))
        .unwrap_or_else(|| {
            panic!(
                "hop-cap-band script: {:?}",
                scope
                    .exception()
                    .map(|value| value.to_rust_string_lossy(scope))
            )
        })
        .to_rust_string_lossy(scope);
    assert_eq!(
        result, r#"{"deepSaw":true,"below":9999,"overError":"TypeError","over":10001}"#,
        "9 999-hop chain fires deepest listener; 10 001-hop chain throws TypeError",
    );
    reset();
}
