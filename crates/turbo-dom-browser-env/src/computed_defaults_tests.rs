//! turbo-surf#36 items 1-2: `getComputedStyle` reports Chrome's computed `display` / `position`
//! / `visibility` for properties no author rule sets (UA display per element, blockification,
//! `[hidden]`, dialog / popover positions, CSS-wide keywords), and the document carries no
//! injected `<title>`. Checked against the Chrome 156 control (`computed_defaults_probe.js` +
//! `computed_defaults_cases.json` -> `computed_defaults_chrome156.json`, recorded by
//! scripts/select-element-chrome-control.mjs), every case and section. Case `dialog-top-layer`
//! (turbo-dom#30) covers modal dialogs, the top layer and `::backdrop`; `pseudo-elements` and
//! `pseudo-ua` (turbo-dom#47) the `pseudoElt` argument and the UA-styled pseudo-elements;
//! `form-control-fonts` (turbo-dom#61) form control fonts; `fullscreen` (turbo-dom#58, async,
//! with simulated user activation) the fullscreen top layer.
use super::*;
use sha2::{Digest, Sha256};

const PROBE: &str = include_str!("computed_defaults_probe.js");
const CASES: &[u8] = include_bytes!("computed_defaults_cases.json");
const CONTROL: &str = include_str!("computed_defaults_chrome156.json");

/// The probe page's user activation: `__cdpGesture()` (the collector's trusted CDP click)
/// sets `__probeActivation`, which consuming APIs clear.
fn probe_activation(
    scope: &mut v8::PinScope,
    document: v8::Local<v8::Object>,
    query: ActivationQuery,
) -> bool {
    let context = document.get_creation_context(scope).expect("page document");
    let global = context.global(scope);
    let key = v8::String::new(scope, "__probeActivation").unwrap();
    let active = global
        .get(scope, key.into())
        .is_some_and(|value| value.is_true());
    match query {
        ActivationQuery::Transient | ActivationQuery::HistoryAction => active,
        ActivationQuery::ConsumeTransient | ActivationQuery::ConsumeHistoryAction => {
            let inactive = v8::Boolean::new(scope, false);
            global.set(scope, key.into(), inactive.into());
            true
        }
        ActivationQuery::CrossOriginFrame => false,
        ActivationQuery::FullscreenAllowed => true,
    }
}

/// Run the probe over `html`; an async case (`fullscreen`) settles over a FIFO task queue
/// standing in for timers, with `__cdpGesture()` granting user activation.
fn probe(html: &str) -> serde_json::Value {
    run(html, PROBE)
}

/// Run `source` (an expression, possibly a promise) in `html` with the probe harness.
fn run(html: &str, source: &str) -> serde_json::Value {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_independent_html(scope, html);
    set_activation_host(context, probe_activation);
    run_js(
        scope,
        "globalThis.innerWidth=1024;globalThis.innerHeight=768;\
         globalThis.__tasks=[];globalThis.setTimeout=callback=>{__tasks.push(callback);return __tasks.length};\
         globalThis.__cdpGesture=()=>{globalThis.__probeActivation=true;__tasks.push(()=>__cdpGestureDone());};",
    );
    capture_popover_scheduler(scope);
    let code = format!(
        "Promise.resolve({}).then(JSON.stringify)",
        source.trim().trim_end_matches(';')
    );
    let code = v8::String::new(scope, &code).unwrap();
    v8::tc_scope!(let scope, scope);
    let promise = v8::Script::compile(scope, code, None)
        .and_then(|script| script.run(scope))
        .and_then(|value| v8::Local::<v8::Promise>::try_from(value).ok())
        .unwrap_or_else(|| panic!("{}", scope.exception().unwrap().to_rust_string_lossy(scope)));
    let step = v8::String::new(scope, "__tasks.length&&__tasks.shift()()").unwrap();
    let step = v8::Script::compile(scope, step, None).unwrap();
    for _ in 0..100_000 {
        scope.perform_microtask_checkpoint();
        if promise.state() != v8::PromiseState::Pending {
            break;
        }
        step.run(scope);
    }
    let value = promise.result(scope).to_rust_string_lossy(scope);
    assert_eq!(promise.state(), v8::PromiseState::Fulfilled, "{value}");
    reset();
    serde_json::from_str(&value).unwrap()
}

