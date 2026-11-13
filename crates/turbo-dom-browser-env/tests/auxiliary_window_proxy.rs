#![cfg(feature = "host-internals")]

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Once;

use turbo_dom_browser_env::host_internals::*;

#[derive(Default)]
struct Host {
    contexts: RefCell<Vec<v8::Global<v8::Context>>>,
    opens: RefCell<Vec<OpenRequest>>,
    messages: RefCell<Vec<(AuxiliaryContextId, AuxiliaryContextId, String)>>,
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
