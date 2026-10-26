use super::*;
use sha2::{Digest, Sha256};

/// A test-only stand-in for the embedder's trusted input driver: guest-visible here, unlike
/// the real driver, so probes can press the mouse pointer and process capture inline.
fn test_host_pointer<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    let command = args.get(0).to_rust_string_lossy(scope);
    let document = {
        let global = scope.get_current_context().global(scope);
        let key = v8::String::new(scope, "document").unwrap();
        global
            .get(scope, key.into())
            .unwrap()
            .to_object(scope)
            .unwrap()
    };
    let value = host_pointer(scope, document, &command);
    rv.set(value);
}

fn activated(_: v8::Local<v8::Context>) -> bool {
    true
}
fn unrestricted(_: v8::Local<v8::Context>) -> bool {
    false
}

/// Run an async probe against the binding with one FIFO timer queue shared by the private
/// scheduler and the probe, optionally under an embedder pointer lock policy.
fn run_probe(probe: &str, prelude: &str, policy: Option<PointerLockPolicy>) -> serde_json::Value {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(
        scope,
        "<!doctype html><html><head></head><body></body></html>",
    );
    if let Some(policy) = policy {
        scope
            .get_current_context()
            .set_slot(std::rc::Rc::new(policy));
        let global = scope.get_current_context().global(scope);
        bind_method(scope, global, "__hostPointer", test_host_pointer);
    }
    run_js(
        scope,
        "globalThis.__tasks=[];globalThis.setTimeout=callback=>{__tasks.push(callback);return __tasks.length};",
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

/// The control ran the same probe in a fresh headless Chrome 156 profile through
/// scripts/invokers-chrome-control.mjs: no pressed button and no user activation, so every
/// capture call is a refusal or a no-op and every lock request is refused.
#[test]
fn pointer_members_match_chrome_156() {
    let probe = include_str!("pointer_events_probe.js");
    let control: serde_json::Value =
        serde_json::from_str(include_str!("pointer_events_chrome156.json")).unwrap();
    assert_eq!(control["browser"], "Chrome/156.0.8078.47");
    assert_eq!(
        control["probeSha256"],
        format!("{:x}", Sha256::digest(probe.as_bytes()))
    );
    let actual = run_probe(probe, "", None);
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

/// The driver's state machine: an override set while pressed is only pending until the next
/// processing, which reports the old override lost (to the document once disconnected) and
/// the new one got; removal drops a pending override; the implicit release clears it.
#[test]
fn host_driver_processes_pending_capture_like_chrome() {
    let actual = run_probe(
        r#"(async () => {
          const [a, b, c] = ['a', 'b', 'c'].map(id => { const node = document.createElement('div'); node.id = id; document.body.append(node); return node; });
          const name = value => value === null ? null : value === document ? '#document' : value.id;
          const process = () => __hostPointer('process').map(name);
          const rows = [];
          a.setPointerCapture(1);
          rows.push(['unpressed', a.hasPointerCapture(1), process()]);
          __hostPointer('press');
          a.setPointerCapture(1);
          rows.push(['pending', a.hasPointerCapture(1), name(__hostPointer('capture')), process(), name(__hostPointer('capture')), process()]);
          b.setPointerCapture(1); a.releasePointerCapture(1);
          rows.push(['switch', a.hasPointerCapture(1), b.hasPointerCapture(1), process()]);
          c.setPointerCapture(1); c.remove();
          rows.push(['removed pending', c.hasPointerCapture(1), process()]);
          b.setPointerCapture(1); process(); b.remove();
          rows.push(['removed active', b.hasPointerCapture(1), name(__hostPointer('capture')), process(), name(__hostPointer('capture'))]);
          document.body.append(b);
          a.setPointerCapture(1); process(); __hostPointer('release');
          b.setPointerCapture(1);
          rows.push(['released', a.hasPointerCapture(1), b.hasPointerCapture(1)]);
          __hostPointer('implicit');
          rows.push(['implicit', a.hasPointerCapture(1), process(), process()]);
          return rows;
        })()"#,
        "",
        Some(PointerLockPolicy {
            activation: activated,
            sandboxed: unrestricted,
        }),
    );
    assert_eq!(
        actual,
        serde_json::json!([
            ["unpressed", false, [null, null]],
            ["pending", true, null, [null, "a"], "a", [null, null]],
            ["switch", false, true, ["a", "b"]],
            ["removed pending", false, ["b", null]],
            ["removed active", false, "b", ["#document", null], null],
            ["released", true, false],
            ["implicit", false, ["a", null], [null, null]]
        ])
    );
}

/// With embedder activation a request is granted in a later task: it resolves, queues a
/// trusted pointerlockchange, retargets into each tree scope, refuses pointer capture, and
/// a removed or exited lock clears synchronously with a queued change event.
#[test]
fn activated_pointer_lock_follows_chrome_controller_steps() {
    let actual = run_probe(
        r#"(async () => {
          const tick = () => new Promise(resolve => setTimeout(resolve, 0));
          const settle = async () => { for (let index = 0; index < 10; index++) await tick(); };
          const log = [];
          const name = value => value === null ? 'null' : value.id || value.nodeName;
          for (const type of ['pointerlockchange', 'pointerlockerror']) document.addEventListener(type, event => log.push([type, event.isTrusted, event.bubbles, name(document.pointerLockElement)].join(' ')));
          const [a, b] = ['a', 'b'].map(id => { const node = document.createElement('div'); node.id = id; document.body.append(node); return node; });
          const host = document.createElement('div'); host.id = 'host'; document.body.append(host);
          const root = host.attachShadow({ mode: 'open' }); root.innerHTML = '<p id=inner></p>';
          const track = (label, promise) => promise.then(value => log.push(label + ' resolved ' + value), error => log.push(label + ' rejected ' + error.name));
          __hostPointer('press'); a.setPointerCapture(1);
          track('a', a.requestPointerLock());
          log.push('pending ' + name(document.pointerLockElement) + ' ' + a.hasPointerCapture(1));
          await settle();
          log.push('granted ' + name(document.pointerLockElement) + ' ' + name(__hostPointer('locked')) + ' capture ' + a.hasPointerCapture(1));
          try { b.setPointerCapture(1); } catch (error) { log.push('capture ' + error.name); }
          track('same', a.requestPointerLock()); await settle();
          track('option', a.requestPointerLock({ unadjustedMovement: true })); await settle();
          track('inner', root.firstChild.requestPointerLock({ unadjustedMovement: true })); await settle();
          log.push('scopes ' + name(document.pointerLockElement) + ' ' + name(root.pointerLockElement));
          host.remove();
          log.push('removed ' + name(document.pointerLockElement) + ' ' + name(__hostPointer('locked')));
          await settle();
          track('b', b.requestPointerLock()); await settle();
          document.exitPointerLock();
          log.push('exited ' + name(document.pointerLockElement));
          await settle();
          document.exitPointerLock(); await settle();
          return log;
        })()"#,
        "",
        Some(PointerLockPolicy {
            activation: activated,
            sandboxed: unrestricted,
        }),
    );
    assert_eq!(
        actual,
        serde_json::json!([
            "pending null true",
            "a resolved undefined",
            "pointerlockchange true false a",
            "granted a a capture false",
            "capture InvalidStateError",
            "same resolved undefined",
            "pointerlockchange true false a",
            "option resolved undefined",
            "inner resolved undefined",
            "pointerlockchange true false host",
            "scopes host inner",
            "removed null null",
            "pointerlockchange true false null",
            "b resolved undefined",
            "pointerlockchange true false b",
            "exited null",
            "pointerlockchange true false null"
        ])
    );
}

