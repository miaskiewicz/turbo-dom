#![cfg(feature = "host-internals")]

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Once;

use sha2::{Digest, Sha256};

use turbo_dom_browser_env::host_internals::*;

#[derive(Default)]
struct Host {
    contexts: RefCell<Vec<v8::Global<v8::Context>>>,
    opens: RefCell<Vec<OpenRequest>>,
    messages: RefCell<Vec<(AuxiliaryContextId, AuxiliaryContextId, String)>>,
    children: RefCell<Vec<v8::Global<v8::Context>>>,
    navigations: RefCell<Vec<(AuxiliaryContextId, AuxiliaryContextId, String)>>,
    actions: RefCell<Vec<(AuxiliaryContextId, AuxiliaryContextId, bool)>>,
}

fn init() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        v8::V8::initialize_platform(v8::new_default_platform(0, false).make_shared());
        v8::V8::initialize();
    });
}

fn eval<'s>(scope: &mut v8::PinScope<'s, '_>, code: &str) -> v8::Local<'s, v8::Value> {
    let source = v8::String::new(scope, code).unwrap();
    v8::Script::compile(scope, source, None)
        .unwrap_or_else(|| panic!("compile: {code}"))
        .run(scope)
        .unwrap_or_else(|| panic!("execute: {code}"))
}

fn assert_js(scope: &mut v8::PinScope, code: &str) {
    assert!(eval(scope, code).is_true(), "{code}");
}

fn set<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    context: v8::Local<'s, v8::Context>,
    name: &str,
    value: v8::Local<'s, v8::Value>,
) {
    let scope = &mut v8::ContextScope::new(scope, context);
    let key = v8::String::new(scope, name).unwrap();
    assert_eq!(
        context
            .global(scope)
            .create_data_property(scope, key.into(), value),
        Some(true)
    );
}

fn close(_: &mut v8::PinScope, _: v8::Local<v8::Context>) -> bool {
    true
}

fn admit<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    source: v8::Local<'s, v8::Context>,
    selected: Option<v8::Local<'s, v8::Context>>,
    request: &OpenRequest,
) -> Option<v8::Local<'s, v8::Context>> {
    let host = scope.get_slot::<Rc<Host>>().cloned().unwrap();
    host.opens.borrow_mut().push(request.clone());
    if let Some(context) = selected {
        return Some(context);
    }
    let context = create_auxiliary_window_context(scope, None);
    context.set_security_token(source.get_security_token(scope));
    {
        let scope = &mut v8::ContextScope::new(scope, context);
        initialize_auxiliary_document(scope, "<!doctype html><body>initial child</body>");
    }
    host.contexts
        .borrow_mut()
        .push(v8::Global::new(scope, context));
    Some(context)
}

fn root<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    origin: &str,
    primary: bool,
) -> v8::Local<'s, v8::Context> {
    let context = create_auxiliary_window_context(scope, None);
    let token = v8::String::new(scope, origin).unwrap();
    context.set_security_token(token.into());
    {
        let scope = &mut v8::ContextScope::new(scope, context);
        if primary {
            initialize_root_auxiliary_document(scope, "<!doctype html><body>root</body>");
        } else {
            initialize_auxiliary_document(scope, "<!doctype html><body>peer</body>");
        }
        assert!(install_auxiliary_context(
            scope,
            context,
            &format!("{origin}/"),
            Some(AuxiliaryHost { open: admit, close })
        ));
    }
    context
}

fn message<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    source: v8::Local<'s, v8::Context>,
    target: v8::Local<'s, v8::Context>,
    value: v8::Local<'s, v8::Value>,
    origin: v8::Local<'s, v8::Value>,
    _transfer: v8::Local<'s, v8::Value>,
) -> bool {
    // This deterministic host records admission only. Delivery/serialization is
    // the consumer's responsibility; this test pins the native source identity.
    assert_eq!(origin.to_rust_string_lossy(scope), "*");
    let value = value.to_rust_string_lossy(scope);
    scope
        .get_slot::<Rc<Host>>()
        .unwrap()
        .messages
        .borrow_mut()
        .push((
            auxiliary_context_identity(source).unwrap().id,
            auxiliary_context_identity(target).unwrap().id,
            value,
        ));
    true
}

