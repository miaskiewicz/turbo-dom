#![cfg(feature = "host-internals")]

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Once;
use turbo_dom_browser_env::{browser_env, host_internals::*};

#[derive(Default)]
struct Host {
    calls: Cell<usize>,
    consumed: Cell<bool>,
    deny: Cell<bool>,
    requests: RefCell<Vec<OpenRequest>>,
    windows: RefCell<Vec<v8::Global<v8::Context>>>,
}

fn open<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    source: v8::Local<'s, v8::Context>,
    _: Option<v8::Local<'s, v8::Context>>,
    request: &OpenRequest,
) -> Option<v8::Local<'s, v8::Context>> {
    let host = scope.get_slot::<Rc<Host>>().cloned().unwrap();
    host.requests.borrow_mut().push(request.clone());
    if host.deny.get() {
        return None;
    }
    let context = create_auxiliary_window_context(scope, None);
    let token = source.get_security_token(scope);
    context.set_security_token(token);
    {
        let scope = &mut v8::ContextScope::new(scope, context);
        initialize_auxiliary_document(scope, "<!doctype html><body>child</body>");
    }
    host.windows
        .borrow_mut()
        .push(v8::Global::new(scope, context));
    Some(context)
}

fn close(_: &mut v8::PinScope, _: v8::Local<v8::Context>) -> bool {
    true
}

fn activate<'s>(scope: &mut v8::PinScope<'s, '_>, anchor: v8::Local<'s, v8::Object>) -> bool {
    let host = scope.get_slot::<Rc<Host>>().cloned().unwrap();
    host.calls.set(host.calls.get() + 1);
    if !host.consumed.get() {
        return false;
    }
    open_auxiliary_hyperlink(scope, anchor);
    true
}

fn handled<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    let event = v8::Local::<v8::Object>::try_from(args.get(0)).unwrap();
    let value = hyperlink_activation_handled(scope, event);
    rv.set_bool(value);
}

fn eval<'s>(scope: &mut v8::PinScope<'s, '_>, code: &str) -> v8::Local<'s, v8::Value> {
    let code = v8::String::new(scope, code).unwrap();
    v8::Script::compile(scope, code, None)
        .unwrap()
        .run(scope)
        .unwrap()
}

fn assert_js(scope: &mut v8::PinScope, code: &str) {
    assert!(eval(scope, code).is_true(), "{code}");
}

fn fixture(test: impl FnOnce(&mut v8::PinScope, &Rc<Host>)) {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        v8::V8::initialize_platform(v8::new_default_platform(0, false).make_shared());
        v8::V8::initialize();
    });
    let mut isolate = v8::Isolate::new(Default::default());
    isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
    let host = Rc::new(Host::default());
    host.consumed.set(true);
    isolate.set_slot(Rc::clone(&host));
    v8::scope!(let scope, &mut isolate);
    let template = auxiliary_window_template(scope);
    let context = v8::Context::new(
        scope,
        v8::ContextOptions {
            global_template: Some(template),
            ..Default::default()
        },
    );
    let scope = &mut v8::ContextScope::new(scope, context);
    browser_env::install_independent_html(scope, "<!doctype html><body><a id=a href=child target=_blank><span id=s>child</span></a><area id=area href=area target=_blank><button id=b>button</button>");
    assert!(install_auxiliary_context(
        scope,
        context,
        "https://example.test/base/",
        Some(AuxiliaryHost { open, close })
    ));
    set_hyperlink_activation_handler(context, Some(activate));
    let key = v8::String::new(scope, "handled").unwrap();
    let function = v8::Function::new(scope, handled).unwrap();
    context
        .global(scope)
        .set(scope, key.into(), function.into());
    eval(scope, "globalThis.a=document.getElementById('a');globalThis.s=document.getElementById('s');globalThis.events=[];document.addEventListener('click',e=>events.push(e));");
    test(scope, &host);
}

#[test]
fn guest_click_and_bubbling_descendant_open_once() {
    fixture(|scope, host| {
        assert_js(scope, "a.click();s.dispatchEvent(new MouseEvent('click',{bubbles:true,cancelable:true}));document.getElementById('area').click();events.length===3&&events.every(handled)");
        assert_eq!(host.calls.get(), 3);
        assert_eq!(host.requests.borrow().len(), 3);
        assert_eq!(host.windows.borrow().len(), 3);
        assert_eq!(
            host.requests.borrow()[0].url,
            "https://example.test/base/child"
        );
    });
}

#[test]
fn canceled_nonprimary_and_nonanchor_dispatches_do_not_activate() {
    fixture(|scope, host| {
        assert_js(scope, "a.addEventListener('click',e=>e.preventDefault(),{once:true});a.click();a.dispatchEvent(new MouseEvent('click',{button:1,bubbles:true}));a.dispatchEvent(new MouseEvent('auxclick',{button:1,bubbles:true}));globalThis.secondary=new MouseEvent('click',{button:2,bubbles:true});Object.defineProperty(secondary,'button',{value:0});a.dispatchEvent(secondary);s.dispatchEvent(new MouseEvent('click'));document.getElementById('b').click();a.appendChild(document.getElementById('b'));document.getElementById('b').click();a.dispatchEvent(new Event('click',{bubbles:true}));events.every(e=>!handled(e))");
        assert_eq!(host.calls.get(), 0);
        assert!(host.requests.borrow().is_empty());
    });
}

