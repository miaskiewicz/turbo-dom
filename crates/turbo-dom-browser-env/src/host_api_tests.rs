//! Embedder entry points of `browser_env` (turbo-surf's native host): nonce hiding, host
//! element creation, the form submission base target, host-delivered event callbacks,
//! interface records and the native document census. Each is asserted through what page
//! script observes afterwards (Chrome's behaviour for the same host step).
use crate::browser_env::*;

/// Install `html` into a fresh context, run `body` with the scope, then reset.
fn with_page(html: &str, body: impl FnOnce(&mut v8::PinScope)) {
    crate::browser_env::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, html);
    body(scope);
    reset();
}

fn eval<'s>(scope: &mut v8::PinScope<'s, '_>, source: &str) -> v8::Local<'s, v8::Value> {
    let code = v8::String::new(scope, source).unwrap();
    v8::Script::compile(scope, code, None)
        .unwrap()
        .run(scope)
        .unwrap()
}

fn json(scope: &mut v8::PinScope, source: &str) -> String {
    let value = eval(scope, &format!("JSON.stringify({source})"));
    value.to_rust_string_lossy(scope)
}

fn object<'s>(scope: &mut v8::PinScope<'s, '_>, source: &str) -> v8::Local<'s, v8::Object> {
    v8::Local::<v8::Object>::try_from(eval(scope, source)).unwrap()
}

