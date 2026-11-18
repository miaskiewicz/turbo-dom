use super::*;
use crate::browser_env::crate_surface::harness_tests::thrown;
use sha2::{Digest, Sha256};

/// A deterministic event loop for the bare binding: virtual-time timers, rendering frames on a
/// 16ms grid that run "update animations and send events", then each animation-frame callback,
/// then the pending-animation commit, each as its own task. A frozen host pins the document
/// timeline the way the Chrome control does (CDP Animation.setPlaybackRate 0).
const LOOP: &str = r#"(() => {
  const ns = __tsNs;
  let now = 1000, sequence = 0, frameQueued = false, frameTime = null, handler = null, rafId = 0;
  let callbacks = [];
  const timers = [], steps = [];
  globalThis.setTimeout = (fn, delay = 0, ...args) => { const id = ++sequence; timers.push({ due: now + Math.max(0, +delay || 0), id, run: () => fn(...args) }); return id; };
  globalThis.clearTimeout = id => { const index = timers.findIndex(timer => timer.id === id); if (index >= 0) timers.splice(index, 1); };
  const queueFrame = () => { if (frameQueued) return; frameQueued = true; timers.push({ due: (Math.floor(now / 16) + 1) * 16, id: ++sequence, run: runFrame }); };
  globalThis.requestAnimationFrame = fn => { callbacks.push({ id: ++rafId, fn }); queueFrame(); return rafId; };
  globalThis.cancelAnimationFrame = id => { callbacks = callbacks.filter(entry => entry.id !== id); };
  function runFrame() {
    frameQueued = false; frameTime = now;
    const current = handler; handler = null;
    if (current) steps.push(() => current.update());
    steps.push(() => {
      if (current) current.dispatch();
      const batch = callbacks; callbacks = [];
      for (const entry of batch) steps.push(() => entry.fn(frameTime));
      steps.push(() => { if (current) current.commit(); frameTime = null; });
    });
  }
  ns.installAnimationHost({
    now: () => globalThis.__frozenTimeline ? 5000 : frameTime !== null ? frameTime : now,
    frame(next) { handler = next; queueFrame(); },
    task(callback) { setTimeout(callback, 0); },
  });
  globalThis.__frameQueued = () => frameQueued;
  globalThis.__step = () => {
    if (steps.length) { steps.shift()(); return true; }
    if (!timers.length) return false;
    let index = 0;
    for (let i = 1; i < timers.length; i++) if (timers[i].due < timers[index].due || (timers[i].due === timers[index].due && timers[i].id < timers[index].id)) index = i;
    const timer = timers.splice(index, 1)[0];
    if (timer.due > now) now = timer.due;
    timer.run();
    return true;
  };
})();"#;

pub(super) fn run_probe(html: &str, probe: &str, frozen: bool) -> serde_json::Value {
    run_probe_in(html, probe, frozen, Switches::default())
}

/// The embedder's CSS switches a probe runs under: `set_css_transitions` and
/// `set_css_animations`, set after install (`None` keeps the default).
#[derive(Clone, Copy, Default)]
struct Switches {
    transitions: Option<bool>,
    animations: Option<bool>,
}

/// [`run_probe`] under the embedder's CSS [`Switches`].
fn run_probe_in(html: &str, probe: &str, frozen: bool, switches: Switches) -> serde_json::Value {
    run_policy_probe(html, probe, frozen, switches, true)
}

fn run_policy_probe(
    html: &str,
    probe: &str,
    frozen: bool,
    switches: Switches,
    presentation: bool,
) -> serde_json::Value {
    super::tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, html);
    if let Some(enabled) = switches.transitions {
        set_css_transitions(scope, enabled);
    }
    if let Some(enabled) = switches.animations {
        set_css_animations(scope, enabled);
    }
    if !presentation {
        assert!(set_animation_presentation(
            scope,
            AnimationPresentation::Disabled
        ));
    }
    run_js(scope, &format!("globalThis.__frozenTimeline = {frozen};"));
    run_js(scope, LOOP);
    v8::tc_scope!(let scope, scope);
    let source = v8::String::new(scope, probe).unwrap();
    let value = v8::Script::compile(scope, source, None)
        .and_then(|script| script.run(scope))
        .unwrap_or_else(|| panic!("{}", thrown(scope.exception(), scope)));
    let promise = v8::Local::<v8::Promise>::try_from(value).unwrap();
    let step = v8::String::new(scope, "__step()").unwrap();
    let step = v8::Script::compile(scope, step, None).unwrap();
    // Step until the probe settles or the loop has nothing left to run.
    for _ in 0..200_000 {
        scope.perform_microtask_checkpoint();
        if promise.state() != v8::PromiseState::Pending
            || !step
                .run(scope)
                .unwrap_or_else(|| panic!("{}", thrown(scope.exception(), scope)))
                .is_true()
        {
            break;
        }
    }
    scope.perform_microtask_checkpoint();
    let settled = promise.result(scope).to_rust_string_lossy(scope);
    assert_eq!(
        promise.state(),
        v8::PromiseState::Fulfilled,
        "probe: {settled}"
    );
    let result = promise.result(scope);
    let json = v8::json::stringify(scope, result)
        .unwrap()
        .to_rust_string_lossy(scope);
    let value = serde_json::from_str(&json).unwrap();
    reset();
    value
}