#[test]
fn cross_origin_descriptors_keep_the_accessing_realm_and_incumbent_message_source() {
    init();
    let mut isolate = v8::Isolate::new(Default::default());
    isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
    isolate.set_failed_access_check_callback(auxiliary_window_failed_access);
    let host = Rc::new(Host::default());
    isolate.set_slot(Rc::clone(&host));
    {
        v8::scope!(let scope, &mut isolate);
        let template = auxiliary_window_template(scope);
        let a = v8::Context::new(
            scope,
            v8::ContextOptions {
                global_template: Some(template),
                ..Default::default()
            },
        );
        let scope = &mut v8::ContextScope::new(scope, a);
        let token = v8::String::new(scope, "https://a.example").unwrap();
        a.set_security_token(token.into());
        initialize_root_auxiliary_document(scope, "<!doctype html><body>root</body>");
        assert!(install_auxiliary_context(
            scope,
            a,
            "https://a.example/",
            Some(AuxiliaryHost { open: admit, close })
        ));
        let b = root(scope, "https://b.example", false);
        let c = root(scope, "https://c.example", false);
        let window_c = c.global(scope);
        set(scope, a, "target", window_c.into());
        set(scope, b, "target", window_c.into());
        install_auxiliary_window_hooks(
            c,
            AuxiliaryWindowHooks {
                post_message: Some(message),
                ..Default::default()
            },
        );
        let from_a = {
            let scope = &mut v8::ContextScope::new(scope, a);
            assert_js(scope, "target.window===target&&target.self===target&&target.parent===target&&target.top===target&&target.opener===null&&target.closed===false&&target.length===0");
            assert_js(scope, "(()=>{try{void target.document}catch(e){return e.name==='SecurityError'&&e instanceof DOMException&&e.constructor.constructor('return globalThis')()===globalThis}return false})()");
            assert_js(scope, "(()=>{try{void target.location}catch(e){return e.name==='NotSupportedError'}return false})()");
            assert_js(scope, "target.close===target.close&&target.close.constructor===Function&&Object.getOwnPropertyDescriptor(target,'closed').get.constructor===Function");
            let value = eval(scope, "target.close");
            v8::Global::new(scope, value)
        };
        let (from_b, send_b) = {
            let scope = &mut v8::ContextScope::new(scope, b);
            assert_js(scope, "(()=>{try{void target.document}catch(e){return e.name==='SecurityError'&&e.constructor.constructor('return globalThis')()===globalThis}return false})()");
            assert_js(scope, "target.close===target.close&&target.close.constructor===Function&&Object.getOwnPropertyDescriptor(target,'closed').get.constructor===Function");
            let close = eval(scope, "target.close");
            let close = v8::Global::new(scope, close);
            let send = eval(
                scope,
                "(function sendFromB(){target.postMessage('from-b','*')})",
            );
            (close, v8::Global::new(scope, send))
        };
        assert!(!v8::Local::new(scope, &from_a).strict_equals(v8::Local::new(scope, &from_b)));
        let send_b = v8::Local::new(scope, &send_b);
        set(scope, a, "sendFromB", send_b);
        {
            let scope = &mut v8::ContextScope::new(scope, a);
            eval(scope, "sendFromB()");
        }
        let messages = host.messages.borrow();
        assert_eq!(messages.len(), 1);
        assert_eq!(
            messages[0],
            (
                auxiliary_context_identity(b).unwrap().id,
                auxiliary_context_identity(c).unwrap().id,
                "from-b".to_owned()
            )
        );
        drop(messages);
        host.contexts.borrow_mut().clear();
    }
    isolate.remove_slot::<Rc<Host>>();
}