/// Chrome hides a parsed element's nonce: the content attribute reads "" while the IDL
/// attribute keeps the value; the host updates the internal slot when the attribute changes.
#[test]
fn nonce_hiding_keeps_the_internal_slot() {
    with_page(
        "<!doctype html><script id=s nonce=abc></script><style id=t></style>",
        |scope| {
            let script = object(scope, "globalThis.s = document.getElementById('s')");
            assert_eq!(node_nonce(scope, script).as_deref(), Some("abc"));
            hide_node_nonce(scope, script);
            assert_eq!(
                json(scope, "[s.getAttribute('nonce'), s.nonce, s.outerHTML]"),
                r#"["","abc","<script id=\"s\" nonce=\"\"></script>"]"#
            );
            update_node_nonce(scope, script, "xyz");
            assert_eq!(
                json(scope, "[s.nonce, s.getAttribute('nonce')]"),
                r#"["xyz",""]"#
            );
            assert_eq!(node_nonce(scope, script).as_deref(), Some("xyz"));
            update_node_nonce_units(scope, script, &[0x61, 0xd800, 0x62]);
            assert_eq!(
                json(scope, "[s.nonce.length, s.nonce.charCodeAt(1)]"),
                "[3,55296]"
            );
            // An element whose nonce lives only in its slot keeps no content attribute.
            let style = object(
                scope,
                "globalThis.t = document.getElementById('t'); t.nonce = 'slot'; t",
            );
            hide_node_nonce(scope, style);
            assert_eq!(
                json(scope, "[t.hasAttribute('nonce'), t.nonce]"),
                r#"[false,"slot"]"#
            );
            // Non-elements have no nonce; hiding one is a no-op.
            let text = object(scope, "document.createTextNode('x')");
            assert_eq!(node_nonce(scope, text), None);
            hide_node_nonce(scope, text);
            let plain = v8::Object::new(scope);
            assert_eq!(node_nonce(scope, plain), None);
            hide_node_nonce(scope, plain);
            update_node_nonce(scope, plain, "ignored");
            update_node_nonce_units(scope, plain, &[0x61]);
            assert_eq!(json(scope, "s.nonce"), r#""a\ud800b""#);
        },
    );
}

/// `create_document_element` is `document.createElement` for the host: HTML documents
/// create HTML elements, XML documents create namespace-less elements unless their
/// content type is XHTML; a non-document yields nothing.
#[test]
fn host_element_creation_follows_the_document_kind() {
    with_page("<!doctype html><p id=p></p>", |scope| {
        let document = object(scope, "document");
        let div = create_document_element(scope, document, "DIV").unwrap();
        let global = scope.get_current_context().global(scope);
        let key = v8::String::new(scope, "created").unwrap();
        global.set(scope, key.into(), div.into());
        assert_eq!(
            json(
                scope,
                "[created.localName, created.namespaceURI, created.ownerDocument === document, created.isConnected, created instanceof HTMLDivElement]"
            ),
            r#"["div","http://www.w3.org/1999/xhtml",true,false,true]"#
        );
        let xml = object(
            scope,
            "globalThis.xml = document.implementation.createDocument(null, null)",
        );
        let item = create_document_element(scope, xml, "Item").unwrap();
        let key = v8::String::new(scope, "item").unwrap();
        global.set(scope, key.into(), item.into());
        assert_eq!(
            json(
                scope,
                "[item.localName, item.namespaceURI, item.ownerDocument === xml]"
            ),
            r#"["Item",null,true]"#
        );
        let xhtml = object(
            scope,
            "globalThis.xhtml = document.implementation.createDocument('http://www.w3.org/1999/xhtml', 'html')",
        );
        let body = create_document_element(scope, xhtml, "body").unwrap();
        let key = v8::String::new(scope, "xbody").unwrap();
        global.set(scope, key.into(), body.into());
        assert_eq!(
            json(scope, "[xbody.namespaceURI, xbody.ownerDocument === xhtml]"),
            r#"["http://www.w3.org/1999/xhtml",true]"#
        );
        let p = object(scope, "document.getElementById('p')");
        assert!(create_document_element(scope, p, "div").is_none());
        let plain = v8::Object::new(scope);
        assert!(create_document_element(scope, plain, "div").is_none());
    });
}

/// The form submission target falls back to the first connected `<base target>`.
#[test]
fn base_target_is_the_first_connected_base_with_a_target() {
    with_page(
        "<!doctype html><head><base href=/x></head><form id=f></form>",
        |scope| {
            let form = object(scope, "document.getElementById('f')");
            assert_eq!(form_submission_base_target(scope, form), None);
            eval(scope, "globalThis.loose = document.createElement('base'); loose.setAttribute('target', 'loose')");
            assert_eq!(form_submission_base_target(scope, form), None);
            eval(scope, "document.head.append(loose)");
            assert_eq!(
                form_submission_base_target(scope, form).as_deref(),
                Some("loose")
            );
            eval(scope, "const b = document.createElement('base'); b.setAttribute('target', 'first'); document.head.prepend(b)");
            assert_eq!(
                form_submission_base_target(scope, form).as_deref(),
                Some("first")
            );
        },
    );
}

/// An engine-delivered callback on a non-DOM target (a worker, a media query list) sees the
/// event at its target, trusted, with a one-item path; leaving clears the dispatch state.
#[test]
fn event_callback_state_brackets_a_host_callback() {
    with_page("<!doctype html>", |scope| {
        let event = object(scope, "globalThis.ev = new Event('message'); ev");
        let target = object(scope, "globalThis.tgt = new EventTarget(); tgt");
        event_callback_state(scope, event, target, true);
        assert_eq!(
            json(
                scope,
                "[ev.target === tgt, ev.currentTarget === tgt, ev.eventPhase, ev.isTrusted, ev.composedPath().length, ev.composedPath()[0] === tgt]"
            ),
            "[true,true,2,true,1,true]"
        );
        eval(scope, "ev.stopPropagation(); ev.stopImmediatePropagation()");
        event_callback_state(scope, event, target, false);
        assert_eq!(
            json(
                scope,
                "[ev.target === tgt, ev.currentTarget, ev.eventPhase, ev.isTrusted, ev.composedPath().length]"
            ),
            "[true,null,0,true,0]"
        );
        // The stop flags are cleared, so the event can be delivered again.
        assert_eq!(
            json(
                scope,
                "(() => { let calls = 0; tgt.addEventListener('message', () => calls++); tgt.dispatchEvent(new Event('message')); return calls; })()"
            ),
            "1"
        );
        // A plain object is no event: nothing changes.
        let plain = object(scope, "globalThis.fake = {}; fake");
        event_callback_state(scope, plain, target, true);
        assert_eq!(json(scope, "Object.keys(fake)"), "[]");
    });
}

/// Host interface records are private to the host: page script never sees them.
#[test]
fn interface_records_are_host_private() {
    with_page("<!doctype html>", |scope| {
        let holder = object(scope, "globalThis.holder = {}; holder");
        assert!(get_interface_record(scope, holder).is_none());
        let record = v8::Integer::new(scope, 7);
        assert!(set_interface_record(scope, holder, record.into()));
        let read = get_interface_record(scope, holder).unwrap();
        assert_eq!(read.int32_value(scope), Some(7));
        assert_eq!(
            json(scope, "[Object.getOwnPropertyNames(holder).length, Object.getOwnPropertySymbols(holder).length, Reflect.ownKeys(holder).length]"),
            "[0,0,0]"
        );
    });
}

/// The census counts the current document and every other live native document.
#[test]
fn native_document_census() {
    crate::browser_env::tests::init_v8();
    reset();
    assert_eq!(native_document_count_for_testing(), 0);
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, "<!doctype html>");
    assert_eq!(native_document_count_for_testing(), 1);
    {
        let child = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, child);
        install_independent_html(scope, "<p>child</p>");
    }
    assert_eq!(native_document_count_for_testing(), 2);
    assert_eq!(sweep_collected_documents(), 0);
    reset();
    assert_eq!(native_document_count_for_testing(), 0);
}

