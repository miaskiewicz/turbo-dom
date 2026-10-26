use super::*;
use sha2::{Digest, Sha256};

/// A deterministic host for the committed probes: a virtual clock, delayed timers, vsync
/// frames on a 60Hz grid that run animation frame callbacks and then the view-transition
/// rendering steps (one job per `__step`, so the driver's microtask checkpoint runs between
/// callbacks as in a browser), rendering suppression, and a configurable page visibility.
/// Each task takes 0.1ms of virtual time, so a busy task loop cannot starve rendering.
const HOST: &str = r#"(() => {
  const interval = 1000 / 60;
  let clock = 0, sequence = 0, timers = [], callbacks = [], steps = [], frameId = 0;
  let requested = false, requestedAt = 0, suppressed = false, jobs = [];
  const request = () => { if (!requested) { requested = true; requestedAt = clock; } };
  globalThis.__hidden = false; globalThis.__errors = [];
  const run = (callback) => { try { callback(); } catch (error) { globalThis.__errors.push(String(error)); } };
  globalThis.setTimeout = (callback, delay, ...args) => {
    const id = ++sequence;
    timers.push({ id, due: clock + Math.max(0, Number(delay) || 0), callback: () => callback(...args) });
    return id;
  };
  globalThis.clearTimeout = (id) => { timers = timers.filter((timer) => timer.id !== id); };
  globalThis.requestAnimationFrame = (callback) => { callbacks.push({ id: ++frameId, callback }); request(); return frameId; };
  globalThis.cancelAnimationFrame = (id) => { callbacks = callbacks.filter((entry) => entry.id !== id); };
  globalThis.queueMicrotask ??= (callback) => { Promise.resolve().then(callback); };
  if (typeof globalThis.performance !== 'object' || globalThis.performance === null) globalThis.performance = {};
  globalThis.performance.now = () => clock;
  globalThis.__viewTransitionHost = (command, argument) => {
    if (command === 'frame') { steps.push(argument); request(); }
    else if (command === 'suppress') { suppressed = argument; if (!suppressed && (callbacks.length || steps.length)) request(); }
    else if (command === 'hidden') return globalThis.__hidden;
  };
  globalThis.__step = () => {
    if (jobs.length) { run(jobs.shift()); return true; }
    const frameAt = requested && !suppressed ? (Math.floor(requestedAt / interval + 1e-6) + 1) * interval : Infinity;
    let next = null;
    for (const timer of timers) if (next === null || timer.due < next.due) next = timer;
    if (next !== null && next.due <= frameAt) {
      timers = timers.filter((timer) => timer !== next);
      clock = Math.max(clock, next.due) + 0.1;
      run(next.callback);
      return true;
    }
    if (frameAt === Infinity) return false;
    clock = Math.max(clock, frameAt);
    const timestamp = Math.round(frameAt * 10) / 10;
    const batch = callbacks; callbacks = [];
    requested = false;
    for (const entry of batch) jobs.push(() => entry.callback(timestamp));
    jobs.push(() => {
      const after = steps; steps = [];
      for (const step of after) jobs.push(() => step(timestamp));
      jobs.push(() => { if (callbacks.length || steps.length) request(); });
    });
    return true;
  };
})();"#;