#[test]
fn retained_lexical_and_native_callbacks_cannot_access_a_cross_origin_replacement() {
    init();
    let mut isolate = v8::Isolate::new(Default::default());
    isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
    isolate.set_failed_access_check_callback(auxiliary_window_failed_access);
    let host = Rc::new(Host::default());
    isolate.set_slot(Rc::clone(&host));
    {
        v8::scope!(let scope, &mut isolate);
        let template = auxiliary_window_template(scope);
        let a = v8::Context::new(
            scope,
            v8::ContextOptions {
                global_template: Some(template),
                ..Default::default()
            },
        );
        let scope = &mut v8::ContextScope::new(scope, a);
        let token = v8::String::new(scope, "https://a.example").unwrap();
        a.set_security_token(token.into());
        initialize_root_auxiliary_document(scope, "<!doctype html><body>root</body>");
        assert!(install_auxiliary_context(
            scope,
            a,
            "https://a.example/",
            Some(AuxiliaryHost { open: admit, close })
        ));
        {
            let scope = &mut v8::ContextScope::new(scope, a);
            eval(scope, "globalThis.popup=open('', 'child')");
        }
        let old = v8::Local::new(scope, &host.contexts.borrow()[0]);
        let initial_id = auxiliary_context_identity(old).unwrap();
        let old_callbacks = {
            let scope = &mut v8::ContextScope::new(scope, old);
            let value = eval(
                scope,
                r#"(()=>{
              const win=window;
              const savedOpen=open;
              const name=Object.getOwnPropertyDescriptor(window,'name');
              const opener=Object.getOwnPropertyDescriptor(window,'opener');
              return {
                doc:Function('return document'),
                globalDoc:Function('return globalThis.document'),
                open:()=>savedOpen.call(win,'/must-not-open'),
                nameGet:()=>name.get.call(win),
                nameSet:()=>name.set.call(win,'must-not-rename'),
                openerSet:()=>opener.set.call(win,null),
              };
            })()"#,
            );
            v8::Global::new(scope, value)
        };
        let callbacks = v8::Local::new(scope, &old_callbacks);
        set(scope, a, "oldCallbacks", callbacks);
        let replacement = create_auxiliary_window_context(scope, Some(old));
        let token = v8::String::new(scope, "https://foreign.example").unwrap();
        replacement.set_security_token(token.into());
        {
            let scope = &mut v8::ContextScope::new(scope, replacement);
            initialize_auxiliary_document(scope, "<!doctype html><body>cross-origin secret</body>");
        }
        assert!(commit_auxiliary_navigation(
            scope,
            old,
            replacement,
            "https://foreign.example/"
        ));
        let after = auxiliary_context_identity(replacement).unwrap();
        assert_eq!(after.id, initial_id.id);
        assert_eq!(after.creation_source, initial_id.creation_source);
        let calls = host.opens.borrow().len();
        {
            let scope = &mut v8::ContextScope::new(scope, a);
            assert_js(scope, "Object.values(oldCallbacks).every(fn=>{try{fn();return false}catch(e){return e.name==='SecurityError'}})");
            assert_js(scope, "(()=>{try{void popup.document}catch(e){return e.name==='SecurityError'}return false})()");
            assert_js(
                scope,
                "popup.window===popup&&popup.opener===window&&!popup.closed",
            );
        }
        assert_eq!(host.opens.borrow().len(), calls);
        assert!(auxiliary_context_opener(scope, replacement)
            .unwrap()
            .strict_equals(a.global(scope).into()));
        let restored = create_auxiliary_window_context(scope, Some(replacement));
        restored.set_security_token(a.get_security_token(scope));
        {
            let scope = &mut v8::ContextScope::new(scope, restored);
            initialize_auxiliary_document(
                scope,
                "<!doctype html><body>restored same origin</body>",
            );
        }
        assert!(commit_auxiliary_navigation(
            scope,
            replacement,
            restored,
            "https://a.example/restored"
        ));
        {
            let scope = &mut v8::ContextScope::new(scope, a);
            assert_js(scope, "oldCallbacks.doc()===popup.document&&oldCallbacks.globalDoc()===popup.document&&popup.document.body.textContent==='restored same origin'");
            assert_js(
                scope,
                "oldCallbacks.nameSet();oldCallbacks.nameGet()==='must-not-rename'",
            );
        }
        assert_eq!(host.opens.borrow().len(), calls);
        host.contexts.borrow_mut().clear();
    }
    isolate.remove_slot::<Rc<Host>>();
}

fn fixture(
    test: impl for<'s, 'i> FnOnce(
        &mut v8::PinScope<'s, 'i>,
        &Rc<Host>,
        v8::Local<'s, v8::Context>,
        v8::Local<'s, v8::Context>,
    ),
) {
    init();
    let mut isolate = v8::Isolate::new(Default::default());
    isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
    isolate.set_failed_access_check_callback(auxiliary_window_failed_access);
    let host = Rc::new(Host::default());
    isolate.set_slot(Rc::clone(&host));
    {
        v8::scope!(let scope, &mut isolate);
        let bootstrap = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, bootstrap);
        let source = root(scope, "https://source.example", true);
        let target = root(scope, "https://target.example", false);
        let scope = &mut v8::ContextScope::new(scope, source);
        let window = target.global(scope);
        set(scope, source, "target", window.into());
        test(scope, &host, source, target);
        host.children.borrow_mut().clear();
        host.contexts.borrow_mut().clear();
    }
    isolate.remove_slot::<Rc<Host>>();
}