/// Flatten `value` into `path = json` rows so a mismatch names the element and property
/// (array items by index).
fn rows(prefix: &str, value: &serde_json::Value, out: &mut Vec<(String, String)>) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, value) in map {
                rows(&format!("{prefix}.{key}"), value, out);
            }
        }
        serde_json::Value::Array(items) => {
            out.push((format!("{prefix}.length"), items.len().to_string()));
            for (index, value) in items.iter().enumerate() {
                rows(&format!("{prefix}.{index}"), value, out);
            }
        }
        _ => out.push((prefix.to_owned(), value.to_string())),
    }
}

/// Every control row of the cases `select` keeps, as `path: binding …, chrome …` mismatches.
fn mismatches(select: impl Fn(&str) -> bool) -> Vec<String> {
    let control: serde_json::Value = serde_json::from_str(CONTROL).unwrap();
    assert!(control["browser"]
        .as_str()
        .unwrap()
        .starts_with("Chrome/156."));
    assert_eq!(control["errors"], serde_json::json!([]));
    assert_eq!(
        control["probeSha256"],
        format!("{:x}", Sha256::digest(PROBE.as_bytes()))
    );
    assert_eq!(
        control["casesSha256"],
        format!("{:x}", Sha256::digest(CASES))
    );
    let cases: serde_json::Value = serde_json::from_slice(CASES).unwrap();
    let mut found = Vec::new();
    for (index, case) in cases.as_array().unwrap().iter().enumerate() {
        let id = case["id"].as_str().unwrap();
        let expected = &control["result"][index];
        assert_eq!(expected["id"], case["id"]);
        if !select(id) {
            continue;
        }
        let actual = probe(case["html"].as_str().unwrap());
        let (mut want, mut got) = (Vec::new(), Vec::new());
        rows(id, &expected["result"], &mut want);
        rows(id, &actual, &mut got);
        let got: std::collections::BTreeMap<_, _> = got.into_iter().collect();
        found.extend(want.into_iter().filter_map(|(path, value)| {
            let binding = got.get(&path).map_or("<missing>", String::as_str);
            (binding != value).then(|| format!("{path}: binding {binding}, chrome {value}"))
        }));
    }
    found
}

#[test]
fn computed_defaults_and_head_match_chrome156_control() {
    let found = mismatches(|id| {
        !matches!(
            id,
            "dialog-top-layer"
                | "pseudo-elements"
                | "pseudo-ua-forms"
                | "pseudo-ua"
                | "form-control-fonts"
                | "fullscreen"
        )
    });
    assert!(
        found.is_empty(),
        "{} mismatches:\n{}",
        found.len(),
        found.join("\n")
    );
}

/// turbo-dom#30: show() vs showModal() vs close(), `:modal`, the top-layer position rule and
/// `::backdrop` reads (case `dialog-top-layer`).
#[test]
fn dialog_top_layer_matches_chrome156_control() {
    let found = mismatches(|id| id == "dialog-top-layer");
    assert!(
        found.is_empty(),
        "{} mismatches:\n{}",
        found.len(),
        found.join("\n")
    );
}

/// turbo-dom#47: `getComputedStyle(el, pseudoElt)` — `::before` / `::after` (legacy and bare
/// forms, escapes, comments), invalid pseudo-elements (empty declaration), strings without a
/// leading colon (the element), disconnected elements, live reads (case `pseudo-elements`).
#[test]
fn pseudo_elements_match_chrome156_control() {
    let found = mismatches(|id| id == "pseudo-elements");
    assert!(
        found.is_empty(),
        "{} mismatches:\n{}",
        found.len(),
        found.join("\n")
    );
}

/// turbo-dom#47: the UA-styled pseudo-elements (`::marker`, `::placeholder`, `::first-line`,
/// `::first-letter`, `::selection` and the other highlights, `::file-selector-button`,
/// `::details-content`, `::checkmark`, `::picker-icon`, `::picker(select)`,
/// `::scroll-marker-group`, `::column`, `::view-transition*`, `::-webkit-scrollbar*`,
/// `::scroll-button()`): Chrome's UA values, the author properties each accepts, inheritance
/// (highlights from the parent's highlight), and the forms that read blank or the element
/// (case `pseudo-ua`).
#[test]
fn pseudo_element_ua_styles_match_chrome156_control() {
    let found = mismatches(|id| id == "pseudo-ua");
    assert!(
        found.is_empty(),
        "{} mismatches:\n{}",
        found.len(),
        found.join("\n")
    );
}

