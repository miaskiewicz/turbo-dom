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