/// Run an async committed probe against the binding under `HOST` and return its JSON value.
fn run_probe(probe: &str, prelude: &str) -> serde_json::Value {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(
        scope,
        "<!doctype html><html><head></head><body></body></html>",
    );
    run_js(scope, HOST);
    capture_popover_scheduler(scope);
    let global = scope.get_current_context().global(scope);
    let key = v8::String::new(scope, "__viewTransitionHost").unwrap();
    let host = global
        .get(scope, key.into())
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
        .unwrap();
    capture_view_transition_host(scope, host);
    run_js(scope, prelude);
    let source = format!(
        "Promise.resolve({}).then(JSON.stringify)",
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
    let step = v8::String::new(scope, "__step()").unwrap();
    let step = v8::Script::compile(scope, step, None).unwrap();
    for _ in 0..2_000_000 {
        scope.perform_microtask_checkpoint();
        if promise.state() != v8::PromiseState::Pending
            || !step.run(scope).is_some_and(|value| value.is_true())
        {
            break;
        }
    }
    assert_ne!(promise.state(), v8::PromiseState::Pending, "probe stalled");
    let errors = v8::String::new(scope, "JSON.stringify(__errors)").unwrap();
    let errors = v8::Script::compile(scope, errors, None)
        .and_then(|script| script.run(scope))
        .unwrap()
        .to_rust_string_lossy(scope);
    assert_eq!(errors, "[]", "host tasks threw");
    let value = promise.result(scope).to_rust_string_lossy(scope);
    assert_eq!(promise.state(), v8::PromiseState::Fulfilled, "{value}");
    reset();
    serde_json::from_str(&value).unwrap()
}

fn compare(
    actual: &serde_json::Value,
    expected: &serde_json::Map<String, serde_json::Value>,
    known_gaps: &[(&str, &str)],
) -> Vec<String> {
    let mut differences = Vec::new();
    for (section, cases) in expected {
        let Some(cases) = cases.as_object() else {
            if &actual[section] != cases
                && !known_gaps.iter().any(|gap| gap == &(section.as_str(), ""))
            {
                differences.push(format!(
                    "{section}\n  binding: {}\n  chrome:  {cases}",
                    actual[section]
                ));
            }
            continue;
        };
        for (case, value) in cases {
            if known_gaps.contains(&(section.as_str(), case.as_str())) {
                continue;
            }
            if &actual[section][case] != value {
                differences.push(format!(
                    "{section}.{case}\n  binding: {}\n  chrome:  {value}",
                    actual[section][case]
                ));
            }
        }
        if actual[section].as_object().map(|object| object.len()) != Some(cases.len()) {
            differences.push(format!("{section}: case count differs"));
        }
    }
    differences
}

/// The control was captured by running the same probes in a fresh headless Chrome 156
/// profile through scripts/view-transitions-chrome-control.mjs: the main probe in a
/// foreground tab, the hidden probe in a background tab whose document is hidden.
#[test]
fn view_transitions_match_chrome_156() {
    let probe = include_str!("view_transitions_probe.js");
    let hidden = include_str!("view_transitions_hidden_probe.js");
    let control: serde_json::Value =
        serde_json::from_str(include_str!("view_transitions_chrome156.json")).unwrap();
    assert_eq!(control["browser"], "Chrome/156.0.8078.47");
    let actual = run_probe(probe, "");
    // KNOWN_GAP: a bare V8 isolate has no host promise-rejection tracker, so this binding
    // harness never fires unhandledrejection (nor, today, does the turbo-surf render tier).
    assert_eq!(actual["unhandled"], serde_json::json!([]));
    // KNOWN_GAP: DOMParser is provided by the turbo-surf runtime, not this binding; the
    // render tier runs it against Chrome. parseHTMLUnsafe is now in the vendored binding
    // (Sanitizer batch) and is compared against the Chrome fixture below.
    assert_eq!(actual["documents"]["domParser"][0], "unavailable", "domParser");
    // KNOWN_GAP: the binding's selector-method SyntaxError (shared by matches, closest and
    // querySelector*) omits Chrome's "Failed to execute 'matches' on 'Element': " prefix;
    // the rejected selectors and the exception names agree.
    let prefix = "Failed to execute 'matches' on 'Element': ";
    let syntax = control["result"]["selectors"]["syntax"].as_array().unwrap();
    for (index, expected) in syntax.iter().enumerate() {
        let message = expected[1].as_str().unwrap();
        assert_eq!(
            actual["selectors"]["syntax"][index],
            serde_json::json!([
                expected[0],
                message.strip_prefix(prefix).unwrap(),
                expected[2]
            ])
        );
    }
    let mut differences = compare(
        &actual,
        control["result"].as_object().unwrap(),
        &[
            ("unhandled", ""),
            ("documents", "domParser"),
            ("selectors", "syntax"),
        ],
    );
    let actual_hidden = run_probe(
        hidden,
        "globalThis.__hidden = true;Object.defineProperty(document, 'visibilityState', { get: () => 'hidden', configurable: true });",
    );
    if actual_hidden != control["hidden"] {
        differences.push(format!(
            "hidden\n  binding: {actual_hidden}\n  chrome:  {}",
            control["hidden"]
        ));
    }
    assert!(
        differences.is_empty(),
        "{} differences:\n{}",
        differences.len(),
        differences.join("\n")
    );
    assert_eq!(
        control["probeSha256"],
        format!("{:x}", Sha256::digest(probe.as_bytes()))
    );
    assert_eq!(
        control["hiddenProbeSha256"],
        format!("{:x}", Sha256::digest(hidden.as_bytes()))
    );
}

/// A host that never captures the trusted scheduler or a rendering seam (the test runner)
/// still completes transitions on the timer and clock it installed before the binding:
/// frames become 16ms tasks, and nothing suppresses rendering.
#[test]
fn view_transitions_complete_without_a_host() {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    run_js(
        scope,
        "globalThis.__clock=0;globalThis.__timers=[];globalThis.setTimeout=(callback,delay)=>{__timers.push([__clock+(delay||0),callback]);__timers.sort((a,b)=>a[0]-b[0]);};globalThis.performance={now:()=>__clock};",
    );
    install_with_html(
        scope,
        "<!doctype html><html><head></head><body></body></html>",
    );
    run_js(
        scope,
        "globalThis.__log=[];const transition=document.startViewTransition(()=>__log.push('callback:'+(document.activeViewTransition===transition)));transition.updateCallbackDone.then(()=>__log.push('ucd'));transition.ready.then(()=>__log.push('ready:'+document.documentElement.matches(':active-view-transition')));transition.finished.then(()=>__log.push('finished:'+(__clock>=250)+':'+document.activeViewTransition));",
    );
    let step = v8::String::new(
        scope,
        "(()=>{const next=__timers.shift();if(!next)return false;__clock=Math.max(__clock,next[0]);next[1]();return true})()",
    )
    .unwrap();
    let step = v8::Script::compile(scope, step, None).unwrap();
    for _ in 0..1000 {
        scope.perform_microtask_checkpoint();
        if !step.run(scope).unwrap().is_true() {
            break;
        }
    }
    let log = v8::String::new(scope, "JSON.stringify(__log)").unwrap();
    let log = v8::Script::compile(scope, log, None)
        .unwrap()
        .run(scope)
        .unwrap()
        .to_rust_string_lossy(scope);
    assert_eq!(
        log,
        r#"["callback:true","ucd","ready:true","finished:true:null"]"#
    );
    reset();
}
