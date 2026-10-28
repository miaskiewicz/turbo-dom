//! Host-side and contract edges of the binding (crate-owned): the embedder APIs given the
//! wrong kind of object, the bounded XPath snapshot's documented budgets (each a
//! NotSupportedError naming the exceeded budget), Trusted Types enforcement at the
//! HTMLScriptElement text sinks, and the interceptor frame-read source.
use crate::browser_env::*;

fn with_page(html: &str, body: impl FnOnce(&mut v8::PinScope)) {
    crate::browser_env::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, html);
    v8::tc_scope!(let scope, scope);
    body(scope);
    reset();
}

fn eval<'s>(scope: &mut v8::PinScope<'s, '_>, source: &str) -> v8::Local<'s, v8::Value> {
    let code = v8::String::new(scope, source).unwrap();
    v8::Script::compile(scope, code, None)
        .and_then(|script| script.run(scope))
        .expect("page script runs")
}

fn json(scope: &mut v8::PinScope, source: &str) -> String {
    let value = eval(scope, &format!("JSON.stringify({source})"));
    value.to_rust_string_lossy(scope)
}

fn object<'s>(scope: &mut v8::PinScope<'s, '_>, source: &str) -> v8::Local<'s, v8::Object> {
    v8::Local::<v8::Object>::try_from(eval(scope, source)).unwrap()
}

/// `set_document_resource_active` accepts only a Document node.
#[test]
fn resource_activity_is_set_on_documents_only() {
    with_page("<p id=p>x</p>", |scope| {
        let plain = object(scope, "({})");
        let element = object(scope, "document.getElementById('p')");
        let document = object(scope, "document");
        assert!(!set_document_resource_active(scope, plain, false));
        assert!(!set_document_resource_active(scope, element, false));
        assert!(set_document_resource_active(scope, document, false));
        assert!(set_document_resource_active(scope, document, true));
    });
}

/// An unknown host pointer command is a no-op returning null; `process` with nothing
/// pending reports no capture change.
#[test]
fn host_pointer_ignores_unknown_commands() {
    with_page("<p id=p>x</p>", |scope| {
        let document = object(scope, "document");
        assert!(host_pointer(scope, document, "bogus").is_null());
        let processed = host_pointer(scope, document, "process");
        let processed = v8::Local::<v8::Array>::try_from(processed).unwrap();
        assert_eq!(processed.length(), 2);
        for index in 0..2 {
            assert!(processed.get_index(scope, index).unwrap().is_null());
        }
    });
}

/// Trusted user input aimed at an object of no installed document reaches nothing.
#[test]
fn editing_user_input_needs_a_document_target() {
    with_page("<div contenteditable id=e>x</div>", |scope| {
        let plain = object(scope, "({})");
        assert!(!editing_user_input(scope, plain, "insertText", Some("y")));
        assert_eq!(
            json(scope, "document.getElementById('e').textContent"),
            r#""x""#
        );
    });
}

/// Native storage collection rejects (before collecting anything) a terminating isolate,
/// more documents than `max_documents`, and more pending job pins or live wrappers than
/// `max_edges`; within budget it reports each document.
#[test]
fn native_storage_collection_enforces_its_budgets() {
    use turbo_dom_parser::rtdom::tree::{CollectionBudget, CollectionError};
    let budget = |max_edges| CollectionBudget {
        max_nodes: 1 << 20,
        max_edges,
        max_payload_bytes: 1 << 24,
    };
    with_page("<p id=a>a</p><p id=b>b</p>", |scope| {
        let wide = budget(1 << 20);
        assert_eq!(
            collect_native_storage(scope, wide, 1)
                .unwrap()
                .documents
                .len(),
            1
        );
        // The current document alone exceeds a zero-document budget.
        assert_eq!(
            collect_native_storage(scope, wide, 0).unwrap_err(),
            CollectionError::BudgetExceeded
        );
        // Two live wrappers exceed a one-edge budget.
        let a = object(scope, "document.getElementById('a')");
        let b = object(scope, "document.getElementById('b')");
        assert_eq!(
            collect_native_storage(scope, budget(1), 1).unwrap_err(),
            CollectionError::BudgetExceeded
        );
        // Two pending job pins exceed it before any wrapper is counted.
        let pins = [
            NativeNodePin::capture(scope, a),
            NativeNodePin::capture(scope, b),
        ];
        assert!(pins.iter().all(Option::is_some));
        assert_eq!(
            collect_native_storage(scope, budget(1), 1).unwrap_err(),
            CollectionError::BudgetExceeded
        );
        assert!(collect_native_storage(scope, wide, 1).is_ok());
        // A second, independent document: more stored documents than the budget admits.
        let context = v8::Context::new(scope, Default::default());
        {
            let scope = &mut v8::ContextScope::new(scope, context);
            install_independent_html(scope, "<i>x</i>");
        }
        assert_eq!(
            collect_native_storage(scope, wide, 2)
                .unwrap()
                .documents
                .len(),
            2
        );
        assert_eq!(
            collect_native_storage(scope, wide, 0).unwrap_err(),
            CollectionError::BudgetExceeded
        );
        // A pending termination stops the collection before it starts.
        let code = v8::String::new(scope, "0").unwrap();
        let script = v8::Script::compile(scope, code, None).unwrap();
        scope.terminate_execution();
        assert!(script.run(scope).is_none());
        assert_eq!(
            collect_native_storage(scope, wide, 2).unwrap_err(),
            CollectionError::BudgetExceeded
        );
        scope.cancel_terminate_execution();
        assert!(collect_native_storage(scope, wide, 2).is_ok());
        drop(pins);
    });
}