fn children<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    _: v8::Local<'s, v8::Context>,
) -> Vec<v8::Local<'s, v8::Context>> {
    let host = scope.get_slot::<Rc<Host>>().cloned().unwrap();
    let children = host.children.borrow();
    children
        .iter()
        .map(|child| v8::Local::new(scope, child))
        .collect()
}

fn location<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    _: v8::Local<'s, v8::Context>,
) -> Option<v8::Local<'s, v8::Value>> {
    Some(
        v8::String::new(scope, "host-controlled-location")
            .unwrap()
            .into(),
    )
}

fn navigate<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    source: v8::Local<'s, v8::Context>,
    target: v8::Local<'s, v8::Context>,
    value: v8::Local<'s, v8::Value>,
) -> bool {
    let value = value.to_rust_string_lossy(scope);
    scope
        .get_slot::<Rc<Host>>()
        .unwrap()
        .navigations
        .borrow_mut()
        .push((
            auxiliary_context_identity(source).unwrap().id,
            auxiliary_context_identity(target).unwrap().id,
            value,
        ));
    true
}

fn focus<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    source: v8::Local<'s, v8::Context>,
    target: v8::Local<'s, v8::Context>,
    focused: bool,
) -> bool {
    scope
        .get_slot::<Rc<Host>>()
        .unwrap()
        .actions
        .borrow_mut()
        .push((
            auxiliary_context_identity(source).unwrap().id,
            auxiliary_context_identity(target).unwrap().id,
            focused,
        ));
    true
}

#[test]
fn cross_origin_children_and_property_introspection_are_host_owned() {
    fixture(|scope, host, _, target| {
        let child = root(scope, "https://child.example", false);
        host.children
            .borrow_mut()
            .push(v8::Global::new(scope, child));
        install_auxiliary_window_hooks(
            target,
            AuxiliaryWindowHooks {
                children: Some(children),
                ..Default::default()
            },
        );
        assert_js(scope, "target.length===1&&target[0].window===target[0]&&target.frames===target&&target.then===undefined");
        assert_js(scope, "Reflect.ownKeys(target).filter(k=>typeof k==='string').join(',')==='0,window,self,location,closed,frames,length,top,opener,parent,blur,close,focus,postMessage,then'");
        assert_js(scope, "[Symbol.toStringTag,Symbol.hasInstance,Symbol.isConcatSpreadable].every(k=>target[k]===undefined&&k in target&&Object.getOwnPropertyDescriptor(target,k).value===undefined)");
        assert_js(scope, "'length' in target&&'focus' in target");
        assert_js(scope, "['then','focus','blur','postMessage'].every(k=>{const d=Object.getOwnPropertyDescriptor(target,k);return !d.enumerable&&d.configurable&&!d.writable&&d.value===target[k]})");
        assert_js(scope, "['window','self','frames','length','top','parent','opener','closed'].every(k=>{const d=Object.getOwnPropertyDescriptor(target,k);return !d.enumerable&&d.configurable&&d.get.call(target)===target[k]&&d.get.constructor===Function})");
        assert_js(
            scope,
            "(()=>{try{void target[1]}catch(e){return e.name==='SecurityError'}return false})()",
        );
        assert_js(
            scope,
            "(()=>{try{return 'document' in target}catch(e){return e.name==='SecurityError'}})()",
        );
        assert_js(scope, "(()=>{try{Object.getOwnPropertyDescriptor(target,'document')}catch(e){return e.name==='SecurityError'}return false})()");
        assert_js(scope, "(()=>{try{void target[Symbol.iterator]}catch(e){return e.name==='SecurityError'}return false})()");
        assert_js(scope, "(()=>{try{Object.getOwnPropertyDescriptor(target,'closed').get.call({})}catch(e){return e instanceof TypeError&&e.message==='Illegal invocation'}return false})()");
    });
}

