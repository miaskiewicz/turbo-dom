//! Activation-gated members with transient activation, dialog / popover edge steps and
//! custom element edges, against the Chrome 156 controls `activation_granted_chrome156.json`,
//! `dialog_popover_edges_chrome156.json` and `custom_elements_edges_chrome156.json` (their
//! `_probe.js`, recorded by `scripts/trusted-input-chrome-control.mjs`, which also delivers
//! trusted CDP mouse and key input).
//!
//! The offline driver plays the embedder: a trusted press sets the window's transient and
//! history-action activation and notifies the close watcher manager
//! ([`notify_user_activation`]); the embedder's [`ActivationHost`] answers from those flags,
//! which the consuming queries clear; a hit on a modal dialog's `::backdrop` targets the
//! dialog ([`modal_backdrop`]) and an uncanceled Esc is a close request
//! ([`process_close_watchers`]).
use super::native_guards_tests::control;
use crate::browser_env::crate_surface::harness_tests::thrown;
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
    light_dismiss(scope, event, args.get(1).is_true());
}

fn host_close_request(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let document = v8::Local::<v8::Object>::try_from(args.get(0)).unwrap();
    rv.set_bool(process_close_watchers(scope, document));
}

fn host_backdrop(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let node = v8::Local::<v8::Object>::try_from(args.get(0)).unwrap();
    let backdrop = modal_backdrop(scope, node);
    let null = v8::null(scope).into();
    rv.set(backdrop.map_or(null, Into::into));
}

const DRIVER: &str = r#"(() => {
  const host = {};
  for (const name of ['Dispatch', 'Activate', 'CloseRequest', 'LightDismiss', 'Backdrop']) { host[name] = globalThis['__host' + name]; delete globalThis['__host' + name]; }
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
  const chain = node => { const out = []; for (; node; node = node.parentNode) out.push(node); return out; };
  let down = null;
  const pointer = step => {
    const hit = document.getElementById(step.target), backdrop = host.Backdrop(hit);
    if ((backdrop !== null) !== !!step.backdrop) throw new Error('backdrop mismatch at ' + step.target);
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
    const event = fire(document.activeElement || document.body, KeyboardEvent, 'keydown', init);
    if (!event.defaultPrevented && step.key === 'Escape') host.CloseRequest(document);
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

/// Run `probe` with the driver above and a virtual-clock task queue standing in for timers.
fn run_probe(probe: &str) -> serde_json::Value {
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
        probe.trim().trim_end_matches(';')
    );
    let source = v8::String::new(scope, &source).unwrap();
    v8::tc_scope!(let scope, scope);
    let promise = v8::Script::compile(scope, source, None)
        .and_then(|script| script.run(scope))
        .and_then(|value| v8::Local::<v8::Promise>::try_from(value).ok())
        .unwrap();
    let step = v8::String::new(scope, "__step()").unwrap();
    let step = v8::Script::compile(scope, step, None).unwrap();
    // Run the earliest timer until the probe settles or no timer is left (or one threw).
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
    reset();
    serde_json::from_str(&value).unwrap()
}

/// Compare a probe result with its control: every difference must be a listed gap
/// (`path = binding value`) and every listed gap must still differ.
fn assert_matches_control(actual: &serde_json::Value, expected: &serde_json::Value, gaps: &[&str]) {
    let mut out = Vec::new();
    differences("result", actual, expected, &mut out);
    let found = out
        .iter()
        .map(|line| {
            let (path, rest) = line.split_once("\n  binding: ").unwrap_or((line, ""));
            let binding = rest.split_once('\n').map_or(rest, |(binding, _)| binding);
            format!("{path} = {binding}")
        })
        .collect::<Vec<_>>();
    let unexpected = found
        .iter()
        .zip(&out)
        .filter(|(line, _)| !gaps.contains(&line.as_str()))
        .map(|(_, full)| full.clone())
        .collect::<Vec<_>>();
    let resolved = gaps
        .iter()
        .filter(|gap| !found.iter().any(|line| line == *gap))
        .collect::<Vec<_>>();
    assert_eq!(
        (unexpected, resolved),
        (Vec::<String>::new(), Vec::<&&str>::new()),
        "(differences, resolved gaps)"
    );
}

