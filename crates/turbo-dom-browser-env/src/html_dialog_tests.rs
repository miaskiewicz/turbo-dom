use super::*;
use sha2::{Digest, Sha256};

/// Run an async committed probe against the binding, draining one FIFO timer queue that the
/// private popover/dialog scheduler and the probe's own setTimeout/requestAnimationFrame share,
/// and return the resolved JSON value.
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

/// The control was captured by running the same probe in a fresh headless Chrome 156 profile
/// through scripts/invokers-chrome-control.mjs (self-contained CDP collector). Every differing
/// case is reported by section and name.
#[test]
fn dialog_behavior_matches_chrome_156() {
    let probe = include_str!("html_dialog_probe.js");
    let control: serde_json::Value =
        serde_json::from_str(include_str!("html_dialog_chrome156.json")).unwrap();
    assert_eq!(control["browser"], "Chrome/156.0.8078.47");
    assert_eq!(
        control["probeSha256"],
        format!("{:x}", Sha256::digest(probe.as_bytes()))
    );
    let actual = run_probe(probe, "");
    let mut differences = Vec::new();
    for (section, cases) in control["result"].as_object().unwrap() {
        let cases = cases.as_object().unwrap();
        for (case, expected) in cases {
            if &actual[section][case] != expected {
                differences.push(format!(
                    "{section}.{case}\n  binding: {}\n  chrome:  {expected}",
                    actual[section][case]
                ));
            }
        }
        if actual[section].as_object().map(|object| object.len()) != Some(cases.len()) {
            differences.push(format!("{section}: case set differs"));
        }
    }
    assert!(differences.is_empty(), "{}", differences.join("\n"));
    assert_eq!(actual, control["result"]);
}

/// Dialog steps run on privately captured constructors, attribute methods, focus and
/// scheduler: guest replacements and Object.prototype pollution cannot forge, suppress or
/// reshape the trusted lifecycle events.
#[test]
fn dialog_steps_ignore_guest_replacements_of_their_primitives() {
    let actual = run_probe(
        r#"(async () => {
          const d = document.createElement('dialog'), pre = document.createElement('button'), rows = [];
          document.body.append(pre, d); OriginalFocus.call(pre);
          for (const type of ['beforetoggle', 'toggle', 'cancel', 'close']) d.addEventListener(type, event => rows.push([type, event.isTrusted, event.bubbles, event.cancelable, event.constructor === OriginalToggle || event.constructor === OriginalEvent]));
          d.showModal(); d.requestClose('r');
          await new Promise(resolve => OriginalTimeout(resolve)); await new Promise(resolve => OriginalTimeout(resolve));
          return [rows, d.open, d.returnValue, document.activeElement === pre, d.matches(':modal')];
        })()"#,
        "globalThis.OriginalToggle=ToggleEvent;globalThis.OriginalEvent=Event;globalThis.OriginalTimeout=setTimeout;globalThis.OriginalFocus=HTMLElement.prototype.focus;\
         Object.defineProperty(Object.prototype,'bubbles',{get(){return true},configurable:true});\
         globalThis.ToggleEvent=function(){throw new Error('guest ToggleEvent')};Event.prototype.preventDefault=()=>{throw new Error('guest preventDefault')};\
         globalThis.setTimeout=()=>{throw new Error('guest timer')};HTMLElement.prototype.focus=()=>{throw new Error('guest focus')};\
         Element.prototype.setAttributeNS=Element.prototype.removeAttributeNS=()=>{throw new Error('guest attributes')};",
    );
    assert_eq!(
        actual,
        serde_json::json!([
            [
                ["beforetoggle", true, false, true, true],
                ["cancel", true, false, true, true],
                ["beforetoggle", true, false, false, true],
                ["toggle", true, false, false, true],
                ["close", true, false, false, true]
            ],
            false,
            "r",
            true,
            false
        ])
    );
}

/// Each document keeps its own modal stack: a modal dialog in one context neither blocks focus
/// nor matches `:modal` in another, and removal clears only its own document's state.
#[test]
fn modal_state_is_owned_by_each_document() {
    let actual = run_probe(
        r#"(async () => {
          const other = document.implementation.createHTMLDocument(''), d = document.createElement('dialog'), inner = other.createElement('dialog');
          document.body.append(d); other.body.append(inner); inner.show(); d.showModal();
          const rows = [d.matches(':modal'), inner.matches(':modal'), other.querySelector(':modal'), document.querySelector(':open') === d, other.querySelector(':open') === inner];
          d.remove(); rows.push(d.matches(':modal'), d.open, document.querySelector(':modal'));
          return rows;
        })()"#,
        "",
    );
    assert_eq!(
        actual,
        serde_json::json!([true, false, null, true, true, false, true, null])
    );
}