#[test]
fn cross_origin_location_and_actions_delegate_exact_source_target_to_host() {
    fixture(|scope, host, source, target| {
        install_auxiliary_window_hooks(
            target,
            AuxiliaryWindowHooks {
                location: Some(location),
                navigate: Some(navigate),
                focus: Some(focus),
                ..Default::default()
            },
        );
        assert_js(scope, "target.location==='host-controlled-location'&&Object.getOwnPropertyDescriptor(target,'location').get.call(target)==='host-controlled-location'");
        eval(scope, "target.location='/first';Object.getOwnPropertyDescriptor(target,'location').set.call(target,'/second');target.focus();target.blur()");
        let source = auxiliary_context_identity(source).unwrap().id;
        let target_id = auxiliary_context_identity(target).unwrap().id;
        assert_eq!(
            *host.navigations.borrow(),
            vec![
                (source, target_id, "/first".to_owned()),
                (source, target_id, "/second".to_owned())
            ]
        );
        assert_eq!(
            *host.actions.borrow(),
            vec![(source, target_id, true), (source, target_id, false)]
        );
        assert_js(scope, "(()=>{try{Object.getOwnPropertyDescriptor(target,'location').set.call({},'/forbidden')}catch(e){return e instanceof TypeError&&e.message==='Illegal invocation'}return false})()");
        assert_eq!(host.navigations.borrow().len(), 2);
        assert_js(scope, "(()=>{try{target.closed=true}catch(e){return e.name==='SecurityError'}return false})()");
        assert_js(scope, "(()=>{try{target[Symbol.toStringTag]='forged'}catch(e){return e.name==='SecurityError'}return false})()");
        // A host root was not opened by script and must not close another actor.
        assert_js(scope, "target.close();target.close();!target.closed");
    });
}

#[test]
fn absent_host_capabilities_fail_truthfully_and_cannot_be_forged_by_guest() {
    fixture(|scope, _, _, _| {
        assert_js(scope, "['focus','blur','postMessage'].every(k=>{try{target[k]('x','*');return false}catch(e){return e.name==='NotSupportedError'&&e instanceof DOMException}})");
        assert_js(scope, "(()=>{try{target.location='/not-admitted'}catch(e){return e.name==='NotSupportedError'}return false})()");
        assert_js(scope, "(()=>{try{Object.getOwnPropertyDescriptor(target,'location').set.call(target,'/not-admitted')}catch(e){return e.name==='NotSupportedError'}return false})()");
        assert_js(scope, "(()=>{try{Object.getOwnPropertyDescriptor(target,'location').get.call(target)}catch(e){return e.name==='NotSupportedError'}return false})()");
        assert_js(scope, "(()=>{const Original=DOMException;globalThis.DOMException=function(){throw Error('guest constructor')};try{void target.document}catch(e){return e.name==='SecurityError'&&e instanceof Original}return false})()");
    });
}

#[test]
fn window_constructor_and_document_getter_reject_borrowed_receivers() {
    fixture(|scope, _, source, _| {
        assert!(install_auxiliary_window_document_guard(scope, source));
        assert!(capture_auxiliary_window_exception(scope, source));
        assert_js(scope, "(()=>{try{Window()}catch(e){return e instanceof TypeError&&e.message==='Illegal constructor'}return false})()");
        assert_js(scope, "(()=>{try{new Window()}catch(e){return e instanceof TypeError&&e.message===\"Failed to construct 'Window': Illegal constructor\"}return false})()");
        assert_js(scope, "(()=>{try{Object.getOwnPropertyDescriptor(window,'document').get.call({})}catch(e){return e instanceof TypeError&&e.message==='Illegal invocation'}return false})()");
    });
}

#[test]
fn cross_origin_denial_before_dom_install_has_safe_named_error_fallback() {
    init();
    let mut isolate = v8::Isolate::new(Default::default());
    isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
    isolate.set_failed_access_check_callback(auxiliary_window_failed_access);
    v8::scope!(let scope, &mut isolate);
    let caller = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, caller);
    let target = root(scope, "https://target.example", true);
    assert!(!capture_auxiliary_window_exception(scope, caller));
    assert!(!install_auxiliary_window_document_guard(scope, caller));
    let window = target.global(scope);
    set(scope, caller, "target", window.into());
    assert_js(scope, "(()=>{try{void target.document}catch(e){return e instanceof Error&&e.name==='SecurityError'&&e.message==='Blocked cross-origin window access'}return false})()");
}