/// The XPath snapshot's budgets: shadow-root / fragment contexts, context depth, tree
/// size and depth, and text / attribute bytes each raise their own NotSupportedError.
#[test]
fn xpath_snapshot_budgets_raise_not_supported_errors() {
    with_page("<!doctype html><body>", |scope| {
        eval(
            scope,
            "globalThis.fail = (context, expression = 'count(.)') => { try { document.evaluate(expression, context, null, 1, null); return 'ok'; } catch (error) { return error.name + ': ' + error.message; } }",
        );
        assert_eq!(
            json(
                scope,
                "(() => { const host = document.body.appendChild(document.createElement('div')); const root = host.attachShadow({mode: 'open'}); root.innerHTML = '<b>x</b>'; const fragment = document.createDocumentFragment(); fragment.append(document.createElement('i')); return [fail(root.firstChild), fail(fragment.firstChild), fail(host)]; })()"
            ),
            r#"["NotSupportedError: XPath fragment/shadow-root contexts are unsupported","NotSupportedError: XPath fragment/shadow-root contexts are unsupported","ok"]"#
        );
        // 128 ancestors above the context are within budget, 129 are not.
        assert_eq!(
            json(
                scope,
                "(() => { const chain = depth => { const top = document.createElement('div'); let node = top; for (let i = 1; i < depth; i++) node = node.appendChild(document.createElement('div')); return [top, node]; }; const [a, deepA] = chain(126); document.body.append(a); const okDepth = fail(deepA); a.remove(); const [b, deepB] = chain(127); document.body.append(b); const tooDeep = fail(deepB); b.remove(); return [okDepth, tooDeep]; })()"
            ),
            r#"["ok","NotSupportedError: XPath context depth exceeds its budget"]"#
        );
        // A detached subtree deeper than the walk budget, and one wider than the node budget.
        assert_eq!(
            json(
                scope,
                "(() => { const top = document.createElement('div'); let node = top; for (let i = 0; i < 129; i++) node = node.appendChild(document.createElement('div')); const deep = fail(top); const wide = document.createElement('div'); for (let i = 0; i < 4096; i++) wide.append(document.createElement('i')); return [deep, fail(wide)]; })()"
            ),
            r#"["NotSupportedError: XPath tree exceeds its node/depth budget","NotSupportedError: XPath tree exceeds its node budget"]"#
        );
        // A detached root with 4085 children (plus the document's own nodes) leaves room for one
        // attribute row, not eleven.
        assert_eq!(
            json(
                scope,
                "(() => { const make = attributes => { const wide = document.createElement('div'); for (let i = 0; i < 4085; i++) wide.append(document.createElement('i')); for (let i = 0; i < attributes; i++) wide.lastChild.setAttribute('a' + i, ''); return fail(wide); }; return [make(1), make(11)]; })()"
            ),
            r#"["ok","NotSupportedError: XPath attributes exceed the node budget"]"#
        );
        assert_eq!(
            json(
                scope,
                "(() => { const big = 'x'.repeat(1024 * 1024); const text = document.createElement('p'); text.append(big); const attribute = document.createElement('p'); attribute.setAttribute('v', big); return [fail(text), fail(attribute), fail(document.createElement('p').appendChild(document.createTextNode(big.slice(256))))]; })()"
            ),
            r#"["NotSupportedError: XPath text exceeds its byte budget","NotSupportedError: XPath attributes exceed the byte budget","ok"]"#
        );
    });
}