fn control(json: &str, probe: &str, cases: &[u8]) -> (serde_json::Value, serde_json::Value) {
    let control: serde_json::Value = serde_json::from_str(json).unwrap();
    // The frozen headless shell (the same 156.0.8078.47 build) reports itself as HeadlessChrome.
    assert!(["Chrome/156.0.8078.47", "HeadlessChrome/156.0.8078.47"]
        .contains(&control["browser"].as_str().unwrap()));
    assert_eq!(control["errors"], serde_json::json!([]));
    assert_eq!(
        control["probeSha256"],
        format!("{:x}", Sha256::digest(probe.as_bytes()))
    );
    assert_eq!(
        control["casesSha256"],
        format!("{:x}", Sha256::digest(cases))
    );
    let cases: serde_json::Value = serde_json::from_slice(cases).unwrap();
    (control["result"][0]["result"].clone(), cases[0].clone())
}

fn mismatches(
    actual: &serde_json::Value,
    expected: &serde_json::Value,
    path: &str,
    out: &mut Vec<String>,
) {
    match (actual, expected) {
        (serde_json::Value::Object(a), serde_json::Value::Object(e)) => {
            for (key, value) in e {
                mismatches(
                    a.get(key).unwrap_or(&serde_json::Value::Null),
                    value,
                    &format!("{path}.{key}"),
                    out,
                );
            }
            for key in a.keys().filter(|key| !e.contains_key(*key)) {
                out.push(format!("{path}.{key}: unexpected {}", a[key]));
            }
        }
        (serde_json::Value::Array(a), serde_json::Value::Array(e)) if a.len() == e.len() => {
            for (index, (a, e)) in a.iter().zip(e).enumerate() {
                mismatches(a, e, &format!("{path}[{index}]"), out);
            }
        }
        _ if actual != expected => out.push(format!(
            "{path}\n  binding: {actual}\n  chrome:  {expected}"
        )),
        _ => {}
    }
}

// Runs the probe scripts/animations-chrome-control.mjs captured in Chrome 156 with a frozen
// document timeline against the binding on the same frozen virtual clock.
#[test]
fn animations_model_matches_chrome156_control() {
    let probe = include_str!("animations_probe.js");
    let cases = include_bytes!("animations_cases.json");
    let (expected, case) = control(include_str!("animations_chrome156.json"), probe, cases);
    assert_eq!(case["frozenTimeline"], true);
    let mut actual = run_probe(case["html"].as_str().unwrap(), probe, true);
    // Keyframe properties enumerate in Blink's hash-map bucket order over CSSPropertyID (turbo-dom#233).
    assert_eq!(actual["keyframeOrder"], expected["keyframeOrder"]);
    // Animation.rangeStart / rangeEnd now exist (turbo-dom#340); the control carries them and
    // the binding exposes them with the same brand signature, so no KNOWN_GAP is needed.
    let mut out = Vec::new();
    mismatches(&actual, &expected, "", &mut out);
    let gaps = [".shape.Animation.prototype"];
    let real: Vec<_> = out
        .iter()
        .filter(|line| !gaps.iter().any(|gap| line.starts_with(gap)))
        .cloned()
        .collect();
    assert!(
        real.is_empty(),
        "{} mismatches:\n{}",
        real.len(),
        real.join("\n")
    );
    let rows = |value: &serde_json::Value| -> Vec<serde_json::Value> {
        value["shape"]["Animation"]["prototype"]
            .as_array()
            .unwrap()
            .clone()
    };
    assert_eq!(rows(&actual), rows(&expected));
}

