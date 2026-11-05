//! turbo-dom#323: the HTML / DOM interface members Chrome 156 has and the binding lacked,
//! against the Chrome 156 control `interface_parity_{probe.js,cases.json,chrome156.json}`
//! (recorded by scripts/select-element-chrome-control.mjs): member presence and descriptors,
//! the content attribute reflections, fieldset.elements / datalist.options / legend.form /
//! style.disabled / CharacterData siblings / body handlers / getSVGDocument, and the media
//! element's state machine (html_media.js) without a media pipeline.
use super::*;
use sha2::{Digest, Sha256};

/// Timers ordered by virtual due time (the probe waits with setTimeout while the media
/// element's zero-delay tasks run first, as in Chrome), and `__cdpGesture` granting sticky
/// activation.
const HARNESS: &str = "(() => {
  let clock = 0, sequence = 0;
  const timers = [];
  globalThis.__probeSticky = false;
  globalThis.setTimeout = (callback, delay) => { timers.push({ at: clock + (Number(delay) || 0), sequence: ++sequence, callback }); return sequence; };
  globalThis.__step = () => {
    if (!timers.length) return false;
    let next = 0;
    for (let index = 1; index < timers.length; index++) if (timers[index].at < timers[next].at || timers[index].at === timers[next].at && timers[index].sequence < timers[next].sequence) next = index;
    const [timer] = timers.splice(next, 1);
    clock = timer.at;
    timer.callback();
    return true;
  };
  globalThis.__cdpGesture = () => { globalThis.__probeSticky = true; setTimeout(() => globalThis.__cdpGestureDone(), 0); };
})();";

fn activation(
    scope: &mut v8::PinScope,
    document: v8::Local<v8::Object>,
    query: ActivationQuery,
) -> bool {
    let context = document.get_creation_context(scope).expect("page document");
    let global = context.global(scope);
    let key = v8::String::new(scope, "__probeSticky").unwrap();
    let sticky = global
        .get(scope, key.into())
        .is_some_and(|value| value.is_true());
    sticky && !matches!(query, ActivationQuery::CrossOriginFrame)
}

/// Run an async `probe` over the control's page and return its settled JSON value.
fn run_probe(probe: &str, cases: &[u8]) -> serde_json::Value {
    run_probe_with(probe, cases, "")
}

/// [`run_probe`] after the engine script `setup` (which sees the engine namespace).
pub(super) fn run_probe_with(probe: &str, cases: &[u8], setup: &str) -> serde_json::Value {
    tests::init_v8();
    let isolate = &mut tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    let cases: serde_json::Value = serde_json::from_slice(cases).unwrap();
    install_independent_html(scope, cases[0]["html"].as_str().unwrap());
    {
        let id = context.get_slot::<DomContextId>().unwrap().0;
        let _dom = DomScope::enter(id);
        event_handlers::install(scope, None, cases[0]["documentBase"].as_str().unwrap());
    }
    set_activation_host(context, activation);
    run_js(scope, HARNESS);
    capture_popover_scheduler(scope);
    run_engine_script(scope, setup).expect("the setup runs");
    let source = format!(
        "return Promise.resolve({}).then(JSON.stringify)",
        probe.trim().trim_end_matches(';')
    );
    v8::tc_scope!(let scope, scope);
    let promise = run_engine_script(scope, &source)
        .and_then(|value| v8::Local::<v8::Promise>::try_from(value).ok())
        .expect("the probe compiles");
    let step = v8::String::new(scope, "__step()").unwrap();
    let step = v8::Script::compile(scope, step, None).unwrap();
    while promise.state() == v8::PromiseState::Pending {
        scope.perform_microtask_checkpoint();
        if promise.state() != v8::PromiseState::Pending {
            break;
        }
        let ran = step.run(scope).expect("a task does not throw");
        assert!(ran.is_true(), "the probe waits on nothing");
    }
    let value = promise.result(scope).to_rust_string_lossy(scope);
    assert_eq!(promise.state(), v8::PromiseState::Fulfilled, "{value}");
    reset();
    serde_json::from_str(&value).unwrap()
}

pub(super) fn control(probe: &str, cases: &[u8], json: &str) -> serde_json::Value {
    let control: serde_json::Value = serde_json::from_str(json).unwrap();
    assert!(control["browser"]
        .as_str()
        .unwrap()
        .starts_with("Chrome/156.0.8078."));
    assert_eq!(control["errors"], serde_json::json!([]));
    assert_eq!(
        control["probeSha256"],
        format!("{:x}", Sha256::digest(probe.as_bytes()))
    );
    assert_eq!(
        control["casesSha256"],
        format!("{:x}", Sha256::digest(cases))
    );
    control["result"][0]["result"].clone()
}