#[test]
fn activation_granted_gates_match_chrome156() {
    let expected = control(include_str!("activation_granted_chrome156.json"), PROBE);
    assert_matches_control(&run_probe(PROBE), &expected, &[]);
}

#[test]
fn dialog_popover_edges_match_chrome156() {
    let probe = include_str!("dialog_popover_edges_probe.js");
    let expected = control(include_str!("dialog_popover_edges_chrome156.json"), probe);
    assert_matches_control(&run_probe(probe), &expected, &[]);
}

/// Run `source` (an async function body's value) with the host entry points exposed as
/// `__test*` globals, draining the driver's virtual-clock tasks.
fn run_host_script(source: &str) -> serde_json::Value {
    let probe = format!(
        "(async () => {{ const host = {{ dismiss: __testLightDismiss, dispatch: __testDispatch, close: __testCloseRequest, activate: __testActivate }}; {source} }})()"
    );
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
    bind_method(scope, global, "__testDispatch", host_dispatch);
    bind_method(scope, global, "__testActivate", host_activate);
    bind_method(scope, global, "__testCloseRequest", host_close_request);
    bind_method(scope, global, "__testLightDismiss", host_light_dismiss);
    // The driver (for its virtual clock) captures and deletes its own copies.
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
    let source = format!("Promise.resolve({probe}).then(JSON.stringify, error => {{ throw String(error && error.stack || error); }})");
    let source = v8::String::new(scope, &source).unwrap();
    v8::tc_scope!(let scope, scope);
    let promise = v8::Script::compile(scope, source, None)
        .and_then(|script| script.run(scope))
        .and_then(|value| v8::Local::<v8::Promise>::try_from(value).ok())
        .unwrap();
    let step = v8::String::new(scope, "__step()").unwrap();
    let step = v8::Script::compile(scope, step, None).unwrap();
    for _ in 0..10_000 {
        scope.perform_microtask_checkpoint();
        if promise.state() != v8::PromiseState::Pending
            || !step.run(scope).is_some_and(|value| value.is_true())
        {
            break;
        }
    }
    scope.perform_microtask_checkpoint();
    let value = promise.result(scope).to_rust_string_lossy(scope);
    assert_eq!(promise.state(), v8::PromiseState::Fulfilled, "{value}");
    reset();
    serde_json::from_str(&value).unwrap()
}

/// Host contract: light dismiss acts only on a trusted pointerdown / pointerup the embedder
/// dispatched at a node. Anything else leaves the showing auto popover alone.
#[test]
fn light_dismiss_ignores_events_it_cannot_act_on() {
    let result = run_host_script(
        r#"
        document.body.innerHTML = '<div id=out>o</div><div id=p popover>p</div>';
        const p = document.getElementById('p'), out = document.getElementById('out');
        p.showPopover();
        const open = () => p.matches(':popover-open');
        const states = [];
        host.dismiss({});
        states.push(open());
        const untrusted = new PointerEvent('pointerdown', { bubbles: true });
        out.dispatchEvent(untrusted);
        host.dismiss(untrusted);
        states.push(open());
        const click = new PointerEvent('click', { bubbles: true });
        host.dispatch(click, out);
        host.dismiss(click);
        states.push(open());
        const atWindow = new PointerEvent('pointerdown');
        host.dispatch(atWindow, window);
        host.dismiss(atWindow);
        const upAtWindow = new PointerEvent('pointerup');
        host.dispatch(upAtWindow, window);
        host.dismiss(upAtWindow);
        states.push(open());
        for (const type of ['pointerdown', 'pointerup']) {
            const event = new PointerEvent(type, { bubbles: true });
            host.dispatch(event, out);
            host.dismiss(event);
        }
        states.push(open());
        return states;
        "#,
    );
    assert_eq!(result, serde_json::json!([true, true, true, true, false]));
}