// The live-timeline probe Chrome 156 ran on its own frame clock, against the binding on an
// advancing virtual clock: pending tasks, promise/event/callback order and progress relations.
#[test]
fn animations_frame_clock_matches_chrome156_control() {
    let probe = include_str!("animations_frames_probe.js");
    let cases = include_bytes!("animations_frames_cases.json");
    let (expected, case) = control(
        include_str!("animations_frames_chrome156.json"),
        probe,
        cases,
    );
    assert_eq!(case["frozenTimeline"], false);
    let actual = run_probe(case["html"].as_str().unwrap(), probe, false);
    let mut out = Vec::new();
    mismatches(&actual, &expected, "", &mut out);
    assert!(
        out.is_empty(),
        "{} mismatches:\n{}",
        out.len(),
        out.join("\n")
    );
}

// Declarative CSS animations and transitions (turbo-dom#187): CSSAnimation / CSSTransition in
// Element/Document.getAnimations(), their timing and keyframes, playState, pause/cancel and
// cancellation when the declaration goes away, against the frozen-timeline Chrome 156 control.
#[test]
fn css_animations_match_chrome156_control() {
    let probe = include_str!("css_animations_probe.js");
    let cases = include_bytes!("css_animations_cases.json");
    let (expected, case) = control(include_str!("css_animations_chrome156.json"), probe, cases);
    assert_eq!(case["frozenTimeline"], true);
    let actual = run_probe(case["html"].as_str().unwrap(), probe, true);
    let mut out = Vec::new();
    mismatches(&actual, &expected, "", &mut out);
    assert!(
        out.is_empty(),
        "{} mismatches:\n{}",
        out.len(),
        out.join("\n")
    );
}

// Guest replacement of the realm's promise, event, timer and style globals cannot reach the
// animation model: promises still settle, trusted events still arrive, styles still compose.
#[test]
fn animations_ignore_guest_intrinsics() {
    let probe = r#"(async () => {
      const t = document.getElementById('t'), rows = [];
      const OriginalPromise = Promise, then = Promise.prototype.then, add = EventTarget.prototype.addEventListener, dispatch = EventTarget.prototype.dispatchEvent;
      const raf = requestAnimationFrame;
      const a = t.animate({ opacity: [0, 1] }, 40);
      a.onfinish = (event) => rows.push(['finish', event.isTrusted, event.currentTime]);
      Promise.prototype.then = function () { throw new Error('guest then'); };
      EventTarget.prototype.addEventListener = () => { throw new Error('guest add'); };
      EventTarget.prototype.dispatchEvent = () => { throw new Error('guest dispatch'); };
      globalThis.Event = function () { throw new Error('guest Event'); };
      globalThis.getComputedStyle = () => { throw new Error('guest style'); };
      a.pause(); a.currentTime = 20;
      const style = Object.getOwnPropertyDescriptor(globalThis, 'getComputedStyle');
      a.play(); a.finish();
      Promise.prototype.then = then; EventTarget.prototype.addEventListener = add; EventTarget.prototype.dispatchEvent = dispatch;
      await new OriginalPromise((resolve) => raf(() => raf(resolve)));
      rows.push(['state', a.playState, a.currentTime, typeof style.value]);
      return rows;
    })()"#;
    let actual = run_probe("<!doctype html><body><div id=t></div>", probe, false);
    assert_eq!(
        actual,
        serde_json::json!([["finish", true, 40], ["state", "finished", 40, "function"]])
    );
}

