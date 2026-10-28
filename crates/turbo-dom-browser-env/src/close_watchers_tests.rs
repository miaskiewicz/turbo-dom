//! Close watchers and light dismiss against the Chrome 156 control
//! `close_watchers_chrome156.json` (`close_watchers_probe.js`, recorded by
//! `scripts/trusted-input-chrome-control.mjs` with trusted CDP mouse and key input).
//!
//! The offline driver plays the embedder (turbo-surf's host input driver): it implements the
//! collector's `__cdpInput` binding over the binding's host entry points: a trusted mouse
//! press notifies user activation ([`notify_user_activation`]) and runs
//! [`light_dismiss`] after its pointerdown; a release runs it after pointerup and before the
//! click; a hit outside the topmost modal dialog lands on its `::backdrop`
//! ([`modal_backdrop`]); an uncanceled Esc keydown is a close request
//! ([`process_close_watchers`]). The embedder's [`ActivationHost`] answers from the window's
//! transient and history-action activation flags, which a press sets and consumers clear.
use super::native_guards_tests::control;
use crate::browser_env::crate_surface::harness_tests::thrown;
use crate::browser_env::reference_target::probe_harness::differences;
use crate::browser_env::*;

const PROBE: &str = include_str!("close_watchers_probe.js");

fn global_flag(scope: &mut v8::PinScope, name: &str) -> bool {
    let global = scope.get_current_context().global(scope);
    let key = v8::String::new(scope, name).unwrap();
    global
        .get(scope, key.into())
        .is_some_and(|value| value.is_true())
}

fn set_global_flag(scope: &mut v8::PinScope, name: &str, value: bool) {
    let global = scope.get_current_context().global(scope);
    let key = v8::String::new(scope, name).unwrap();
    let value = v8::Boolean::new(scope, value);
    global.set(scope, key.into(), value.into());
}

/// The window's activation: `__transient` / `__history`, set by a trusted press.
fn activation_host(
    scope: &mut v8::PinScope,
    _document: v8::Local<v8::Object>,
    query: ActivationQuery,
) -> bool {
    match query {
        ActivationQuery::Transient => global_flag(scope, "__transient"),
        ActivationQuery::HistoryAction => global_flag(scope, "__history"),
        ActivationQuery::ConsumeTransient => {
            let had = global_flag(scope, "__transient");
            set_global_flag(scope, "__transient", false);
            had
        }
        ActivationQuery::ConsumeHistoryAction => {
            let had = global_flag(scope, "__history");
            set_global_flag(scope, "__history", false);
            had
        }
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
    set_global_flag(scope, "__transient", true);
    set_global_flag(scope, "__history", true);
    let document = v8::Local::<v8::Object>::try_from(args.get(0)).unwrap();
    notify_user_activation(scope, document);
}

fn host_close_request(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let document = v8::Local::<v8::Object>::try_from(args.get(0)).unwrap();
    rv.set_bool(process_close_watchers(scope, document));
}

fn host_light_dismiss(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let event = v8::Local::<v8::Object>::try_from(args.get(0)).unwrap();
    light_dismiss(scope, event, args.get(1).is_true());
}

fn host_backdrop(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let node = v8::Local::<v8::Object>::try_from(args.get(0)).unwrap();
    match modal_backdrop(scope, node) {
        Some(dialog) => rv.set(dialog.into()),
        None => rv.set_null(),
    }
}

const DRIVER: &str = r#"(() => {
  const host = {};
  for (const name of ['Dispatch', 'Activate', 'CloseRequest', 'LightDismiss', 'Backdrop']) { host[name] = globalThis['__host' + name]; delete globalThis['__host' + name]; }
  const queue = [], closeResults = [];
  globalThis.__closeResults = closeResults;
  globalThis.__queue = queue;
  globalThis.setTimeout = (callback, delay) => { queue.push({ due: Number(delay) || 0, callback }); return queue.length; };
  const fire = (target, Type, type, init) => { const event = new Type(type, init); if (type === 'pointerdown') host.Activate(document); host.Dispatch(event, target); return event; };
  const chain = node => { const out = []; for (; node; node = node.parentNode) out.push(node); return out; };
  let down = null;
  const pointer = step => {
    const [hostId, innerId] = step.target.split('>');
    const outer = document.getElementById(hostId), hit = innerId ? outer.shadowRoot.getElementById(innerId) : outer, backdrop = host.Backdrop(hit);
    if ((backdrop !== null) !== step.backdrop) throw new Error('backdrop mismatch at ' + step.target);
    const target = backdrop || hit;
    const init = { bubbles: true, cancelable: true, composed: true, clientX: step.x, clientY: step.y, pointerId: 1, pointerType: 'mouse', isPrimary: true, button: 0, buttons: step.type === 'down' ? 1 : 0 };
    const event = fire(target, PointerEvent, 'pointer' + step.type, init);
    if (step.type === 'down') {
      host.LightDismiss(event, backdrop !== null);
      if (!event.defaultPrevented) fire(target, MouseEvent, 'mousedown', init);
      down = target;
      return;
    }
    fire(target, MouseEvent, 'mouseup', init);
    host.LightDismiss(event, backdrop !== null);
    const ups = chain(target), common = chain(down).find(node => ups.includes(node));
    down = null;
    if (common) fire(common, PointerEvent, 'click', { ...init, isPrimary: false });
  };
  const key = step => {
    const init = { key: step.key, code: step.code, keyCode: step.keyCode, bubbles: true, cancelable: true, composed: true };
    const target = document.activeElement || document.body;
    const event = fire(target, KeyboardEvent, 'keydown', init);
    if (!event.defaultPrevented && step.key === 'Escape') closeResults.push(host.CloseRequest(document));
    fire(document.activeElement || document.body, KeyboardEvent, 'keyup', init);
  };
  globalThis.__cdpInput = payload => {
    // CDP delivers each input event as its own task after the tasks already queued, and
    // resolves the command after the tasks the input queued.
    const { id, steps } = JSON.parse(payload);
    for (const step of steps) {
      if (step.type === 'down' || step.type === 'up') setTimeout(() => pointer(step), 0);
      else if (step.type === 'key') setTimeout(() => key(step), 0);
      else throw new Error('Unsupported trusted step');
    }
    setTimeout(() => setTimeout(() => globalThis.__cdpInputDone(id), 0), 0);
  };
})()"#;