/// turbo-dom#47: every `pseudoElt` form Chrome 156 accepts — computed, blank (`::scroll-marker`,
/// invalid arguments, pseudo-classes after the pseudo-element) or the element's own style
/// (`::cue`, unknown `-webkit-` / `-internal-` names) — on a div, a list item and an input
/// (case `pseudo-ua-forms`).
#[test]
fn pseudo_element_forms_match_chrome156_control() {
    let found = mismatches(|id| id == "pseudo-ua-forms");
    assert!(
        found.is_empty(),
        "{} mismatches:\n{}",
        found.len(),
        found.join("\n")
    );
}

/// turbo-dom#61: input (every type), select, textarea and button compute html.css's
/// `font: -webkit-small-control` (13.3333px Arial; monospace for textarea and the date / time
/// inputs), optgroup is bolder, author rules win (case `form-control-fonts`).
#[test]
fn form_control_fonts_match_chrome156_control() {
    let found = mismatches(|id| id == "form-control-fonts");
    assert!(
        found.is_empty(),
        "{} mismatches:\n{}",
        found.len(),
        found.join("\n")
    );
}

/// turbo-dom#58: requestFullscreen() / exitFullscreen() with and without user activation,
/// the fullscreen stack, `:fullscreen` / `:modal` / `:-webkit-full-screen(-ancestor)`, the
/// `:not(:root):fullscreen` UA values, the black fixed `::backdrop`, fullscreenchange /
/// fullscreenerror, the root element, removal, a dialog, a shadow tree (case `fullscreen`).
#[test]
fn fullscreen_matches_chrome156_control() {
    let found = mismatches(|id| id == "fullscreen");
    assert!(
        found.is_empty(),
        "{} mismatches:\n{}",
        found.len(),
        found.join("\n")
    );
}

/// turbo-test#125: a reused computed style follows the viewport: the fullscreen element's width /
/// height are the viewport's, so a resize changes them although the document did not change.
#[test]
fn computed_style_reuse_follows_viewport_changes() {
    let value = run(
        "<!doctype html><body><div id=f>f</div></body>",
        r"(async () => {
          const f = document.getElementById('f'), retained = getComputedStyle(f), rows = [];
          const read = () => rows.push([getComputedStyle(f).width, retained.width, retained.height]);
          read();
          await new Promise((done) => { globalThis.__cdpGestureDone = done; __cdpGesture('click'); });
          await f.requestFullscreen();
          read(); read();
          innerWidth = 800; innerHeight = 600; read();
          await document.exitFullscreen();
          read();
          return rows;
        })()",
    );
    assert_eq!(
        value,
        serde_json::json!([
            ["", "", ""],
            ["1024px", "1024px", "768px"],
            ["1024px", "1024px", "768px"],
            ["800px", "800px", "600px"],
            ["", "", ""]
        ])
    );
}

/// The stand-in embedder answers from `__probeActivation`; consuming clears it.
#[test]
fn probe_activation_answers_from_the_gesture_flag() {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, "<!doctype html><body>");
    let code = v8::String::new(scope, "globalThis.__probeActivation = true; document").unwrap();
    let document = v8::Script::compile(scope, code, None)
        .unwrap()
        .run(scope)
        .unwrap();
    let document = v8::Local::<v8::Object>::try_from(document).unwrap();
    let mut answers = Vec::new();
    for query in [
        ActivationQuery::Transient,
        ActivationQuery::HistoryAction,
        ActivationQuery::CrossOriginFrame,
        ActivationQuery::FullscreenAllowed,
        ActivationQuery::ConsumeTransient,
        ActivationQuery::Transient,
        ActivationQuery::ConsumeHistoryAction,
    ] {
        answers.push(probe_activation(scope, document, query));
    }
    assert_eq!(answers, [true, true, false, true, true, false, true]);
    reset();
}