// Running animations with no reachable end need no frames: the loop goes idle while style
// still follows the document timeline lazily, as Chrome's between-frames clock does.
#[test]
fn animations_request_frames_only_when_needed() {
    let probe = r#"(async () => {
      const t = document.getElementById('t'), rows = [];
      const a = t.animate({ opacity: [0, 1] }, { duration: 1000, iterations: Infinity });
      await a.ready;
      const start = a.startTime;
      await new Promise((resolve) => setTimeout(resolve, 250));
      rows.push([a.playState, a.pending, Math.round(a.currentTime), getComputedStyle(t).opacity, document.timeline.currentTime - start === a.currentTime, __frameQueued()]);
      const b = t.animate({ marginLeft: ['0px', '100px'] }, 100);
      await b.finished;
      rows.push([b.playState, getComputedStyle(t).marginLeft]);
      return rows;
    })()"#;
    let actual = run_probe("<!doctype html><body><div id=t></div>", probe, false);
    assert_eq!(actual[0][0], "running");
    assert_eq!(actual[0][1], false);
    let elapsed = actual[0][2].as_i64().unwrap();
    assert!((250..=270).contains(&elapsed), "{actual}");
    assert_eq!(actual[0][3], format!("{}", elapsed as f64 / 1000.0));
    assert_eq!(actual[0][4], true);
    assert_eq!(
        actual[0][5], false,
        "an endless animation must not keep frames scheduled"
    );
    // An unset margin computes to 0px in Chrome 156 (turbo-dom#305, computed_defaults_chrome156.json).
    assert_eq!(actual[1], serde_json::json!(["finished", "0px"]));
}

// Without an embedding frame host the document timeline is inactive: animations stay pending
// and play through no frames, exactly like a document that has no rendering.
#[test]
fn animations_without_frame_host_are_inactive() {
    super::tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, "<!doctype html><body><div id=t></div>");
    let value = eval(
        scope,
        "const a=document.getElementById('t').animate({opacity:[0,1]},100);JSON.stringify([document.timeline.currentTime,a.pending,a.playState,a.currentTime,a.startTime,getComputedStyle(document.getElementById('t')).opacity])",
    );
    assert_eq!(value, r#"[null,true,"running",0,null,"0"]"#);
    reset();
}

fn eval(scope: &mut v8::PinScope, source: &str) -> String {
    v8::tc_scope!(let scope, scope);
    let code = v8::String::new(scope, source).unwrap();
    v8::Script::compile(scope, code, None)
        .and_then(|script| script.run(scope))
        .unwrap_or_else(|| panic!("{}", thrown(scope.exception(), scope)))
        .to_rust_string_lossy(scope)
}

#[test]
fn mismatches_report_differences_and_unexpected_keys() {
    let expected = serde_json::json!({"a": [1, 2], "b": "x"});
    let actual = serde_json::json!({"a": [1, 3], "b": "x", "c": true});
    let mut out = Vec::new();
    mismatches(&actual, &expected, "", &mut out);
    assert_eq!(out.len(), 2, "{out:?}");
    assert!(out[0].starts_with(".a[1]"), "{out:?}");
    assert_eq!(out[1], ".c: unexpected true");
}

const TRANSITIONS_HTML: &str = "<!doctype html><html><head></head><body></body></html>";

/// A control's result, after checking it was recorded from `probe` (`key` names the recorder:
/// `jsdom` for scripts/jsdom-media-control.cjs, `browser` for scripts/media-chrome-control.mjs).
fn recorded(json: &str, probe: &str, key: &str, version: &str) -> serde_json::Value {
    let control: serde_json::Value = serde_json::from_str(json).unwrap();
    assert_eq!(control[key], version);
    assert_eq!(
        control["probeSha256"],
        format!("{:x}", Sha256::digest(probe.as_bytes()))
    );
    control["result"].clone()
}

// CSS transitions and animations off (turbo-test's default, for vitest parity): jsdom 27.4.0
// has no Web Animations and runs no CSS transitions or @keyframes animations, so
// getComputedStyle reports the end value at once and no transition / animation event fires.
#[test]
fn css_transitions_off_match_jsdom27_control() {
    let probe = include_str!("jsdom_transitions_probe.js");
    let expected = recorded(
        include_str!("jsdom_transitions_jsdom27.json"),
        probe,
        "jsdom",
        "27.4.0",
    );
    let off = Switches {
        transitions: Some(false),
        animations: Some(false),
    };
    let actual = run_probe_in(TRANSITIONS_HTML, probe, false, off);
    assert_eq!(actual, expected);
}

// CSS transitions and animations on (the default) keep Chrome 156's transitions, @keyframes
// animations and their events (turbo-dom#313), explicitly set or not.
#[test]
fn css_transitions_on_match_chrome156_control() {
    let probe = include_str!("jsdom_transitions_probe.js");
    let expected = recorded(
        include_str!("jsdom_transitions_chrome156.json"),
        probe,
        "browser",
        "HeadlessChrome/156.0.8078.47",
    );
    assert_ne!(expected["events"], serde_json::json!([]));
    let expected = by_target(expected);
    assert_eq!(
        by_target(run_probe(TRANSITIONS_HTML, probe, false)),
        expected
    );
    let on = Switches {
        transitions: Some(true),
        animations: Some(true),
    };
    assert_eq!(
        by_target(run_probe_in(TRANSITIONS_HTML, probe, false, on)),
        expected
    );
}