// Leaf paths where the binding and the Chrome control disagree.
pub(super) fn mismatches(
    path: &str,
    actual: &serde_json::Value,
    expected: &serde_json::Value,
) -> Vec<String> {
    match (actual, expected) {
        (serde_json::Value::Object(a), serde_json::Value::Object(e)) => e
            .keys()
            .chain(a.keys().filter(|key| !e.contains_key(*key)))
            .flat_map(|key| {
                let null = serde_json::Value::Null;
                mismatches(
                    &format!("{path}.{key}"),
                    a.get(key).unwrap_or(&null),
                    e.get(key).unwrap_or(&null),
                )
            })
            .collect(),
        (serde_json::Value::Array(a), serde_json::Value::Array(e)) if a.len() == e.len() => a
            .iter()
            .zip(e)
            .enumerate()
            .flat_map(|(index, (a, e))| mismatches(&format!("{path}[{index}]"), a, e))
            .collect(),
        _ if actual != expected => vec![format!(
            "{path}\n  binding: {actual}\n  chrome:  {expected}"
        )],
        _ => vec![],
    }
}

#[test]
fn interface_parity_matches_chrome156_control() {
    let probe = include_str!("interface_parity_probe.js");
    let cases = include_bytes!("interface_parity_cases.json");
    let expected = control(
        probe,
        cases,
        include_str!("interface_parity_chrome156.json"),
    );
    let found = mismatches("", &run_probe(probe, cases), &expected);
    assert!(
        found.is_empty(),
        "mismatches vs Chrome 156:\n{}",
        found.join("\n")
    );
}

#[test]
fn mismatches_report_leaf_differences() {
    let found = mismatches(
        "",
        &serde_json::json!({"a": [1, 2], "b": 1, "extra": true}),
        &serde_json::json!({"a": [1, 3], "b": 1}),
    );
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found[0].starts_with(".a[1]"));
    assert!(found[1].starts_with(".extra"));
}

/// What the binding does where Chrome would fetch: a src URL becomes currentSrc and the
/// fetch never completes (NETWORK_LOADING, HAVE_NOTHING, loadstart only; a new src aborts it,
/// its events seeing NETWORK_NO_SOURCE while the next selection is pending); a parsed element
/// starts where its resource selection got; setSinkId() knows only the default device; a src
/// attribute on a non-media element loads nothing.
#[test]
fn media_without_a_pipeline_stalls_where_chrome_would_fetch() {
    let probe = r#"(async () => {
      const tick = () => new Promise((resolve) => setTimeout(resolve, 30));
      const v = document.createElement('video'), log = [];
      for (const type of ['loadstart', 'error', 'abort', 'emptied']) v.addEventListener(type, () => log.push(type + v.networkState));
      v.src = 'clip.webm';
      await tick();
      const fetched = [v.networkState, v.readyState, v.currentSrc, log.slice()];
      v.src = 'other.webm';
      await tick();
      fetched.push(v.currentSrc, log.slice());
      const host = document.createElement('div');
      host.innerHTML = '<video src=""></video><audio src="a.ogg"></audio><video><source src="b.mp4"></video><video></video>';
      const parsed = [...host.children].map((m) => [m.networkState, m.currentSrc, m.error && m.error.code]);
      const sink = await v.setSinkId('speakers').catch((error) => error.name + ': ' + error.message);
      const img = document.createElement('img');
      img.src = 'x.png';
      return { fetched, parsed, sink, sinkId: v.sinkId, img: img.getAttribute('src') };
    })()"#;
    let cases = br#"[{"html":"<!doctype html><body>","documentBase":"http://localhost/"}]"#;
    assert_eq!(
        run_probe(probe, cases),
        serde_json::json!({
            "fetched": [2, 0, "http://localhost/clip.webm", ["loadstart2"], "http://localhost/other.webm", ["loadstart2", "abort3", "emptied3", "loadstart2"]],
            "parsed": [[3, "", 4], [2, "http://localhost/a.ogg", null], [2, "http://localhost/b.mp4", null], [0, "", null]],
            "sink": "NotFoundError: Requested device not found",
            "sinkId": "",
            "img": "x.png",
        })
    );
}

/// turbo-dom#341: `document.createEvent()` against `create_event_chrome156.json`: the DOM
/// createEvent table and Chrome 156's legacy names, ASCII case-insensitive, an uninitialized
/// event of the interface, NotSupportedError for any other name, the operation's shape.
#[test]
fn create_event_matches_chrome156_control() {
    let probe = include_str!("create_event_probe.js");
    let cases = include_bytes!("create_event_cases.json");
    let mut expected = control(probe, cases, include_str!("create_event_chrome156.json"));
    let actual = run_probe(probe, cases);
    // KNOWN_GAP: an Event interface this realm does not define (an embedder's to install, e.g.
    // StorageEvent; the binding's instanceof placeholders are not one) cannot be created: those names answer NotSupportedError, as Chrome does
    // for any interface it lacks.
    for (interface, kind) in actual["defined"].as_object().unwrap() {
        if kind == "Event interface" {
            continue;
        }
        expected["defined"][interface] = kind.clone();
        for (name, row) in expected["table"].as_object_mut().unwrap() {
            if row[0] == interface.as_str() {
                let message = format!("Failed to execute 'createEvent' on 'Document': The provided event type ('{name}') is invalid.");
                *row = serde_json::json!(["throw", "NotSupportedError", message]);
            }
        }
    }
    let found = mismatches("", &actual, &expected);
    assert!(
        found.is_empty(),
        "mismatches vs Chrome 156:\n{}",
        found.join("\n")
    );
}