#[test]
fn cross_origin_child_index_metadata_matches_pinned_chrome_156() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../src/auxiliary_window_indexes_chrome156.json"
    ))
    .unwrap();
    assert_eq!(oracle["browser"], "HeadlessChrome/156.0.8078.47");
    assert_eq!(oracle["errors"], serde_json::json!([]));
    for (field, bytes) in [
        (
            "casesSha256",
            include_bytes!("../src/auxiliary_window_indexes_cases.json").as_slice(),
        ),
        (
            "probeSha256",
            include_bytes!("../src/auxiliary_window_indexes_probe.js").as_slice(),
        ),
        (
            "collectorSha256",
            include_bytes!("../scripts/auxiliary-window-indexes-chrome-control.mjs").as_slice(),
        ),
    ] {
        assert_eq!(oracle[field], format!("{:x}", Sha256::digest(bytes)));
    }

    let mut actual = Vec::new();
    for count in 0..=2 {
        fixture(|scope, host, source, target| {
            for index in 0..count {
                let child = create_auxiliary_window_context(scope, None);
                child.set_security_token(target.get_security_token(scope));
                {
                    let scope = &mut v8::ContextScope::new(scope, child);
                    initialize_auxiliary_document(scope, "<!doctype html><body>child</body>");
                    assert!(install_nested_auxiliary_source(
                        scope,
                        child,
                        target,
                        "https://target.example/nested",
                        &format!("NamedChild{index}")
                            .encode_utf16()
                            .collect::<Vec<_>>(),
                        None,
                    ));
                }
                host.children
                    .borrow_mut()
                    .push(v8::Global::new(scope, child));
            }
            install_auxiliary_window_hooks(
                target,
                AuxiliaryWindowHooks {
                    children: Some(children),
                    ..Default::default()
                },
            );
            let window = target.global(scope);
            set(scope, source, "__windowIndexTarget", window.into());
            let probe = include_str!("../src/auxiliary_window_indexes_probe.js");
            let json = eval(scope, &format!("JSON.stringify({probe})"));
            let result: serde_json::Value =
                serde_json::from_str(&json.to_rust_string_lossy(scope)).unwrap();
            println!("native child-index count={count}: {result}");
            actual.push(serde_json::json!({"count": count, "result": result}));
        });
    }
    assert_eq!(serde_json::Value::Array(actual), oracle["rows"]);
}

fn nested_child<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    parent: v8::Local<'s, v8::Context>,
    name: &[u16],
) -> v8::Local<'s, v8::Context> {
    let child = create_auxiliary_window_context(scope, None);
    child.set_security_token(parent.get_security_token(scope));
    {
        let scope = &mut v8::ContextScope::new(scope, child);
        initialize_auxiliary_document(scope, "<!doctype html><body>named child</body>");
        assert!(install_nested_auxiliary_source(
            scope,
            child,
            parent,
            "https://target.example/named",
            name,
            None
        ));
    }
    child
}