/// A probe result with its `events` rows stably sorted by target id: each target's own event
/// order is kept, while the order across targets (which Chrome dispatches in its animation
/// update order, Blink's allocation order of the animations, not observable through the DOM)
/// is not compared. css_animation_events_chrome156.json records which frame dispatches each.
fn by_target(mut result: serde_json::Value) -> serde_json::Value {
    let rows = result["events"].as_array_mut().unwrap();
    rows.sort_by(|a, b| a[1].as_str().cmp(&b[1].as_str()));
    result
}

/// The Chrome 156 control's rows with only `transition*` events (`transitions`) or only
/// `animation*` events kept.
fn events_of(control: &serde_json::Value, transitions: bool) -> serde_json::Value {
    let rows = control["events"].as_array().unwrap().iter();
    rows.filter(|row| row[0].as_str().unwrap().starts_with("transition") == transitions)
        .cloned()
        .collect()
}

// set_css_transitions(false) alone (turbo-dom#313): no CSS transition runs and no transition
// event fires (the end value applies at once, as in the jsdom 27 control), while @keyframes
// animations, their events and the whole Web Animations surface stay Chrome's.
#[test]
fn css_transitions_off_keeps_animations() {
    let probe = include_str!("jsdom_transitions_probe.js");
    let chrome = recorded(
        include_str!("jsdom_transitions_chrome156.json"),
        probe,
        "browser",
        "HeadlessChrome/156.0.8078.47",
    );
    let jsdom = recorded(
        include_str!("jsdom_transitions_jsdom27.json"),
        probe,
        "jsdom",
        "27.4.0",
    );
    let mut expected = chrome.clone();
    for key in ["after", "later"] {
        expected[key] = jsdom[key].clone();
    }
    expected["animations"] = serde_json::json!([["CSSAnimation", "a"]]);
    expected["events"] = events_of(&chrome, false);
    assert_eq!(
        expected["events"],
        serde_json::json!([["animationstart", "a"]])
    );
    let off = Switches {
        transitions: Some(false),
        animations: None,
    };
    assert_eq!(
        by_target(run_probe_in(TRANSITIONS_HTML, probe, false, off)),
        by_target(expected)
    );
}

// set_css_animations(false) alone (turbo-dom#313): no @keyframes animation runs and no
// animation event fires (the element keeps its own value, as in the jsdom 27 control), while
// CSS transitions, their events and the whole Web Animations surface stay Chrome's.
#[test]
fn css_animations_off_keeps_transitions() {
    let probe = include_str!("jsdom_transitions_probe.js");
    let chrome = recorded(
        include_str!("jsdom_transitions_chrome156.json"),
        probe,
        "browser",
        "HeadlessChrome/156.0.8078.47",
    );
    let jsdom = recorded(
        include_str!("jsdom_transitions_jsdom27.json"),
        probe,
        "jsdom",
        "27.4.0",
    );
    let mut expected = chrome.clone();
    expected["animated"] = jsdom["animated"].clone();
    expected["animations"] = serde_json::json!([["CSSTransition", "t"], ["CSSTransition", "e"]]);
    expected["events"] = events_of(&chrome, true);
    assert_eq!(expected["events"].as_array().unwrap().len(), 5);
    let off = Switches {
        transitions: None,
        animations: Some(false),
    };
    assert_eq!(
        by_target(run_probe_in(TRANSITIONS_HTML, probe, false, off)),
        by_target(expected)
    );
}

// transition* / animation* events (turbo-dom#313) against Chrome 156 on the advancing virtual
// clock: which frame dispatches each event (before that frame's animation-frame callbacks),
// each event's interface, elapsedTime, animation, bubbles / cancelable / composed and trusted
// flag, phase-skipping and backwards seeks, cancellation by style, display:none and cancel(),
// and the AnimationEvent / TransitionEvent constructors.
#[test]
fn css_animation_events_match_chrome156_control() {
    let probe = include_str!("css_animation_events_probe.js");
    let cases = include_bytes!("css_animation_events_cases.json");
    let (expected, case) = control(
        include_str!("css_animation_events_chrome156.json"),
        probe,
        cases,
    );
    assert_eq!(case["frozenTimeline"], false);
    let actual = run_probe(case["html"].as_str().unwrap(), probe, false);
    let mut out = Vec::new();
    mismatches(&actual, &expected, "", &mut out);
    assert!(
        out.is_empty(),
        "{} mismatches:\n{}",
        out.len(),
        out.join("\n")
    );
}