/// Host contract: a close request issued from a dialog's own cancel listener (the embedder
/// re-entering `process_close_watchers`) is absorbed while that cancel action runs.
#[test]
fn reentrant_close_request_during_cancel_is_absorbed() {
    let result = run_host_script(
        r#"
        document.body.innerHTML = '<dialog id=d>d</dialog>';
        const d = document.getElementById('d'), log = [];
        let nested = null;
        d.addEventListener('cancel', event => { log.push(['cancel', event.cancelable]); if (nested === null) nested = host.close(document); });
        d.addEventListener('close', () => log.push(['close', d.open]));
        host.activate(document);
        d.showModal();
        const processed = host.close(document);
        await new Promise(resolve => setTimeout(resolve, 10));
        return [processed, nested, d.open, log];
        "#,
    );
    assert_eq!(
        result,
        serde_json::json!([true, true, false, [["cancel", true], ["close", false]]])
    );
}

#[test]
fn custom_elements_edges_match_chrome156() {
    let probe = include_str!("custom_elements_edges_probe.js");
    let expected = control(include_str!("custom_elements_edges_chrome156.json"), probe);
    assert_matches_control(&run_probe(probe), &expected, &[]);
}

#[test]
fn selection_text_edges_match_chrome156() {
    let probe = include_str!("selection_text_edges_probe.js");
    let expected = control(include_str!("selection_text_edges_chrome156.json"), probe);
    assert_matches_control(
        &run_probe(probe),
        &expected,
        &[
            // miaskiewicz/turbo-dom#144: caret moves into a table stop at its edge.
            r##"result.moves = "#text:ab:1 #text:ab:2 #text:ab:2 #text:ab:2 #text:ab:2 #text:ab:2 #text:ab:2 #text:ab:2 #text:ab:1 #text:ab:0 #text:ab:0 #text:ab:0 #text:ab:0 #text:ab:0 #text:ab:0 #text:ab:0""##,
        ],
    );
}

/// miaskiewicz/turbo-dom#131: a `<details>` whose open attribute the parser set (every
/// fragment parser, template contents) or a clone copied queues a
/// toggle task from "closed" to "open" (Chrome 156).
#[test]
fn parsed_details_toggle_matches_chrome156() {
    let probe = include_str!("details_toggle_probe.js");
    let expected = control(include_str!("details_toggle_chrome156.json"), probe);
    assert_matches_control(
        &run_probe(probe),
        &expected,
        // miaskiewicz/turbo-dom#171: an inert document's tasks never run.
        &[r#"result.template = []"#],
    );
}

/// miaskiewicz/turbo-dom#139: popover method exceptions (Chrome 156 messages) and nested
/// show / hide / toggle calls from beforetoggle.
#[test]
fn popover_exceptions_match_chrome156() {
    let probe = include_str!("popover_exceptions_probe.js");
    let expected = control(include_str!("popover_exceptions_chrome156.json"), probe);
    assert_matches_control(&run_probe(probe), &expected, &[]);
}

/// turbo-dom#171: a task queued for a node of an inert document (template contents) runs
/// on the creating window's event loop: `<details open>` parsed into a template gets its
/// toggle task (Chrome 156: one toggle, "closed" -> "open").
#[test]
#[ignore = "turbo-dom#171: fails until the fix"]
fn template_contents_details_toggle_runs() {
    let got = run_probe(
        "(async () => { document.body.innerHTML = '<template id=t><details id=td open></details></template>'; \
         const log = []; document.getElementById('t').content.firstChild.addEventListener('toggle', e => log.push(e.oldState + '>' + e.newState)); \
         await new Promise(resolve => setTimeout(resolve, 20)); return log; })()",
    );
    assert_eq!(got.to_string(), r#"["closed>open"]"#);
}

/// turbo-dom#144: caret `modify('move', …, 'character')` steps into and out of a table's
/// cells (Chrome 156), with no known gaps left in the control.
#[test]
#[ignore = "turbo-dom#144: fails until the fix"]
fn selection_caret_moves_into_tables_match_chrome156() {
    let probe = include_str!("selection_text_edges_probe.js");
    let expected = control(include_str!("selection_text_edges_chrome156.json"), probe);
    assert_matches_control(&run_probe(probe), &expected, &[]);
}
