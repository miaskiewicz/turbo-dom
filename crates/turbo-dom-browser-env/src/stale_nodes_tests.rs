//! Node wrappers that outlive their tree (turbo-test#204).
//!
//! A host that reuses one isolate across test files keeps module-level JS state between them
//! (vitest `isolate: false`): emotion's style sheet, for one, caches the `<style>` elements it
//! inserted. When the host resets the per-thread DOM and installs a fresh document, those
//! wrappers still carry a handle into the old tree. Handles are tree-local indices, so the old
//! handle either aliases an unrelated node of the new tree or lies past its end, where a read
//! indexed out of bounds and aborted the process from inside a V8 callback.
//!
//! A wrapper from a discarded tree must behave like a node of a detached realm in Chrome: every
//! native access throws `TypeError: Illegal invocation`, and nothing reaches the new tree.
use super::*;
use crate::browser_env::crate_surface::harness_tests::thrown;

fn run(scope: &mut v8::PinScope, code: &str) -> String {
    v8::tc_scope!(let scope, scope);
    let source = v8::String::new(scope, code).unwrap();
    match v8::Script::compile(scope, source, None).and_then(|script| script.run(scope)) {
        Some(value) => value.to_rust_string_lossy(scope),
        None => format!("threw {}", thrown(scope.exception(), scope)),
    }
}

/// Every native read a stale wrapper can reach, each reported as its value or its error.
const STALE_READS: &str = r"
  (() => {
    const out = {};
    const read = (name, f) => { try { out[name] = String(f()); } catch (e) { out[name] = e.name + ': ' + e.message; } };
    const get = (proto, name, node) => Object.getOwnPropertyDescriptor(proto, name).get.call(node);
    for (const [label, node] of Object.entries(globalThis.__kept)) {
      read(label + '.nodeType', () => get(Node.prototype, 'nodeType', node));
      read(label + '.nodeName', () => get(Node.prototype, 'nodeName', node));
      read(label + '.parentNode', () => get(Node.prototype, 'parentNode', node));
      read(label + '.textContent', () => get(Node.prototype, 'textContent', node));
      read(label + '.tagName', () => get(Element.prototype, 'tagName', node));
      read(label + '.id', () => get(Element.prototype, 'id', node));
    }
    return JSON.stringify(out);
  })()
";

fn first_file(scope: &mut v8::PinScope) {
    install(scope);
    // Many nodes, so the kept handles lie past the end of the next file's smaller tree; plus
    // the body, whose small handle lies inside it and would alias one of its nodes.
    let made = run(
        scope,
        r"
          for (let i = 0; i < 40; i++) document.body.appendChild(document.createElement('div'));
          const style = document.createElement('style');
          style.id = 'emotion';
          document.head.appendChild(style);
          const text = document.createTextNode('kept');
          style.appendChild(text);
          const inert = document.implementation.createHTMLDocument('');
          inert.body.innerHTML = '<p id=inert>x</p>';
          globalThis.__kept = { style, text, body: document.body, inert: inert.getElementById('inert') };
          'ok'
        ",
    );
    assert_eq!(made, "ok");
}

fn next_file(scope: &mut v8::PinScope) {
    reset();
    install(scope);
    // The next file's own inert document takes the id the previous file's had.
    let made = run(
        scope,
        r"
          globalThis.__fresh = document.implementation.createHTMLDocument('');
          __fresh.body.innerHTML = '<p id=other>y</p>';
          document.body.id = 'next';
          'ok'
        ",
    );
    assert_eq!(made, "ok");
}

fn stale_reads() -> serde_json::Value {
    tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    first_file(scope);
    next_file(scope);
    let reads = run(scope, STALE_READS);
    reset();
    serde_json::from_str(&reads).unwrap_or_else(|_| panic!("stale reads: {reads}"))
}

#[test]
fn wrappers_from_a_reset_tree_throw_illegal_invocation() {
    let reads = stale_reads();
    let reads = reads.as_object().unwrap();
    assert_eq!(reads.len(), 24);
    for (read, value) in reads {
        assert_eq!(
            value, "TypeError: Illegal invocation",
            "{read} reached the new tree"
        );
    }
}

#[test]
fn the_new_tree_is_untouched_by_stale_wrappers() {
    tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    first_file(scope);
    next_file(scope);
    let result = run(
        scope,
        r"
          const set = (proto, name, node, value) => { try { Object.getOwnPropertyDescriptor(proto, name).set.call(node, value); } catch (e) { return e.name; } return 'set'; };
          const k = globalThis.__kept;
          [set(Element.prototype, 'id', k.body, 'clobbered'),
           set(Node.prototype, 'textContent', k.text, 'clobbered'),
           set(Element.prototype, 'id', k.inert, 'clobbered'),
           document.body.id,
           document.documentElement.outerHTML.includes('clobbered'),
           __fresh.getElementById('other') !== null,
           __fresh.documentElement.outerHTML.includes('clobbered')].join()
        ",
    );
    reset();
    assert_eq!(
        result,
        "TypeError,TypeError,TypeError,next,false,true,false"
    );
}

#[test]
fn a_handle_past_the_end_of_its_tree_throws_instead_of_aborting() {
    tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install(scope);
    assert_eq!(
        run(
            scope,
            "globalThis.__node = document.createElement('div'); 'ok'"
        ),
        "ok"
    );
    let global = context.global(scope);
    let key = v8::String::new(scope, "__node").unwrap();
    let node = global.get(scope, key.into()).unwrap();
    let node = v8::Local::<v8::Object>::try_from(node).unwrap();
    // A handle this tree never minted, in a wrapper that otherwise belongs to it.
    let forged = v8::Integer::new_from_unsigned(scope, 1_000_000);
    node.set_internal_field(0, forged.into());
    let reads = run(
        scope,
        r"
          const get = (proto, name) => { try { return String(Object.getOwnPropertyDescriptor(proto, name).get.call(__node)); } catch (e) { return e.name + ': ' + e.message; } };
          [get(Node.prototype, 'nodeType'), get(Element.prototype, 'tagName'), get(Node.prototype, 'parentNode')].join('|')
        ",
    );
    reset();
    assert_eq!(
        reads,
        "TypeError: Illegal invocation|TypeError: Illegal invocation|TypeError: Illegal invocation"
    );
}