// Each switch gates its own events on the same scenario: with transitions off no
// transition* event fires (and every animation* event still does), and the other way round.
#[test]
fn css_event_switches_are_independent() {
    let html = serde_json::from_slice::<serde_json::Value>(include_bytes!(
        "css_animation_events_cases.json"
    ))
    .unwrap()[0]["html"]
        .as_str()
        .unwrap()
        .to_owned();
    // The probe drives each CSS animation through getAnimations(); without them it only
    // records the events that fired.
    let events = |switches: Switches| -> Vec<String> {
        let probe = format!(
            "(async () => {{ const rows = []; for (const type of ['transitionrun', 'transitionstart', 'transitionend', 'transitioncancel', 'animationstart', 'animationiteration', 'animationend', 'animationcancel']) document.addEventListener(type, (event) => rows.push(type + ':' + event.target.id)); for (const element of document.querySelectorAll('#wrap > div')) {{ getComputedStyle(element).opacity; element.classList.add('on'); getComputedStyle(element).opacity; }} await new Promise((resolve) => setTimeout(resolve, 100)); return rows; }})()"
        );
        let value = run_probe_in(&html, &probe, false, switches);
        serde_json::from_value(value).unwrap()
    };
    let both = events(Switches::default());
    let starts =
        |rows: &[String], prefix: &str| rows.iter().filter(|row| row.starts_with(prefix)).count();
    assert!(
        starts(&both, "transition") > 0 && starts(&both, "animation") > 0,
        "{both:?}"
    );
    let no_transitions = events(Switches {
        transitions: Some(false),
        animations: None,
    });
    assert_eq!(
        starts(&no_transitions, "transition"),
        0,
        "{no_transitions:?}"
    );
    let animation_rows: Vec<_> = both
        .iter()
        .filter(|row| row.starts_with("animation"))
        .cloned()
        .collect();
    assert_eq!(no_transitions, animation_rows);
    let no_animations = events(Switches {
        transitions: None,
        animations: Some(false),
    });
    let transition_rows: Vec<_> = both
        .iter()
        .filter(|row| row.starts_with("transition"))
        .cloned()
        .collect();
    assert_eq!(no_animations, transition_rows);
    let none = events(Switches {
        transitions: Some(false),
        animations: Some(false),
    });
    assert!(none.is_empty(), "{none:?}");
}

#[test]
fn css_effect_lifecycle_guest_behavior_matches_chrome156() {
    let probe = include_str!("css_effect_lifecycle_probe.js");
    let (mut expected, case) = control(
        include_str!("css_effect_lifecycle_chrome156.json"),
        probe,
        include_bytes!("css_effect_lifecycle_cases.json"),
    );
    // Geometry itself is the native consumer's responsibility. The Chrome control
    // establishes the reported 32px repro; this bare binding tests lifecycle parity.
    assert_eq!(expected["finite"]["width"], serde_json::json!(32));
    expected["finite"].as_object_mut().unwrap().remove("width");
    let mut actual = run_probe(case["html"].as_str().unwrap(), probe, false);
    actual["finite"].as_object_mut().unwrap().remove("width");
    assert_eq!(actual, expected);
}

#[test]
fn scoped_shadow_css_effects_match_chrome156() {
    let probe = include_str!("css_effect_shadow_probe.js");
    let (expected, case) = control(
        include_str!("css_effect_shadow_chrome156.json"),
        probe,
        include_bytes!("css_effect_shadow_cases.json"),
    );
    assert_eq!(
        run_probe(case["html"].as_str().unwrap(), probe, false),
        expected
    );
}