/// Under `require-trusted-types-for 'script'` (a policy that blocks every string sink),
/// the HTMLScriptElement innerText / text setters throw Chrome's TypeError and leave the
/// script unchanged.
#[test]
fn script_text_sinks_require_trusted_scripts() {
    with_page("<!doctype html><body>", |scope| {
        let callback =
            v8::Local::<v8::Function>::try_from(eval(scope, "(action) => action !== 'sink'"))
                .unwrap();
        assert!(install_trusted_types_policy(scope, callback));
        assert_eq!(
            json(
                scope,
                "(() => { const script = document.createElement('script'); script.append('void 0'); const run = assign => { try { assign(); return 'ok'; } catch (error) { return error.name + ': ' + error.message; } }; return [run(() => { script.innerText = 'a'; }), run(() => { script.text = 'b'; }), script.text, run(() => { document.createElement('div').innerText = 'c'; })]; })()"
            ),
            r#"["TypeError: Failed to set the 'innerText' property on 'HTMLScriptElement': This document requires 'TrustedScript' assignment.","TypeError: Failed to set the 'text' property on 'HTMLScriptElement': This document requires 'TrustedScript' assignment.","void 0","ok"]"#
        );
        // An SVG <script>'s href (plain or xlink) is a TrustedScriptURL sink; other SVG
        // attributes are not.
        assert_eq!(
            json(
                scope,
                "(() => { const script = document.createElementNS('http://www.w3.org/2000/svg', 'script'); const run = assign => { try { assign(); return 'ok'; } catch (error) { return error.name + ': ' + error.message; } }; return [run(() => script.setAttribute('href', 'a.js')), run(() => script.setAttributeNS('http://www.w3.org/1999/xlink', 'xlink:href', 'b.js')), run(() => script.setAttribute('type', 'text/x')), script.getAttributeNames()]; })()"
            ),
            r#"["TypeError: Failed to execute 'setAttribute' on 'Element': This document requires 'TrustedScriptURL' assignment.","TypeError: Failed to execute 'setAttributeNS' on 'Element': This document requires 'TrustedScriptURL' assignment.","ok",["type"]]"#
        );
    });
}

#[cfg(feature = "test-support")]
mod frame_read {
    use super::*;
    use std::cell::RefCell;

    thread_local! {
        /// (frame read, V8 accessor) per intercepted named set.
        static SEEN: RefCell<Vec<(bool, bool)>> = const { RefCell::new(Vec::new()) };
    }

    fn setter<'s>(
        scope: &mut v8::PinScope<'s, '_>,
        _: v8::Local<'s, v8::Name>,
        _: v8::Local<'s, v8::Value>,
        args: v8::PropertyCallbackArguments<'s>,
        _: v8::ReturnValue<()>,
    ) -> v8::Intercepted {
        let frame = crate::test_support::frame_read_should_throw(scope, &args);
        SEEN.with(|seen| {
            seen.borrow_mut()
                .push((frame, args.should_throw_on_error()))
        });
        v8::Intercepted::kNo
    }

    /// The exported frame-read source agrees with V8's accessor in sloppy and strict code.
    #[test]
    fn exported_frame_read_agrees_with_the_accessor() {
        crate::browser_env::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        let template = v8::ObjectTemplate::new(scope);
        template.set_named_property_handler(
            v8::NamedPropertyHandlerConfiguration::new().setter(setter),
        );
        let target = template.new_instance(scope).unwrap();
        let global = context.global(scope);
        let key = v8::String::new(scope, "target").unwrap();
        global.set(scope, key.into(), target.into());
        SEEN.with(|seen| seen.borrow_mut().clear());
        eval(
            scope,
            "target.a = 1; (() => { 'use strict'; target.b = 2; })()",
        );
        assert_eq!(
            SEEN.with(|seen| seen.borrow().clone()),
            [(false, false), (true, true)]
        );
    }
}
