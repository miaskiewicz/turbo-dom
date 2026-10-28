//! The binding's private natives (captured by its JS modules at install, or reachable on the
//! `globalThis[Symbol.for('ts')]` namespace) called with the wrong receivers / arguments:
//! each refuses with its documented error or an empty result, and still answers a valid call.
use crate::browser_env::*;

fn with_page(body: impl FnOnce(&mut v8::PinScope)) {
    crate::browser_env::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(
        scope,
        "<!doctype html><p id=p></p><input id=i type=checkbox>",
    );
    let global = context.global(scope);
    bind_method(scope, global, "__t_record", lookup_event_record);
    bind_method(scope, global, "__t_version", computed_style_version);
    bind_method(scope, global, "__t_checked", form_checked_native);
    bind_method(scope, global, "__t_resolve", native_resolve_document_url);
    body(scope);
    reset();
}

fn json(scope: &mut v8::PinScope, source: &str) -> String {
    let code = v8::String::new(
        scope,
        &format!(
            "JSON.stringify((() => {{ const ts = globalThis[Symbol.for('ts')]; const run = f => {{ try {{ const v = f(); return v === undefined ? 'undefined' : v; }} catch (e) {{ return [e.constructor.name, e.message]; }} }}; return {source}; }})())"
        ),
    )
    .unwrap();
    v8::Script::compile(scope, code, None)
        .unwrap()
        .run(scope)
        .unwrap()
        .to_rust_string_lossy(scope)
}

#[test]
fn native_function_and_named_object_factories_reject_bad_arguments() {
    with_page(|scope| {
        assert_eq!(
            json(
                scope,
                "[run(() => ts.makeNativeFunction(5)), run(() => ts.makeNativeFunction(null, 'x')), run(() => ts.makeNamedObject(5, () => {})), run(() => ts.makeNamedObject(null, 5)), run(() => typeof ts.makeNativeFunction(() => 1, 'one', 0)), run(() => ts.makeNativeFunction(() => 1, 'one', 0)())]"
            ),
            r#"[["TypeError","Expected a function"],["TypeError","Expected a function"],["TypeError","Expected a prototype and named-property dispatcher"],["TypeError","Expected a prototype and named-property dispatcher"],"function",1]"#
        );
    });
}

#[test]
fn event_record_lookup_answers_only_events() {
    with_page(|scope| {
        assert_eq!(
            json(
                scope,
                "[run(() => __t_record(5)), run(() => __t_record({})), run(() => __t_record(new Event('ping')).type)]"
            ),
            r#"["undefined","undefined","ping"]"#
        );
    });
}

#[test]
fn computed_style_version_answers_only_nodes_and_tracks_mutations() {
    with_page(|scope| {
        assert_eq!(
            json(
                scope,
                "(() => { const p = document.getElementById('p'); const before = __t_version(p); p.setAttribute('title', 't'); return [run(() => __t_version(5)), run(() => __t_version({})), typeof before, __t_version(p) > before]; })()"
            ),
            r#"["undefined","undefined","number",true]"#
        );
    });
}

#[test]
fn checked_state_native_brand_checks_its_input() {
    with_page(|scope| {
        assert_eq!(
            json(
                scope,
                "(() => { const input = document.getElementById('i'); return [run(() => __t_checked(0, 5)), run(() => __t_checked(0, {})), run(() => __t_checked(0, document.getElementById('p'))), __t_checked(0, input), __t_checked(1, input, true), __t_checked(0, input), input.checked]; })()"
            ),
            r#"[["TypeError","Illegal invocation"],["TypeError","Illegal invocation"],["TypeError","Illegal invocation"],false,true,true,true]"#
        );
    });
}

#[test]
fn document_url_resolution_needs_a_node() {
    with_page(|scope| {
        assert_eq!(
            json(
                scope,
                "[run(() => __t_resolve(5, 'a')), run(() => __t_resolve(document.getElementById('p'), 'https://example.test/x/../y'))]"
            ),
            r#"["undefined","https://example.test/y"]"#
        );
    });
}

