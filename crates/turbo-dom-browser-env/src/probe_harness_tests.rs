//! Offline runner for committed Chrome probes: a deterministic virtual clock behind
//! `setTimeout`/`performance.now` (which the popover/interest scheduler captures), and the
//! `__cdpInput` binding of `scripts/trusted-input-chrome-control.mjs` implemented over the
//! engine's trusted host dispatch, like turbo-surf's host input driver.
use super::*;

fn host_dispatch(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let (Ok(event), Ok(target)) = (
        v8::Local::<v8::Object>::try_from(args.get(0)),
        v8::Local::<v8::Object>::try_from(args.get(1)),
    ) else {
        return;
    };
    dispatch_host_event(scope, event, target);
}
/// The focused element itself (not retargeted out of shadow trees), else the body.
fn host_focused(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let active = ACTIVE
        .with(|active| *active.borrow())
        .filter(|node| with_tree(|tree| is_connected(tree, *node)).unwrap_or(false))
        .or_else(|| with_tree(|tree| tree.query_selector("body")).flatten());
    let value = wrap_opt(scope, active);
    rv.set(value);
}

const DRIVER: &str = r#"(() => {
  const dispatch = globalThis.__hostDispatch, focused = globalThis.__hostFocused;
  delete globalThis.__hostDispatch; delete globalThis.__hostFocused;
  const clock = { now: 0, seq: 0, timers: [] };
  globalThis.__clock = clock;
  globalThis.setTimeout = (callback, delay) => { const id = ++clock.seq; clock.timers.push({ id, due: clock.now + Math.max(0, Number(delay) || 0), callback }); return id; };
  globalThis.clearTimeout = id => { const index = clock.timers.findIndex(timer => timer.id === id); if (index >= 0) clock.timers.splice(index, 1); };
  Object.defineProperty(globalThis, 'performance', { value: { now: () => clock.now }, configurable: true, writable: true });
  let hovered = null;
  const fire = (target, Type, type, init) => { const event = new Type(type, init); dispatch(event, target); return event; };
  const move = target => {
    const init = { bubbles: true, composed: true, pointerType: 'mouse', isPrimary: true, pointerId: 1 };
    if (hovered !== target) {
      if (hovered) { fire(hovered, PointerEvent, 'pointerout', { ...init, relatedTarget: target }); fire(hovered, MouseEvent, 'mouseout', { ...init, relatedTarget: target }); }
      fire(target, PointerEvent, 'pointerover', { ...init, relatedTarget: hovered }); fire(target, MouseEvent, 'mouseover', { ...init, relatedTarget: hovered });
      hovered = target;
    }
    fire(target, PointerEvent, 'pointermove', init); fire(target, MouseEvent, 'mousemove', init);
  };
  const key = step => {
    const init = { key: step.key, code: step.code, keyCode: step.keyCode, shiftKey: !!(step.modifiers & 8), ctrlKey: !!(step.modifiers & 2), altKey: !!(step.modifiers & 1), metaKey: !!(step.modifiers & 4), bubbles: true, cancelable: true, composed: true };
    fire(focused(), KeyboardEvent, 'keydown', init);
    fire(focused(), KeyboardEvent, 'keyup', init);
  };
  globalThis.__cdpInput = payload => {
    const { id, steps } = JSON.parse(payload);
    for (const step of steps) {
      if (step.type === 'move') move(globalThis.__cdpTargets[step.target]);
      else if (step.type === 'key') key(step);
      else throw new Error('Unsupported trusted step');
    }
    // CDP resolves the input command as a later task, after tasks the input queued.
    setTimeout(() => globalThis.__cdpInputDone(id), 0);
  };
})()"#;

/// Run an async committed probe and return its resolved JSON value.
pub(in super::super) fn run(probe: &str) -> serde_json::Value {
    super::super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(
        scope,
        "<!doctype html><html><head></head><body></body></html>",
    );
    let global = scope.get_current_context().global(scope);
    bind_method(scope, global, "__hostDispatch", host_dispatch);
    bind_method(scope, global, "__hostFocused", host_focused);
    run_js(scope, DRIVER);
    capture_popover_scheduler(scope);
    let source = format!(
        "Promise.resolve({}).then(JSON.stringify, error => {{ throw String(error && error.stack || error); }})",
        probe.trim().trim_end_matches(';')
    );
    let source = v8::String::new(scope, &source).unwrap();
    v8::tc_scope!(let scope, scope);
    let promise = v8::Script::compile(scope, source, None)
        .and_then(|script| script.run(scope))
        .and_then(|value| v8::Local::<v8::Promise>::try_from(value).ok())
        .unwrap_or_else(|| {
            panic!(
                "probe threw: {:?}",
                scope
                    .exception()
                    .map(|error| error.to_rust_string_lossy(scope))
            )
        });
    // The earliest due timer runs next; ties keep scheduling order.
    let step = v8::String::new(
        scope,
        "(()=>{const c=__clock;if(!c.timers.length)return false;let best=0;for(let i=1;i<c.timers.length;i++){const t=c.timers[i],b=c.timers[best];if(t.due<b.due||(t.due===b.due&&t.id<b.id))best=i;}const [timer]=c.timers.splice(best,1);c.now=Math.max(c.now,timer.due);timer.callback();return true;})()",
    )
    .unwrap();
    let step = v8::Script::compile(scope, step, None).unwrap();
    for _ in 0..100_000 {
        scope.perform_microtask_checkpoint();
        if promise.state() != v8::PromiseState::Pending {
            break;
        }
        if !step.run(scope).is_some_and(|value| value.is_true()) {
            scope.perform_microtask_checkpoint();
            break;
        }
    }
    if let Some(error) = scope.exception() {
        panic!("probe task threw: {}", error.to_rust_string_lossy(scope));
    }
    let value = promise.result(scope).to_rust_string_lossy(scope);
    assert_eq!(promise.state(), v8::PromiseState::Fulfilled, "{value}");
    reset();
    serde_json::from_str(&value).unwrap()
}

/// Every differing leaf, by path, so one mismatch never hides the next.
pub(in super::super) fn differences(
    path: &str,
    actual: &serde_json::Value,
    expected: &serde_json::Value,
    out: &mut Vec<String>,
) {
    match (actual, expected) {
        (serde_json::Value::Object(a), serde_json::Value::Object(e)) => {
            for (key, value) in e {
                differences(
                    &format!("{path}.{key}"),
                    a.get(key).unwrap_or(&serde_json::Value::Null),
                    value,
                    out,
                );
            }
            for key in a.keys().filter(|key| !e.contains_key(*key)) {
                out.push(format!("{path}.{key}: extra in binding"));
            }
        }
        (serde_json::Value::Array(a), serde_json::Value::Array(e)) if a.len() == e.len() => {
            for (index, (a, e)) in a.iter().zip(e).enumerate() {
                differences(&format!("{path}[{index}]"), a, e, out);
            }
        }
        _ if actual != expected => out.push(format!(
            "{path}\n  binding: {actual}\n  chrome:  {expected}"
        )),
        _ => {}
    }
}

/// Load a committed control, check its provenance, and return its result.
pub(in super::super) fn control(json: &str, probe: &str) -> serde_json::Value {
    use sha2::{Digest, Sha256};
    let control: serde_json::Value = serde_json::from_str(json).unwrap();
    assert_eq!(control["browser"], "Chrome/156.0.8078.47");
    let digest = Sha256::digest(probe.as_bytes());
    let hex = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(
        control["probeSha256"], hex,
        "control was captured from a different probe"
    );
    control["result"].clone()
}
