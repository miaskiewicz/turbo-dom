use super::*;
use sha2::{Digest, Sha256};

/// Run an async committed probe against the binding with a FIFO task queue standing in for
/// setTimeout/requestAnimationFrame, and return the resolved JSON value.
fn run_probe(probe: &str, prelude: &str) -> serde_json::Value {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    // Pin the UA font to the macOS Chrome 156 control capture ("Times"/16px) so the
    // caret/text model's computed-font-dependent values match on every host.
    assert!(set_computed_font_defaults(context, "Times", 16.0));
    install_with_html(
        scope,
        "<!doctype html><html><head></head><body></body></html>",
    );
    run_js(
        scope,
        "globalThis.__tasks=[];globalThis.setTimeout=callback=>{__tasks.push(callback);return __tasks.length};globalThis.requestAnimationFrame=callback=>setTimeout(()=>callback(0));globalThis.queueMicrotask??=callback=>{Promise.resolve().then(callback)};",
    );
    capture_popover_scheduler(scope);
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

/// Collect every differing subtree as (path, chrome, binding), stopping at known-gap paths. Fixture
/// case lists are `[label, result]` pairs and are addressed by label.
fn differences(
    path: &str,
    chrome: &serde_json::Value,
    ours: &serde_json::Value,
    gaps: &[(&str, serde_json::Value, &str)],
    out: &mut Vec<(String, serde_json::Value, serde_json::Value)>,
) {
    use serde_json::Value;
    if gaps.iter().any(|gap| gap.0 == path) {
        if chrome != ours {
            out.push((path.to_string(), chrome.clone(), ours.clone()));
        }
        return;
    }
    match (chrome, ours) {
        (Value::Object(a), Value::Object(b)) if a.len() == b.len() => {
            for (key, value) in a {
                match b.get(key) {
                    Some(other) => differences(&format!("{path}/{key}"), value, other, gaps, out),
                    None => out.push((format!("{path}/{key}"), value.clone(), Value::Null)),
                }
            }
        }
        (Value::Array(a), Value::Array(b)) if a.len() == b.len() => {
            for (index, (x, y)) in a.iter().zip(b).enumerate() {
                let label = match (x.get(0), x.as_array().map(|row| row.len())) {
                    (Some(Value::String(label)), Some(2)) if path.ends_with("/cases") => {
                        label.clone()
                    }
                    _ => index.to_string(),
                };
                differences(&format!("{path}/{label}"), x, y, gaps, out);
            }
        }
        _ if chrome != ours => out.push((path.to_string(), chrome.clone(), ours.clone())),
        _ => {}
    }
}

/// Compare a probe with its Chrome 156 control. Each known gap is pinned to the binding's exact
/// current value, so a gap that closes (or regresses differently) fails until the pin is updated.
fn assert_matches_control(
    probe: &str,
    fixture: &str,
    known_gaps: &[(&str, serde_json::Value, &str)],
) {
    let control: serde_json::Value = serde_json::from_str(fixture).unwrap();
    assert_eq!(control["browser"], "Chrome/156.0.8078.47");
    assert_eq!(control["v8"], "15.6.75.19");
    assert_eq!(
        control["probeSha256"],
        format!("{:x}", Sha256::digest(probe.as_bytes()))
    );
    let actual = run_probe(probe, "");
    let mut found = Vec::new();
    differences("", &control["result"], &actual, known_gaps, &mut found);
    let mut unexplained = Vec::new();
    for (path, chrome, ours) in &found {
        match known_gaps.iter().find(|gap| gap.0 == path) {
            Some((_, pinned, why)) => assert_eq!(
                ours, pinned,
                "known gap {path} changed ({why})\n  chrome: {chrome}"
            ),
            None => unexplained.push(format!("{path}\n  chrome:  {chrome}\n  binding: {ours}")),
        }
    }
    for gap in known_gaps {
        assert!(
            found.iter().any(|(path, _, _)| path == gap.0),
            "known gap {} now matches Chrome: remove the pin",
            gap.0
        );
    }
    assert!(unexplained.is_empty(), "{}", unexplained.join("\n"));
}

/// execCommand / queryCommand* over a fixture matrix captured in a fresh headless Chrome 156 profile
/// with scripts/invokers-chrome-control.mjs: return values, the resulting DOM and selection, the
/// trusted event logs, query states and values, undo/redo coalescing and focused text controls.
#[test]
fn editing_matches_chrome_156() {
    assert_matches_control(
        include_str!("editing_probe.js"),
        include_str!("editing_chrome156.json"),
        &[
            (
                "/selectionDuringInput/1",
                serde_json::json!(2),
                "Blink's SelectionEditor shifts a caret at an insertion offset past inserted text before \
                 `input` fires; the binding's Selection range follows live-Range rules and stays put",
            ),
        ],
    );
}

/// EditContext, its event interfaces, HTMLElement.editContext and ariaNotify against Chrome 156.
#[test]
fn edit_context_matches_chrome_156() {
    assert_matches_control(
        include_str!("edit_context_probe.js"),
        include_str!("edit_context_chrome156.json"),
        &[
            (
                "/bounds/6",
                serde_json::json!(["ok", [["Object", null, null, null, null], ["Object", null, null, null, null]]]),
                "standalone turbo-test's DOMRect is the generic interface stub; the embedder's geometry \
                 DOMRect (turbo-surf geometry_backend) is resolved lazily and carries the copied values",
            ),
            (
                "/bounds/12",
                serde_json::json!("ok"),
                "same DOMRect stub: the stored copy has no readable x in standalone turbo-test",
            ),
        ],
    );
}

/// Replay trusted-input steps: `eval` steps run as global scripts (like CDP Runtime.evaluate) and
/// record their JSON completion value; `insertText` / `key` steps drive the embedder input entry.
fn run_input_steps(steps: &serde_json::Value) -> serde_json::Value {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    // Pin the UA font to the macOS Chrome 156 control capture ("Times"/16px) so the
    // caret/text model's computed-font-dependent values match on every host.
    assert!(set_computed_font_defaults(context, "Times", 16.0));
    install_with_html(
        scope,
        "<!doctype html><html><head></head><body></body></html>",
    );
    let mut results = Vec::new();
    for step in steps.as_array().unwrap() {
        if let Some(code) = step.get("eval").and_then(|code| code.as_str()) {
            v8::tc_scope!(let tc, scope);
            let source = v8::String::new(tc, code).unwrap();
            let value = v8::Script::compile(tc, source, None)
                .and_then(|script| script.run(tc))
                .unwrap_or_else(|| {
                    panic!(
                        "step threw: {:?}",
                        tc.exception().map(|error| error.to_rust_string_lossy(tc))
                    )
                });
            let json = v8::json::stringify(tc, value)
                .unwrap()
                .to_rust_string_lossy(tc);
            results.push(serde_json::from_str::<serde_json::Value>(&json).unwrap());
            continue;
        }
        let global = scope.get_current_context().global(scope);
        let key = v8::String::new(scope, "document").unwrap();
        let document = global
            .get(scope, key.into())
            .and_then(|value| value.to_object(scope))
            .unwrap();
        match (
            step.get("insertText"),
            step.get("key").and_then(|key| key.as_str()),
        ) {
            (Some(text), _) => {
                editing_user_input(scope, document, "insertText", text.as_str());
            }
            (None, Some("Backspace")) => {
                editing_user_input(scope, document, "deleteContentBackward", None);
            }
            (None, Some("Delete")) => {
                editing_user_input(scope, document, "deleteContentForward", None);
            }
            _ => panic!("unknown step {step}"),
        }
    }
    reset();
    serde_json::Value::Array(results)
}

/// Trusted user input (Chrome's typing path: cancelable beforeinput with target ranges, the typing
/// command and input; EditContext textupdate) replayed against a headless Chrome 156 control driven
/// by real CDP input through scripts/editing-input-chrome-control.mjs. Keyboard events are the
/// embedder driver's own; Chrome's deprecated textInput event is not part of the comparison.
#[test]
fn trusted_input_matches_chrome_156() {
    let steps = include_str!("editing_input_steps.json");
    let control: serde_json::Value =
        serde_json::from_str(include_str!("editing_input_chrome156.json")).unwrap();
    assert_eq!(control["browser"], "Chrome/156.0.8078.47");
    assert_eq!(
        control["stepsSha256"],
        format!("{:x}", Sha256::digest(steps.as_bytes()))
    );
    let actual = run_input_steps(&serde_json::from_str(steps).unwrap());
    let mut found = Vec::new();
    differences("", &control["result"], &actual, &[], &mut found);
    let report: Vec<String> = found
        .iter()
        .map(|(path, chrome, ours)| format!("{path}\n  chrome:  {chrome}\n  binding: {ours}"))
        .collect();
    assert!(report.is_empty(), "{}", report.join("\n"));
}

/// Commands run on primordials captured at install: guest replacements of DOM, Selection, event and
/// prototype members cannot reshape the edit, suppress the trusted input event or forge its data.
#[test]
fn editing_ignores_guest_replacements_of_its_primitives() {
    let actual = run_probe(
        r#"(() => {
          const rows = [];
          document.body.innerHTML = '<div id=e contenteditable>hello</div>';
          const e = document.body.firstChild, t = e.firstChild;
          getSelection().setBaseAndExtent(t, 2, t, 4);
          e.addEventListener('input', ev => rows.push([ev.type, ev.isTrusted, ev.inputType, ev.data, ev.constructor === OriginalInput]));
          Node.prototype.insertBefore = Node.prototype.removeChild = () => { throw new Error('guest tree'); };
          Element.prototype.setAttribute = Element.prototype.removeAttribute = () => { throw new Error('guest attributes'); };
          CharacterData.prototype.insertData = CharacterData.prototype.deleteData = () => { throw new Error('guest text'); };
          globalThis.InputEvent = function () { throw new Error('guest InputEvent'); };
          Object.defineProperty(Object.prototype, 'inputType', { get() { return 'forged'; }, configurable: true });
          Array.prototype[Symbol.iterator] = function () { throw new Error('guest iterator'); };
          Array.prototype.splice = Array.prototype.slice = () => { throw new Error('guest array'); };
          Document.prototype.createElement = () => { throw new Error('guest createElement'); };
          getSelection().setBaseAndExtent = () => { throw new Error('guest selection'); };
          rows.push(document.execCommand('bold'), e.innerHTML);
          rows.push(document.execCommand('insertText', false, 'X'), e.innerHTML);
          rows.push(document.execCommand('undo'), document.execCommand('undo'), e.innerHTML);
          delete Object.prototype.inputType;
          return rows;
        })()"#,
        "globalThis.OriginalInput=InputEvent;",
    );
    assert_eq!(
        actual,
        serde_json::json!([
            ["input", true, "formatBold", null, true],
            true,
            "he<b>ll</b>o",
            ["input", true, "insertText", "X", true],
            true,
            "he<b>X</b>o",
            ["input", true, "historyUndo", null, true],
            ["input", true, "historyUndo", null, true],
            true,
            true,
            "hello"
        ])
    );
}

/// Each document keeps its own editor state: the undo stack, styleWithCSS and the default paragraph
/// separator of a fresh context start from Chrome's defaults.
#[test]
fn editor_state_is_owned_by_each_context() {
    let first = run_probe(
        "(() => { document.body.innerHTML = '<div contenteditable>ab</div>'; const t = document.body.firstChild.firstChild; getSelection().collapse(t, 1); document.execCommand('styleWithCSS', false, true); document.execCommand('defaultParagraphSeparator', false, 'p'); document.execCommand('insertText', false, 'X'); return [document.queryCommandEnabled('undo'), document.queryCommandState('styleWithCSS'), document.queryCommandValue('defaultParagraphSeparator')]; })()",
        "",
    );
    let second = run_probe(
        "(() => [document.queryCommandEnabled('undo'), document.queryCommandState('styleWithCSS'), document.queryCommandValue('defaultParagraphSeparator')])()",
        "",
    );
    assert_eq!(first, serde_json::json!([true, true, "p"]));
    assert_eq!(second, serde_json::json!([false, false, "div"]));
}