#[test]
fn cascade_lookup_brand_checks_its_element() {
    with_page(|scope| {
        assert_eq!(
            json(
                scope,
                "[run(() => ts.cascadeProp.call({}, 'color')), run(() => ts.cascadeProp.call(document.createTextNode('t'), 'color')), ts.cascadeProp.call(document.getElementById('p'), 'color')]"
            ),
            r#"[["TypeError","Illegal invocation"],["TypeError","Illegal invocation"],""]"#
        );
    });
}

/// The EventTarget wrapper hands nodes to the native add / remove / dispatch functions
/// through Function.prototype.apply, which a page can patch to capture the natives and call
/// them on any receiver with any arguments: they refuse with TypeErrors, never abort.
#[test]
fn event_target_natives_survive_a_patched_apply() {
    with_page(|scope| {
        assert_eq!(
            json(
                scope,
                "(() => { const original = Function.prototype.apply, natives = []; Function.prototype.apply = function (receiver, args) { natives.push(this); return Reflect.apply(original, this, [receiver, args]); }; const p = document.getElementById('p'); try { p.addEventListener('x', () => {}); p.removeEventListener('x', () => {}); p.dispatchEvent(new Event('x')); } finally { Function.prototype.apply = original; } const [add, remove, dispatch] = natives; const proxy = new Proxy(p, {}); return [natives.length, run(() => add.call({}, 'x', () => {})), run(() => add.call(p, 'x')), run(() => add.call(proxy, 'x', () => {})), run(() => remove.call({}, 'x', () => {})), run(() => remove.call(p)), run(() => dispatch.call({}, new Event('x'))), run(() => dispatch.call(proxy, new Event('x'))), run(() => dispatch.call(p, new Event('y')))]; })()"
            ),
            r#"[3,["TypeError","Illegal invocation"],["TypeError","Event listener registration requires 2 arguments"],["TypeError","Illegal invocation"],["TypeError","Illegal invocation"],["TypeError","Event listener registration requires 2 arguments"],["TypeError","Illegal invocation"],["TypeError","Illegal invocation"],true]"#
        );
    });
}

/// Natives the binding's JS calls through the page-patchable Function.prototype.apply /
/// call: a page that patches them captures each native and can call it with a foreign
/// receiver, a Proxy, a text node, no arguments, a Symbol or a throwing toString. Every such
/// call ends in a TypeError / DOMException or a plain result, never an abort.
const CAPTURE: &str = "(() => { const oa = Function.prototype.apply, oc = Function.prototype.call, seen = []; const note = f => { if (!seen.includes(f)) seen.push(f); }; Function.prototype.apply = function (t, a) { note(this); return Reflect.apply(oa, this, [t, a]); }; Function.prototype.call = function (t, ...a) { note(this); return Reflect.apply(oa, this, [t, a]); }; try { const d = document; const e = d.createElement('div'); d.body.append(e); e.innerHTML = '<b id=x>t</b>'; e.querySelectorAll('b'); e.style.color = 'red'; getComputedStyle(e).color; e.setAttribute('onclick', 'void 0'); e.focus(); e.blur(); e.click(); e.remove(); } finally { Function.prototype.apply = oa; Function.prototype.call = oc; } return seen; })()";

const HOSTILE: &str = "(natives => { const names = ['__cascadeProp', '__noteInlineHandler', '__readAppliedInlineStyle', '__writeAppliedInlineStyle', 'blur', 'click', 'createElement', 'focus', 'hasAttribute', 'querySelectorAll']; const p = document.getElementById('p'), bad = { toString() { throw new RangeError('toString'); } }; const receivers = [['plain', {}], ['proxy', new Proxy(p, {})], ['text', document.createTextNode('t')], ['document', document], ['element', p]]; const argsets = [['none', []], ['symbol', [Symbol('s'), Symbol('s')]], ['throwing', [bad, bad]]]; const out = {}; const counts = {}; for (const f of natives) { if (!names.includes(f.name)) continue; const key = f.name + '#' + (counts[f.name] = (counts[f.name] || 0) + 1); out[key] = []; for (const [rn, r] of receivers) for (const [an, a] of argsets) { let v; try { const x = Reflect.apply(f, r, a); v = x === undefined ? 'undefined' : x === null ? 'null' : typeof x === 'object' ? 'object' : x; } catch (e) { v = [e.constructor.name, e.name, e.message]; } out[key].push(rn + '/' + an + ': ' + JSON.stringify(v)); } } return out; })";

