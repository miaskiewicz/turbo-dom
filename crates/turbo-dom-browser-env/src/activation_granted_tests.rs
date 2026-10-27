//! Activation-gated members with transient activation, against the Chrome 156 control
//! `activation_granted_chrome156.json` (`activation_granted_probe.js`, recorded by
//! `scripts/trusted-input-chrome-control.mjs` with trusted CDP mouse input).
//!
//! The offline driver plays the embedder: a trusted press sets the window's transient and
//! history-action activation and notifies the close watcher manager
//! ([`notify_user_activation`]); the embedder's [`ActivationHost`] answers from those flags,
//! which the consuming queries clear.
use super::native_guards_tests::control;
use crate::browser_env::reference_target::probe_harness::differences;
use crate::browser_env::*;

const PROBE: &str = include_str!("activation_granted_probe.js");

fn flag(scope: &mut v8::PinScope, name: &str, value: Option<bool>) -> bool {
    let global = scope.get_current_context().global(scope);
    let key = v8::String::new(scope, name).unwrap();
    let had = global
        .get(scope, key.into())
        .is_some_and(|value| value.is_true());
    if let Some(value) = value {
        let value = v8::Boolean::new(scope, value);
        global.set(scope, key.into(), value.into());
    }
    had
}

fn activation_host(
    scope: &mut v8::PinScope,
    _document: v8::Local<v8::Object>,
    query: ActivationQuery,
) -> bool {
    match query {
        ActivationQuery::Transient => flag(scope, "__transient", None),
        ActivationQuery::HistoryAction => flag(scope, "__history", None),
        ActivationQuery::ConsumeTransient => flag(scope, "__transient", Some(false)),
        ActivationQuery::ConsumeHistoryAction => flag(scope, "__history", Some(false)),
        ActivationQuery::CrossOriginFrame => false,
        ActivationQuery::FullscreenAllowed => true,
    }
}

fn host_dispatch(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let event = v8::Local::<v8::Object>::try_from(args.get(0)).unwrap();
    let target = v8::Local::<v8::Object>::try_from(args.get(1)).unwrap();
    dispatch_host_event(scope, event, target);
}

fn host_activate(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    flag(scope, "__transient", Some(true));
    flag(scope, "__history", Some(true));
    let document = v8::Local::<v8::Object>::try_from(args.get(0)).unwrap();
    notify_user_activation(scope, document);
}

fn host_light_dismiss(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let event = v8::Local::<v8::Object>::try_from(args.get(0)).unwrap();
    light_dismiss(scope, event, false);
}

const DRIVER: &str = r#"(() => {
  const host = {};
  for (const name of ['Dispatch', 'Activate', 'LightDismiss']) { host[name] = globalThis['__host' + name]; delete globalThis['__host' + name]; }
  // Timers on a virtual clock: the earliest due task runs next (FIFO among equals).
  const queue = [];
  let now = 0, sequence = 0;
  globalThis.__step = () => {
    if (!queue.length) return false;
    let best = 0;
    for (let index = 1; index < queue.length; index++) if (queue[index].due < queue[best].due) best = index;
    const [task] = queue.splice(best, 1);
    now = task.due;
    task.callback();
    return true;
  };
  globalThis.setTimeout = (callback, delay) => { queue.push({ due: now + (Number(delay) || 0), callback }); return ++sequence; };
  const fire = (target, Type, type, init) => { const event = new Type(type, init); if (type === 'pointerdown') host.Activate(document); host.Dispatch(event, target); return event; };
  const pointer = step => {
    const target = document.getElementById(step.target) || document.body;
    const init = { bubbles: true, cancelable: true, composed: true, clientX: step.x, clientY: step.y, pointerId: 1, pointerType: 'mouse', isPrimary: true, button: 0, buttons: step.type === 'down' ? 1 : 0 };
    const event = fire(target, PointerEvent, 'pointer' + step.type, init);
    host.LightDismiss(event);
    fire(target, MouseEvent, 'mouse' + step.type, init);
    if (step.type === 'up') fire(target, PointerEvent, 'click', { ...init, isPrimary: false });
  };
  globalThis.__cdpInput = payload => {
    const { id, steps } = JSON.parse(payload);
    for (const step of steps) {
      if (step.type === 'down' || step.type === 'up') setTimeout(() => pointer(step), 0);
      else throw new Error('Unsupported trusted step');
    }
    setTimeout(() => setTimeout(() => globalThis.__cdpInputDone(id), 0), 0);
  };
})()"#;