#[test]
fn cross_origin_named_child_lookup_matches_pinned_chrome_without_guest_getters() {
    let oracle: serde_json::Value =
        serde_json::from_str(include_str!("../src/auxiliary_window_names_chrome156.json")).unwrap();
    assert_eq!(oracle["browser"], "HeadlessChrome/156.0.8078.47");
    assert_eq!(oracle["errors"], serde_json::json!([]));
    for (field, bytes) in [
        (
            "casesSha256",
            include_bytes!("../src/auxiliary_window_names_cases.json").as_slice(),
        ),
        (
            "probeSha256",
            include_bytes!("../src/auxiliary_window_names_probe.js").as_slice(),
        ),
        (
            "collectorSha256",
            include_bytes!("../scripts/auxiliary-window-names-chrome-control.mjs").as_slice(),
        ),
    ] {
        assert_eq!(oracle[field], format!("{:x}", Sha256::digest(bytes)));
    }
    let mut rows = Vec::new();
    for case in 0..oracle["rows"].as_array().unwrap().len() {
        fixture(|scope, host, source, target| {
            // V8 parses the JSON names so lone UTF-16 surrogates stay lossless;
            // result keys are represented as code-unit arrays in the control.
            let entry = eval(
                scope,
                &format!(
                    "globalThis.__windowNameCase=({})[{case}]",
                    include_str!("../src/auxiliary_window_names_cases.json")
                ),
            );
            let entry = v8::Local::<v8::Object>::try_from(entry).unwrap();
            let key = v8::String::new(scope, "names").unwrap();
            let value = entry.get(scope, key.into()).unwrap();
            let names = v8::Local::<v8::Array>::try_from(value).unwrap();
            let key = v8::String::new(scope, "rename").unwrap();
            let rename = entry.get(scope, key.into()).unwrap();
            let key = v8::String::new(scope, "trap").unwrap();
            let trapped = entry.get(scope, key.into()).unwrap().is_true();
            for index in 0..names.length() {
                let name = names.get_index(scope, index).unwrap();
                let name = v8::Local::<v8::String>::try_from(name).unwrap();
                let mut units = vec![0; name.length()];
                name.write_v2(scope, 0, &mut units, v8::WriteFlags::empty());
                let child = nested_child(scope, target, &units);
                {
                    let scope = &mut v8::ContextScope::new(scope, child);
                    if index == 0 && rename.is_string() {
                        set(scope, child, "__renamed", rename);
                        eval(scope, "window.name=__renamed");
                    }
                    if trapped {
                        eval(scope, "globalThis.__nameGetterReads=0;Object.defineProperty(window,'name',{get(){__nameGetterReads++;throw Error('guest name getter')}})");
                    }
                }
                host.children
                    .borrow_mut()
                    .push(v8::Global::new(scope, child));
            }
            install_auxiliary_window_hooks(
                target,
                AuxiliaryWindowHooks {
                    children: Some(children),
                    location: Some(location),
                    ..Default::default()
                },
            );
            let window = target.global(scope);
            set(scope, source, "__windowIndexTarget", window.into());
            let value = eval(
                scope,
                &format!(
                    "JSON.stringify({})",
                    include_str!("../src/auxiliary_window_names_probe.js")
                ),
            );
            let result: serde_json::Value =
                serde_json::from_str(&value.to_rust_string_lossy(scope)).unwrap();
            let key = v8::String::new(scope, "id").unwrap();
            let id = entry
                .get(scope, key.into())
                .unwrap()
                .to_rust_string_lossy(scope);
            println!("native child-name case={id}: {result}");
            rows.push(serde_json::json!({"id":id,"result":result}));
            if trapped {
                for index in 0..host.children.borrow().len() {
                    let child = v8::Local::new(scope, &host.children.borrow()[index]);
                    let scope = &mut v8::ContextScope::new(scope, child);
                    assert_js(scope, "__nameGetterReads===0");
                }
            }
        });
    }
    assert_eq!(serde_json::Value::Array(rows), oracle["rows"]);
}

#[test]
fn named_children_require_current_document_parent_and_live_native_identity() {
    fixture(|scope, host, _, target| {
        let child = nested_child(scope, target, &"Owned".encode_utf16().collect::<Vec<_>>());
        host.children
            .borrow_mut()
            .push(v8::Global::new(scope, child));
        install_auxiliary_window_hooks(
            target,
            AuxiliaryWindowHooks {
                children: Some(children),
                ..Default::default()
            },
        );
        assert_js(scope, "target.Owned===target[0]");
        let isolated = create_auxiliary_window_context(scope, None);
        isolated.set_security_token(target.get_security_token(scope));
        {
            let scope = &mut v8::ContextScope::new(scope, isolated);
            initialize_auxiliary_document(scope, "<!doctype html><body>isolated</body>");
        }
        // A nested child cannot acquire a top-level COOP/group-separation identity.
        assert!(!commit_isolated_auxiliary_navigation(
            scope,
            child,
            isolated,
            "https://isolated.example/"
        ));
        assert_js(scope, "target.Owned===target[0]");
        let replacement = create_auxiliary_window_context(scope, Some(child));
        replacement.set_security_token(target.get_security_token(scope));
        assert_js(
            scope,
            "(()=>{try{void target.Owned}catch(e){return e.name==='SecurityError'}return false})()",
        );
        {
            let scope = &mut v8::ContextScope::new(scope, replacement);
            initialize_auxiliary_document(scope, "<!doctype html><body>replacement</body>");
        }
        assert!(commit_auxiliary_navigation(
            scope,
            child,
            replacement,
            "https://target.example/replaced"
        ));
        // A stale host capture cannot authorize a name using an old lexical realm.
        assert_js(
            scope,
            "(()=>{try{void target.Owned}catch(e){return e.name==='SecurityError'}return false})()",
        );
        host.children.borrow_mut()[0] = v8::Global::new(scope, replacement);
        assert_js(scope, "target.Owned===target[0]");
        assert!(retire_auxiliary_context(scope, replacement));
        assert_js(
            scope,
            "(()=>{try{void target.Owned}catch(e){return e.name==='SecurityError'}return false})()",
        );
        // Even a genuine unrelated Window in a mistaken host list is not a child.
        let unrelated = root(scope, "https://target.example", false);
        {
            let scope = &mut v8::ContextScope::new(scope, unrelated);
            eval(scope, "name='Owned'");
        }
        host.children.borrow_mut()[0] = v8::Global::new(scope, unrelated);
        assert_js(scope, "(()=>{try{Object.getOwnPropertyDescriptor(target,'Owned')}catch(e){return e.name==='SecurityError'}return false})()");
        let unregistered = create_auxiliary_window_context(scope, None);
        {
            let scope = &mut v8::ContextScope::new(scope, unregistered);
            initialize_auxiliary_document(scope, "<!doctype html><body>unregistered</body>");
            eval(scope, "name='Owned'");
        }
        host.children.borrow_mut()[0] = v8::Global::new(scope, unregistered);
        assert_js(
            scope,
            "(()=>{try{void target.Owned}catch(e){return e.name==='SecurityError'}return false})()",
        );
    });
}