/// The sandboxed pointer lock flag refuses before any grant, with a queued pointerlockerror;
/// guest replacements of DOMException, Event and the timer cannot reshape the lock steps.
#[test]
fn sandboxed_and_guest_patched_pointer_lock_requests() {
    fn sandboxed(_: v8::Local<v8::Context>) -> bool {
        true
    }
    let actual = run_probe(
        r#"(async () => {
          const node = document.createElement('div'); document.body.append(node);
          const log = [];
          document.addEventListener('pointerlockerror', event => log.push(event.type + ' ' + event.isTrusted));
          const promise = node.requestPointerLock();
          try { await promise; log.push('resolved'); } catch (error) { log.push(error.name + ': ' + error.message); }
          await new Promise(resolve => OriginalTimeout(resolve)); await new Promise(resolve => OriginalTimeout(resolve));
          return [promise instanceof OriginalPromise, log];
        })()"#,
        "globalThis.OriginalPromise=Promise;globalThis.OriginalTimeout=setTimeout;\
         globalThis.DOMException=function(){throw new Error('guest DOMException')};globalThis.Event=function(){throw new Error('guest Event')};\
         globalThis.setTimeout=callback=>{throw new Error('guest timer')};",
        Some(PointerLockPolicy { activation: activated, sandboxed }),
    );
    assert_eq!(
        actual,
        serde_json::json!([
            true,
            [
                "SecurityError: Failed to execute 'requestPointerLock' on 'Element': Blocked pointer lock on an element because the element's frame is sandboxed and the 'allow-pointer-lock' permission is not set.",
                "pointerlockerror true"
            ]
        ])
    );
}