/// The host's Trusted Types entry points: a sink input converts through the document's
/// policy (a typed value is unwrapped exactly, a string passes only while the policy allows
/// it), and the engine's code-generation check reads only genuine TrustedScript values.
#[test]
fn trusted_types_host_conversion() {
    with_page("<!doctype html><body></body>", |scope| {
        let document = object(scope, "document");
        let typed = eval(
            scope,
            "globalThis.typed = trustedTypes.createPolicy('host', { createScript: s => s + ';' }).createScript('a + 1'); typed",
        );
        let text = v8::String::new(scope, "b + 2").unwrap().into();
        let source = trusted_script_source(scope, typed).unwrap();
        assert_eq!(source.to_rust_string_lossy(scope), "a + 1;");
        assert!(trusted_script_source(scope, text).is_none());
        let html = eval(scope, "trustedTypes.emptyHTML");
        assert!(trusted_script_source(scope, html).is_none());
        let converted =
            trusted_type_string(scope, document, typed, "TrustedScript", "eval", "").unwrap();
        assert_eq!(converted.to_rust_string_lossy(scope), "a + 1;");
        let converted =
            trusted_type_string(scope, document, text, "TrustedScript", "eval", "").unwrap();
        assert_eq!(converted.to_rust_string_lossy(scope), "b + 2");
        // A policy that refuses string sinks: the string is rejected (with a TypeError), the
        // typed value still converts.
        let deny =
            v8::Local::<v8::Function>::try_from(eval(scope, "(action => action !== 'sink')"))
                .unwrap();
        assert!(install_trusted_types_policy(scope, deny));
        {
            v8::tc_scope!(let scope, scope);
            assert!(
                trusted_type_string(scope, document, text, "TrustedScript", "eval", "").is_none()
            );
            let error = scope
                .exception()
                .map(|error| error.to_rust_string_lossy(scope));
            assert!(
                error
                    .as_deref()
                    .is_some_and(|error| error.starts_with("TypeError")),
                "{error:?}"
            );
        }
        let converted =
            trusted_type_string(scope, document, typed, "TrustedScript", "eval", "").unwrap();
        assert_eq!(converted.to_rust_string_lossy(scope), "a + 1;");
    });
}