/// Run the probe with the driver above and a FIFO task queue standing in for timers (each
/// task runs after the microtasks of the previous one).
fn run_probe() -> (serde_json::Value, serde_json::Value) {
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
    bind_method(scope, global, "__hostCloseRequest", host_close_request);
    bind_method(scope, global, "__hostLightDismiss", host_light_dismiss);
    bind_method(scope, global, "__hostBackdrop", host_backdrop);
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
    let step = v8::String::new(
        scope,
        "__queue.length?(__queue.shift().callback(),true):false",
    )
    .unwrap();
    let step = v8::Script::compile(scope, step, None).unwrap();
    // Run queued tasks until the probe settles or none is left (or one threw).
    for _ in 0..100_000 {
        scope.perform_microtask_checkpoint();
        if promise.state() != v8::PromiseState::Pending
            || !step.run(scope).is_some_and(|value| value.is_true())
        {
            break;
        }
    }
    scope.perform_microtask_checkpoint();
    let error = thrown(scope.exception(), scope);
    assert_eq!(error, "no exception", "probe task threw");
    let value = promise.result(scope).to_rust_string_lossy(scope);
    assert_eq!(promise.state(), v8::PromiseState::Fulfilled, "{value}");
    let results = v8::String::new(scope, "JSON.stringify(__closeResults)").unwrap();
    let results = v8::Script::compile(scope, results, None)
        .unwrap()
        .run(scope)
        .unwrap()
        .to_rust_string_lossy(scope);
    reset();
    (
        serde_json::from_str(&value).unwrap(),
        serde_json::from_str(&results).unwrap(),
    )
}

#[test]
#[ignore = "turbo-dom#119: fails until the fix"]
fn close_watchers_and_light_dismiss_match_chrome156() {
    let expected = control(include_str!("close_watchers_chrome156.json"), PROBE);
    let mut expected = expected;
    let (mut actual, close_results) = run_probe();
    // process_close_watchers reports whether an enabled close watcher took each Esc: not for
    // closedby=none (9th), a non-modal dialog (11th) or removed dialogs (18th, 19th).
    let mut processed = vec![true; 19];
    for index in [8, 10, 17, 18] {
        processed[index] = false;
    }
    assert_eq!(close_results, serde_json::json!(processed));
    // `close` events and `toggle` events are tasks on different task sources: Chrome 156
    // interleaves the two sources either way (recordings differ under load), the binding in
    // queueing order. Within each run of consecutive close / toggle events, compare the close
    // events (in order) followed by the toggle events (in order).
    for result in [&mut expected, &mut actual] {
        for scenario in result.as_object_mut().unwrap().values_mut() {
            let log = scenario["log"].as_array_mut().unwrap();
            let mut index = 0;
            while index < log.len() {
                let task = |entry: &serde_json::Value| entry[0] == "close" || entry[0] == "toggle";
                if !task(&log[index]) {
                    index += 1;
                    continue;
                }
                let end = (index..log.len())
                    .find(|&end| !task(&log[end]))
                    .unwrap_or(log.len());
                log[index..end].sort_by_key(|entry| entry[0] != "close");
                index = end;
            }
        }
    }
    let mut out = Vec::new();
    differences("result", &actual, &expected, &mut out);
    assert!(
        out.is_empty(),
        "{} differences:\n{}",
        out.len(),
        out.join("\n")
    );
}