#[test]
fn current_native_connection_and_href_are_rechecked_after_listeners() {
    fixture(|scope, host| {
        assert_js(scope, "a.addEventListener('click',()=>a.removeAttribute('href'),{once:true});a.click();a.href='child';a.addEventListener('click',()=>a.remove(),{once:true});a.click();a.click();events.every(e=>!handled(e))");
        assert_eq!(host.calls.get(), 0);
    });
}

#[test]
fn denied_and_noopener_null_are_consumed_without_driver_fallback() {
    fixture(|scope, host| {
        host.deny.set(true);
        assert_js(scope, "a.click();handled(events[0])");
        assert_eq!(host.requests.borrow().len(), 1);
        assert!(host.windows.borrow().is_empty());
        host.deny.set(false);
        assert_js(scope, "a.click();handled(events[1])");
        assert_eq!(host.requests.borrow().len(), 2);
        assert_eq!(host.windows.borrow().len(), 1);
        assert!(host.requests.borrow().last().unwrap().noopener);
        let child = v8::Local::new(scope, &host.windows.borrow()[0]);
        assert!(auxiliary_context_opener(scope, child).is_none());
    });
}

#[test]
fn private_receipt_resets_on_redispatch_and_cannot_be_forged() {
    fixture(|scope, host| {
        assert_js(scope, "globalThis.e=new MouseEvent('click',{bubbles:true,cancelable:true});a.dispatchEvent(e);handled(e)");
        host.consumed.set(false);
        assert_js(scope, "e.hyperlinkActivationHandled=true;Object.defineProperty(e,'button',{get(){throw Error('guest button')}});a.dispatchEvent(e);!handled(e)&&!handled({hyperlinkActivationHandled:true})");
        assert_eq!(host.calls.get(), 2);
        assert_eq!(host.requests.borrow().len(), 1);
        host.consumed.set(true);
        assert_js(scope, "a.addEventListener('click',e=>e.preventDefault(),{once:true});a.dispatchEvent(e);!handled(e)");
        assert_eq!(host.calls.get(), 2);
    });
}

#[test]
fn guest_getters_do_not_change_native_admission_or_cancellation() {
    fixture(|scope, host| {
        assert_js(scope, "for(const p of ['href','target','rel'])Object.defineProperty(a,p,{get(){throw Error('guest attribute')}});globalThis.e=new MouseEvent('click',{bubbles:true,cancelable:true});Object.defineProperty(e,'button',{value:2});a.dispatchEvent(e);handled(e)");
        assert_eq!(host.calls.get(), 1);
        assert_eq!(
            host.requests.borrow()[0].url,
            "https://example.test/base/child"
        );
        assert_js(scope, "globalThis.c=new MouseEvent('click',{bubbles:true,cancelable:true});Object.defineProperty(c,'defaultPrevented',{value:false});c.preventDefault();a.dispatchEvent(c);!handled(c)");
        assert_eq!(host.calls.get(), 1);
    });
}

#[test]
fn handler_can_be_removed_and_absence_is_not_consumed() {
    fixture(|scope, host| {
        let context = scope.get_current_context();
        set_hyperlink_activation_handler(context, None);
        assert_js(scope, "a.click();!handled(events[0])");
        assert_eq!(host.calls.get(), 0);
    });
}

#[test]
fn registration_is_local_to_the_elements_own_context() {
    fixture(|scope, host| {
        let template = auxiliary_window_template(scope);
        let other = v8::Context::new(
            scope,
            v8::ContextOptions {
                global_template: Some(template),
                ..Default::default()
            },
        );
        {
            let scope = &mut v8::ContextScope::new(scope, other);
            browser_env::install_independent_html(
                scope,
                "<!doctype html><a id=a href=child target=_blank>child</a>",
            );
            assert!(install_auxiliary_context(
                scope,
                other,
                "https://other.test/",
                Some(AuxiliaryHost { open, close })
            ));
            let event = eval(scope, "globalThis.e=new MouseEvent('click',{bubbles:true});document.getElementById('a').dispatchEvent(e);e");
            let event = v8::Local::<v8::Object>::try_from(event).unwrap();
            assert!(!hyperlink_activation_handled(scope, event));
        }
        assert_eq!(host.calls.get(), 0);
        assert_js(scope, "a.click();handled(events[0])");
        assert_eq!(host.calls.get(), 1);
    });
}

#[test]
fn retiring_context_capabilities_removes_the_registered_host() {
    fixture(|scope, host| {
        browser_env::clear_context_capabilities(scope);
        assert_js(
            scope,
            "(()=>{try{a.click();return false}catch(e){return e.name==='TypeError'}})()",
        );
        assert_eq!(host.calls.get(), 0);
        assert!(host.requests.borrow().is_empty());
    });
}