/// The census counts listeners subscribed to an AbortSignal: adding one with a signal adds
/// a subscription (a plain listener does not), aborting the signal removes it.
#[test]
fn native_listener_census() {
    with_page("<!doctype html><p id=p></p>", |scope| {
        let before = native_listener_subscription_count_for_testing();
        eval(
            scope,
            "globalThis.p = document.getElementById('p'); p.addEventListener('click', () => {})",
        );
        assert_eq!(native_listener_subscription_count_for_testing(), before);
        eval(scope, "globalThis.controller = new AbortController(); p.addEventListener('click', () => {}, { signal: controller.signal })");
        assert_eq!(native_listener_subscription_count_for_testing(), before + 1);
        eval(scope, "controller.abort()");
        assert_eq!(native_listener_subscription_count_for_testing(), before);
    });
}

/// V8's interceptor inline-cache hazard: a load site that read an absent property through
/// the missing-member interceptor still sees the property once it becomes a real own one.
#[test]
fn absent_member_reads_see_later_own_properties() {
    with_page("<!doctype html><p id=p></p>", |scope| {
        assert_eq!(
            json(
                scope,
                "(() => { const p = document.getElementById('p'); const read = n => n.__reactProps$x; const before = []; for (let i = 0; i < 2000; i++) before.push(read(p)); p.__reactProps$x = 7; return [before.every(v => v === undefined), read(p), '__reactProps$x' in document.body]; })()"
            ),
            "[true,7,false]"
        );
    });
}