/// Light dismiss only acts on trusted pointerdown / pointerup events at a node: page-made
/// events, other event types and non-events leave an open auto popover alone, while the same
/// trusted press and release outside it hide it.
#[test]
fn light_dismiss_ignores_untrusted_and_other_events() {
    crate::browser_env::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(
        scope,
        "<!doctype html><div id=out></div><div id=p popover>p</div>",
    );
    let eval = |scope: &mut v8::PinScope, source: &str| -> String {
        let code = v8::String::new(scope, source).unwrap();
        let value = v8::Script::compile(scope, code, None)
            .unwrap()
            .run(scope)
            .unwrap();
        value.to_rust_string_lossy(scope)
    };
    eval(scope, "globalThis.p = document.getElementById('p'); globalThis.out = document.getElementById('out'); p.showPopover()");
    let object = |scope: &mut v8::PinScope<'_, '_>, source: &str| {
        let code = v8::String::new(scope, source).unwrap();
        let value = v8::Script::compile(scope, code, None)
            .unwrap()
            .run(scope)
            .unwrap();
        v8::Global::new(scope, v8::Local::<v8::Object>::try_from(value).unwrap())
    };
    let target = object(scope, "out");
    for source in [
        "new PointerEvent('pointerdown', { bubbles: true })",
        "new PointerEvent('pointerup', { bubbles: true })",
        "({ type: 'pointerdown', isTrusted: true })",
    ] {
        let event = object(scope, source);
        let event = v8::Local::new(scope, &event);
        if source.starts_with("new") {
            let target = v8::Local::new(scope, &target);
            let code = v8::String::new(scope, "(e, t) => t.dispatchEvent(e)").unwrap();
            let function = v8::Script::compile(scope, code, None)
                .unwrap()
                .run(scope)
                .unwrap();
            let function = v8::Local::<v8::Function>::try_from(function).unwrap();
            let undefined = v8::undefined(scope).into();
            function.call(scope, undefined, &[event.into(), target.into()]);
        }
        light_dismiss(scope, event, false);
    }
    // Trusted, but neither pointerdown nor pointerup.
    for kind in ["click", "mousedown"] {
        let event = object(
            scope,
            &format!("new MouseEvent('{kind}', {{ bubbles: true }})"),
        );
        let event = v8::Local::new(scope, &event);
        let target = v8::Local::new(scope, &target);
        dispatch_host_event(scope, event, target);
        light_dismiss(scope, event, false);
    }
    assert_eq!(eval(scope, "p.matches(':popover-open')"), "true");
    for kind in ["pointerdown", "pointerup"] {
        let event = object(
            scope,
            &format!("new PointerEvent('{kind}', {{ bubbles: true }})"),
        );
        let event = v8::Local::new(scope, &event);
        let target = v8::Local::new(scope, &target);
        dispatch_host_event(scope, event, target);
        light_dismiss(scope, event, false);
    }
    assert_eq!(eval(scope, "p.matches(':popover-open')"), "false");
    reset();
}

/// The stand-in embedder's activation answers: the window's flags, consumed by the consume
/// queries; never a cross-origin frame; fullscreen always allowed.
#[test]
fn activation_host_answers_from_the_window_flags() {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, "<!doctype html><body>");
    let code = v8::String::new(
        scope,
        "globalThis.__transient = true; globalThis.__history = true; document",
    )
    .unwrap();
    let document = v8::Script::compile(scope, code, None)
        .unwrap()
        .run(scope)
        .unwrap();
    let document = v8::Local::<v8::Object>::try_from(document).unwrap();
    let mut answers = Vec::new();
    for query in [
        ActivationQuery::Transient,
        ActivationQuery::CrossOriginFrame,
        ActivationQuery::FullscreenAllowed,
        ActivationQuery::ConsumeTransient,
        ActivationQuery::Transient,
        ActivationQuery::HistoryAction,
        ActivationQuery::ConsumeHistoryAction,
        ActivationQuery::HistoryAction,
    ] {
        answers.push(activation_host(scope, document, query));
    }
    assert_eq!(answers, [true, false, true, true, false, true, true, false]);
    reset();
}