fn navigated_document<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    old: v8::Local<'s, v8::Context>,
    token: v8::Local<'s, v8::Value>,
    url: &str,
    body: &str,
) -> v8::Local<'s, v8::Context> {
    let current = create_auxiliary_window_context(scope, Some(old));
    current.set_security_token(token);
    {
        let scope = &mut v8::ContextScope::new(scope, current);
        initialize_auxiliary_document(scope, body);
    }
    assert!(commit_auxiliary_navigation(scope, old, current, url));
    current
}

#[test]
fn interleaved_popup_navigations_restore_only_the_matching_proxy_origin() {
    fixture(|scope, host, source, _| {
        eval(
            scope,
            "globalThis.first=open('', 'First');globalThis.second=open('', 'Second')",
        );
        let first = v8::Local::new(scope, &host.contexts.borrow()[0]);
        let second = v8::Local::new(scope, &host.contexts.borrow()[1]);
        let saved_first = {
            let scope = &mut v8::ContextScope::new(scope, first);
            let callback = eval(scope, "Function('return document.body.textContent')");
            v8::Global::new(scope, callback)
        };
        let saved_second = {
            let scope = &mut v8::ContextScope::new(scope, second);
            let callback = eval(scope, "Function('return document.body.textContent')");
            v8::Global::new(scope, callback)
        };
        let saved_first = v8::Local::new(scope, &saved_first);
        let saved_second = v8::Local::new(scope, &saved_second);
        set(scope, source, "savedFirst", saved_first);
        set(scope, source, "savedSecond", saved_second);
        let foreign = v8::String::new(scope, "https://other.example").unwrap();
        let first_foreign = navigated_document(
            scope,
            first,
            foreign.into(),
            "https://other.example/",
            "<!doctype html><body>foreign first</body>",
        );
        let token = source.get_security_token(scope);
        let second_current = navigated_document(
            scope,
            second,
            token,
            "https://source.example/second",
            "<!doctype html><body>current second</body>",
        );
        assert_js(scope, "(()=>{try{savedFirst();return false}catch(e){return e.name==='SecurityError'}})()&&savedSecond()==='current second'");
        let first_restored = navigated_document(
            scope,
            first_foreign,
            token,
            "https://source.example/first",
            "<!doctype html><body>restored first</body>",
        );
        assert_js(
            scope,
            "savedFirst()==='restored first'&&savedSecond()==='current second'&&first!==second",
        );
        assert_ne!(
            auxiliary_context_identity(first_restored).unwrap().id,
            auxiliary_context_identity(second_current).unwrap().id
        );
    });
}

#[test]
fn retired_parent_cannot_republish_a_live_child_name_from_a_stale_host_capture() {
    fixture(|scope, host, _, target| {
        let child = nested_child(scope, target, &"Owned".encode_utf16().collect::<Vec<_>>());
        host.children
            .borrow_mut()
            .push(v8::Global::new(scope, child));
        install_auxiliary_window_hooks(
            target,
            AuxiliaryWindowHooks {
                children: Some(children),
                ..Default::default()
            },
        );
        assert_js(scope, "target.Owned===target[0]");
        assert!(retire_auxiliary_context(scope, target));
        assert_js(scope, "target.closed&&(()=>{try{void target.Owned}catch(e){return e.name==='SecurityError'}return false})()");
        assert!(!auxiliary_context_identity(child).unwrap().closed);
    });
}