#[test]
fn captured_natives_refuse_hostile_calls() {
    with_page(|scope| {
        let out: serde_json::Value =
            serde_json::from_str(&json(scope, &format!("{HOSTILE}({CAPTURE})"))).unwrap();
        let illegal = r#"["TypeError","TypeError","Illegal invocation"]"#;
        let symbol = r#"["TypeError","TypeError","Cannot convert a Symbol to a string"]"#;
        let symbol_value =
            r#"["TypeError","TypeError","Cannot convert a Symbol value to a string"]"#;
        let throwing = r#"["RangeError","RangeError","toString"]"#;
        // (native, its valid receiver, results there for no arguments / a Symbol / a throwing
        // toString); every other receiver is an Illegal invocation.
        let natives: &[(&str, &str, [&str; 3])] = &[
            (
                "createElement#1",
                "document",
                [r#""object""#, symbol, throwing],
            ),
            (
                "createElement#2",
                "document",
                [r#""object""#, symbol, throwing],
            ),
            (
                "querySelectorAll#1",
                "element",
                [r#""object""#, symbol, throwing],
            ),
            (
                "querySelectorAll#2",
                "document",
                [r#""object""#, symbol, throwing],
            ),
            (
                "__readAppliedInlineStyle#1",
                "element",
                [r#""""#, r#""""#, r#""""#],
            ),
            (
                "__writeAppliedInlineStyle#1",
                "element",
                [r#""undefined""#, symbol_value, throwing],
            ),
            (
                "__cascadeProp#1",
                "element",
                [r#""""#, symbol_value, throwing],
            ),
            ("focus#1", "element", [r#""undefined""#; 3]),
            ("blur#1", "element", [r#""undefined""#; 3]),
            ("click#1", "element", [r#""undefined""#; 3]),
            ("hasAttribute#1", "element", ["false", symbol, throwing]),
        ];
        let mut keys: Vec<_> = out.as_object().unwrap().keys().cloned().collect();
        keys.sort();
        let mut expected_keys: Vec<_> = natives
            .iter()
            .map(|native| native.0.to_string())
            .chain(["__noteInlineHandler#1".to_string()])
            .collect();
        expected_keys.sort();
        assert_eq!(keys, expected_keys);
        for (name, valid, results) in natives {
            let mut expected = Vec::new();
            for receiver in ["plain", "proxy", "text", "document", "element"] {
                for (index, args) in ["none", "symbol", "throwing"].iter().enumerate() {
                    let value = if receiver == *valid {
                        results[index]
                    } else {
                        illegal
                    };
                    expected.push(format!("{receiver}/{args}: {value}"));
                }
            }
            assert_eq!(out[*name], serde_json::json!(expected), "{name}");
        }
        // The inline-handler note ignores receivers that are not nodes and converts its
        // arguments only for nodes.
        let mut expected = Vec::new();
        for receiver in ["plain", "proxy", "text", "document", "element"] {
            for (args, value) in [
                ("none", r#""undefined""#),
                ("symbol", symbol_value),
                ("throwing", throwing),
            ] {
                let node = matches!(receiver, "text" | "document" | "element");
                let value = if node || args == "none" {
                    value
                } else {
                    r#""undefined""#
                };
                expected.push(format!("{receiver}/{args}: {value}"));
            }
        }
        assert_eq!(out["__noteInlineHandler#1"], serde_json::json!(expected));
    });
}

/// Natives the binding's JS wraps behind its own checks (captured with Reflect.apply, so a
/// page cannot reach them): called directly, each still refuses a foreign receiver or a
/// throwing / Symbol argument with a TypeError instead of trusting its wrapper.
#[test]
fn wrapped_natives_refuse_hostile_calls_directly() {
    with_page(|scope| {
        let global = scope.get_current_context().global(scope);
        bind_method(scope, global, "__t_createElementNS", doc_create_element_ns);
        bind_method(scope, global, "__t_createElement", doc_create_element);
        bind_method(scope, global, "__t_createTextNode", doc_create_text_node);
        bind_method(scope, global, "__t_getElementById", doc_get_element_by_id);
        bind_method(scope, global, "__t_querySelector", doc_query_selector);
        bind_method(scope, global, "__t_replaceChildren", el_replace_children);
        bind_method(
            scope,
            global,
            "__t_insertAdjacentElement",
            el_insert_adjacent_element,
        );
        bind_method(scope, global, "__t_splitText", el_split_text);
        bind_method(scope, global, "__t_attachShadow", el_attach_shadow);
        bind_method(scope, global, "__t_setAttribute", el_set_attribute);
        let out = json(
            scope,
            "(() => { const p = document.getElementById('p'), t = document.createTextNode('abc'), bad = { toString() { throw new RangeError('toString'); } }, sym = Symbol('s'); const at = (name, receiver, ...args) => { const r = run(() => globalThis['__t_' + name].call(receiver, ...args)); return r && typeof r === 'object' && !Array.isArray(r) ? 'object' : r; }; return { createElementNS: [at('createElementNS', p, null, 'a'), at('createElementNS', document, null), at('createElementNS', document, sym, 'a'), at('createElementNS', document, null, bad), at('createElementNS', document, 'urn:x', 'a')], createElement: [at('createElement', p, 'a'), at('createElement', document, bad), at('createElement', document, 'a')], createTextNode: [at('createTextNode', p, 'a'), at('createTextNode', document, sym), at('createTextNode', document, 'a')], getElementById: [at('getElementById', p, 'p'), at('getElementById', document, bad), at('getElementById', document, 'p')], querySelector: [at('querySelector', p, 'b'), at('querySelector', document, '\\\\70 ')], replaceChildren: [at('replaceChildren', t, 'x'), at('replaceChildren', document.createElement('i'), bad)], insertAdjacentElement: [at('insertAdjacentElement', t, 'beforeend', p), at('insertAdjacentElement', document.createElement('i'), 'beforeend', 'text')], splitText: [at('splitText', p, 1), at('splitText', t, sym), at('splitText', t, 9)], attachShadow: [at('attachShadow', t, { mode: 'open', slotAssignment: 'named' })], setAttribute: [at('setAttribute', t, 'a', 'b'), at('setAttribute', p, sym, 'b')] }; })()",
        );
        assert_eq!(
            out,
            r#"{"createElementNS":[["TypeError","Illegal invocation"],["TypeError","Not enough arguments"],["TypeError","Cannot convert a Symbol to a string"],["RangeError","toString"],"object"],"createElement":[["TypeError","Illegal invocation"],["RangeError","toString"],"object"],"createTextNode":[["TypeError","Illegal invocation"],["TypeError","Cannot convert a Symbol to a string"],"object"],"getElementById":[["TypeError","Illegal invocation"],["RangeError","toString"],"object"],"querySelector":[["TypeError","Illegal invocation"],"object"],"replaceChildren":[["TypeError","Illegal invocation"],["RangeError","toString"]],"insertAdjacentElement":[["TypeError","Illegal invocation"],["TypeError","The argument must be a native Node"]],"splitText":[["TypeError","Illegal invocation"],["TypeError","Cannot convert a Symbol value to a number"],["DOMException","CharacterData offset exceeds its length"]],"attachShadow":[["TypeError","Illegal invocation"]],"setAttribute":[["TypeError","Illegal invocation"],["TypeError","Cannot convert a Symbol to a string"]]}"#
        );
    });
}