/// Run the probe with the driver above and a virtual-clock task queue standing in for timers.
fn run_probe() -> serde_json::Value {
    crate::browser_env::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(
        scope,
        "<!doctype html><html><head></head><body></body></html>",
    );
    set_activation_host(context, activation_host);
    let global = context.global(scope);
    bind_method(scope, global, "__hostDispatch", host_dispatch);
    bind_method(scope, global, "__hostActivate", host_activate);
    bind_method(scope, global, "__hostLightDismiss", host_light_dismiss);
    let source = v8::String::new(scope, DRIVER).unwrap();
    v8::Script::compile(scope, source, None)
        .unwrap()
        .run(scope)
        .unwrap();
    capture_popover_scheduler(scope);
    let source = format!(
        "Promise.resolve({}).then(JSON.stringify, error => {{ throw String(error && error.stack || error); }})",
        PROBE.trim().trim_end_matches(';')
    );
    let source = v8::String::new(scope, &source).unwrap();
    v8::tc_scope!(let scope, scope);
    let promise = v8::Script::compile(scope, source, None)
        .and_then(|script| script.run(scope))
        .and_then(|value| v8::Local::<v8::Promise>::try_from(value).ok())
        .unwrap();
    let step = v8::String::new(scope, "__step()").unwrap();
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

/// Measured Chrome behaviour the binding does not reproduce yet, as `path = binding value`.
const KNOWN_GAPS: &[&str] = &[
    // miaskiewicz/turbo-dom#131: a parser-created `<details open>` fires no toggle event.
    r#"result.details.log = [["toggle","d","closed","open"],["toggle","d","open","closed"],["toggle","e","open","open"]]"#,
    // miaskiewicz/turbo-dom#130: entering fullscreen blurs the outside focus before resolving.
    r#"result.focus.result[0] = "BODY""#,
    // miaskiewicz/turbo-dom#129: a request's change event is lost when its element left.
    r#"result.removeChildren.log = [["fullscreenchange","f"],["fullscreenchange","document"],["fullscreenchange","document"]]"#,
    r#"result.removeTop.log = [["fullscreenchange","document"]]"#,
    r#"result.removedWhileExiting.log = [["fullscreenchange","document"]]"#,
    r#"result.shadow.log = [["fullscreenchange","document"]]"#,
];

#[test]
fn activation_granted_gates_match_chrome156() {
    let expected = control(include_str!("activation_granted_chrome156.json"), PROBE);
    let actual = run_probe();
    let mut out = Vec::new();
    differences("result", &actual, &expected, &mut out);
    let found = out
        .iter()
        .map(|line| {
            let (path, rest) = line.split_once("\n  binding: ").unwrap_or((line, ""));
            let binding = rest.split_once("\n").map_or(rest, |(binding, _)| binding);
            format!("{path} = {binding}")
        })
        .collect::<Vec<_>>();
    let unexpected = found
        .iter()
        .zip(&out)
        .filter(|(line, _)| !KNOWN_GAPS.contains(&line.as_str()))
        .map(|(_, full)| full.clone())
        .collect::<Vec<_>>();
    let resolved = KNOWN_GAPS
        .iter()
        .filter(|gap| !found.iter().any(|line| line == *gap))
        .collect::<Vec<_>>();
    assert!(
        unexpected.is_empty() && resolved.is_empty(),
        "{} differences:\n{}\nresolved gaps: {resolved:?}",
        unexpected.len(),
        unexpected.join("\n")
    );
}