#[test]
fn disabled_presentation_retains_finite_lifecycle_and_underlying_styles() {
    let actual = run_policy_probe(
        "<!doctype html><style>#target{opacity:0.6;transform:none}</style><div id=target></div>",
        r#"(async()=>{
          const target=document.getElementById('target');
          const animation=target.animate({opacity:[0,1],transform:['rotate(0deg)','rotate(90deg)']},{duration:64,fill:'both'});
          await animation.ready;
          const middle=[getComputedStyle(target).opacity,getComputedStyle(target).transform];
          const result=await animation.finished;
          return [middle,getComputedStyle(target).opacity,getComputedStyle(target).transform,
            result===animation,animation.playState,typeof target.animate,typeof document.timeline];
        })()"#,
        false,
        Switches::default(),
        false,
    );
    assert_eq!(
        actual,
        serde_json::json!([
            ["0.6", "none"],
            "0.6",
            "none",
            true,
            "finished",
            "function",
            "object"
        ])
    );
}

#[test]
fn disabled_presentation_does_not_cancel_infinite_or_paused_effects() {
    let actual = run_policy_probe(
        "<!doctype html><div id=target style='opacity:0.6;transform:none'></div>",
        r#"(async()=>{
          const target=document.getElementById('target');
          const animation=target.animate({opacity:[0,1],transform:['rotate(0deg)','rotate(90deg)']},{duration:64,iterations:Infinity,fill:'both'});
          await animation.ready;animation.pause();await animation.ready;animation.currentTime=32;
          let settled=false;animation.finished.then(()=>settled=true,()=>settled=true);
          await new Promise(resolve=>requestAnimationFrame(resolve));
          return [getComputedStyle(target).opacity,getComputedStyle(target).transform,animation.playState,settled];
        })()"#,
        false,
        Switches::default(),
        false,
    );
    assert_eq!(actual, serde_json::json!(["0.6", "none", "paused", false]));
}

#[test]
fn disabled_css_presentation_keeps_canonical_events_and_later_effects() {
    let actual = run_policy_probe(
        "<!doctype html><style>@keyframes spin{to{transform:rotate(90deg)}}#target{opacity:0.4;transform:none;animation:spin 64ms both}</style><div id=target></div>",
        r#"(async()=>{
          const target=document.getElementById('target'), events=[];
          target.addEventListener('animationend',event=>events.push([event.type,event.isTrusted]));
          const css=target.getAnimations()[0];await css.finished;
          await new Promise(resolve=>requestAnimationFrame(resolve));
          const child=document.createElement('div');child.style.cssText='opacity:0.3;transform:none';document.body.append(child);
          const later=child.animate({opacity:[0,1]}, {duration:64,fill:'both'});await later.ready;
          return [getComputedStyle(target).transform,events,getComputedStyle(child).opacity,typeof Animation];
        })()"#,
        false,
        Switches::default(),
        false,
    );
    assert_eq!(
        actual,
        serde_json::json!(["none", [["animationend", true]], "0.3", "function"])
    );
}

#[test]
fn disabled_presentation_preserves_transitions_timers_raf_and_updated_underlying_values() {
    let actual = run_policy_probe(
        "<!doctype html><div id=target style='opacity:0.2;transition:opacity 64ms linear'></div>",
        r#"(async()=>{
          const target=document.getElementById('target'),events=[];
          for(const type of ['transitionrun','transitionstart','transitionend'])target.addEventListener(type,e=>events.push([e.type,e.isTrusted]));
          getComputedStyle(target).opacity;
          await new Promise(resolve=>requestAnimationFrame(resolve));
          target.style.opacity='0.7';const initial=getComputedStyle(target).opacity;
          const transition=target.getAnimations()[0];
          await transition.ready;
          const ready=getComputedStyle(target).opacity;
          let timer=false,raf=false;
          setTimeout(()=>timer=true,0);requestAnimationFrame(()=>raf=true);
          const finished=await transition.finished;
          const finishedState=transition.playState;
          await new Promise(resolve=>setTimeout(resolve,0));
          const later=target.animate({opacity:[0,1]},{duration:64,iterations:Infinity});
          await later.ready;target.style.opacity='0.4';
          let settled=false;later.finished.then(()=>settled=true,()=>settled=true);
          await new Promise(resolve=>requestAnimationFrame(resolve));
          return [initial,ready,finished===transition,finishedState,events,timer,raf,getComputedStyle(target).opacity,settled];
        })()"#,
        false,
        Switches::default(),
        false,
    );
    assert_eq!(
        actual,
        serde_json::json!([
            "0.7",
            "0.7",
            true,
            "finished",
            [
                ["transitionrun", true],
                ["transitionstart", true],
                ["transitionend", true]
            ],
            true,
            true,
            "0.4",
            false
        ])
    );
}