/// The host's form entry-list transaction: entries read the form's controls; while a
/// FormData-constructor transaction holds the form, no further entry list is built (the
/// constructing-entry-list flag) until the host finishes it. Finishing anything but a form
/// changes nothing. Without a host FormData runtime there is no formdata dispatcher.
#[test]
fn form_entry_list_transactions_and_formdata_dispatch() {
    with_page(
        "<!doctype html><form id=f><input name=a value=1><button name=b value=2>s</button></form><div id=d></div>",
        |scope| {
            let form = object(scope, "globalThis.f = document.getElementById('f')");
            let none: v8::Local<v8::Value> = v8::undefined(scope).into();
            let rows = |scope: &mut v8::PinScope, value: Option<v8::Local<v8::Array>>| {
                value.map(|array| {
                    let global = scope.get_current_context().global(scope);
                    let key = v8::String::new(scope, "rows").unwrap();
                    global.set(scope, key.into(), array.into());
                    json(scope, "rows")
                })
            };
            let entries = r#"[["a","1"]]"#.to_string();
            let read = form_submission_entries(scope, form, none);
            assert_eq!(rows(scope, read), Some(entries.clone()));
            let submitter = eval(scope, "f.querySelector('button')");
            let read = form_submission_entries(scope, form, submitter);
            assert_eq!(rows(scope, read), Some(r#"[["a","1"],["b","2"]]"#.to_string()));
            let held = begin_form_data_constructor_entries(scope, form.into(), none);
            assert_eq!(rows(scope, held), Some(entries.clone()));
            assert!(begin_form_data_constructor_entries(scope, form.into(), none).is_none());
            assert!(form_submission_entries(scope, form, none).is_none());
            // Finishing a non-form, a non-node or a non-object leaves the transaction held.
            let div = eval(scope, "document.getElementById('d')");
            finish_form_data_constructor(scope, div);
            let plain = eval(scope, "({})");
            finish_form_data_constructor(scope, plain);
            let five = v8::Integer::new(scope, 5).into();
            finish_form_data_constructor(scope, five);
            assert!(form_submission_entries(scope, form, none).is_none());
            finish_form_data_constructor(scope, form.into());
            let read = begin_form_data_constructor_entries(scope, form.into(), none);
            assert_eq!(rows(scope, read), Some(entries));
            finish_form_data_constructor(scope, form.into());
            eval(scope, "globalThis.seen = []; f.addEventListener('formdata', () => seen.push(1))");
            let data = eval(scope, "({})");
            assert!(!dispatch_formdata_event(scope, form, data));
            assert_eq!(json(scope, "seen"), "[]");
        },
    );
}

/// Both host form callbacks need a native document: without one they do not register.
#[test]
fn form_host_callbacks_without_a_document() {
    crate::browser_env::tests::init_v8();
    reset();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    let code = v8::String::new(scope, "(() => true)").unwrap();
    let value = v8::Script::compile(scope, code, None)
        .unwrap()
        .run(scope)
        .unwrap();
    let callback = v8::Local::<v8::Function>::try_from(value).unwrap();
    assert!(!install_form_submission_handler(scope, callback));
    assert!(!install_form_value_reader(scope, callback));
    install_with_html(scope, "<!doctype html><form></form>");
    assert!(install_form_submission_handler(scope, callback));
    assert!(install_form_value_reader(scope, callback));
    reset();
}

/// Light-document membership and composed containment answer only for nodes: shadow-tree
/// nodes are composed descendants of their host but not light document nodes, detached
/// nodes are neither, and nodes of different realms never contain each other.
#[test]
fn light_membership_and_composed_containment() {
    with_page("<!doctype html><div id=host></div>", |scope| {
        let host = object(scope, "globalThis.host = document.getElementById('host')");
        let inner = object(
            scope,
            "const root = host.attachShadow({ mode: 'open' }); root.innerHTML = '<b>i</b>'; root.firstChild",
        );
        let detached = object(scope, "document.createElement('i')");
        let body = object(scope, "document.body");
        let plain = object(scope, "({})");
        assert!(is_light_document_node(scope, host));
        assert!(!is_light_document_node(scope, inner));
        assert!(!is_light_document_node(scope, detached));
        assert!(!is_light_document_node(scope, plain));
        assert!(composed_contains_node(scope, body, inner));
        assert!(composed_contains_node(scope, host, host));
        assert!(!composed_contains_node(scope, inner, host));
        assert!(!composed_contains_node(scope, body, detached));
        assert!(!composed_contains_node(scope, plain, host));
        assert!(!composed_contains_node(scope, body, plain));
        let child = v8::Context::new(scope, Default::default());
        let foreign = {
            let scope = &mut v8::ContextScope::new(scope, child);
            install_independent_html(scope, "<p>child</p>");
            let code = v8::String::new(scope, "document.body").unwrap();
            let value = v8::Script::compile(scope, code, None)
                .unwrap()
                .run(scope)
                .unwrap();
            v8::Global::new(scope, v8::Local::<v8::Object>::try_from(value).unwrap())
        };
        let foreign = v8::Local::new(scope, &foreign);
        assert!(!composed_contains_node(scope, body, foreign));
        assert!(!composed_contains_node(scope, foreign, body));
    });
}

/// A host submission handler that throws while a click submits a form: the error is
/// reported to the window (as an uncaught exception would be) and the click completes.
#[test]
fn throwing_submission_handler_is_reported() {
    with_page(
        "<!doctype html><form id=f action=/go><button id=b>go</button></form>",
        |scope| {
            let handler = v8::Local::<v8::Function>::try_from(eval(
                scope,
                "(operation => { if (operation === 'submit') throw new RangeError('host handler'); return true; })",
            ))
            .unwrap();
            assert!(install_form_submission_handler(scope, handler));
            assert_eq!(
                json(
                    scope,
                    "(() => { const seen = []; window.onerror = (message, source, line, column, error) => { seen.push(error && error.message); return true; }; document.getElementById('f').addEventListener('submit', () => seen.push('submit')); document.getElementById('b').click(); window.onerror = null; return seen; })()"
                ),
                r#"["submit","host handler"]"#
            );
        },
    );
}
