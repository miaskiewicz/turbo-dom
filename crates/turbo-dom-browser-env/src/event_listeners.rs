//! Private phased Window dispatch and listener option capabilities.
use super::*;

struct WindowPhases(v8::Weak<v8::Function>);
struct ListenerParser(v8::Weak<v8::Function>);
struct SignalReader(v8::Weak<v8::Function>);
struct WindowInterfaces(v8::Weak<v8::Function>);
struct SignalSubscription(v8::Weak<v8::Function>);
struct ErrorGetters(Vec<v8::Weak<v8::Function>>);
fn records(scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue) {
    let operation = args.get(0).to_rust_string_lossy(scope);
    let Ok(object) = v8::Local::<v8::Object>::try_from(args.get(1)) else {
        return;
    };
    if operation == "targetChanged" {
        event_brands::notify_native_event_target(scope, object);
        return;
    }
    if operation == "subscription" {
        let Ok(hook) = v8::Local::<v8::Function>::try_from(args.get(2)) else {
            return;
        };
        let signal = v8::Weak::new(scope, object);
        let hook = v8::Weak::new(scope, hook);
        subscription(scope, &signal, &hook, args.get(3).is_true());
        return;
    }
    if operation == "aborted" {
        let Some(context) = object.get_creation_context(scope) else {
            return;
        };
        let Some(reader) = context.get_slot::<SignalReader>() else {
            return;
        };
        let reader = dom_ownership::local(scope, &reader.0);
        let receiver = v8::undefined(scope).into();
        if let Some(value) = reader.call(scope, receiver, &[object.into()]) {
            rv.set(value);
        }
        return;
    }
    if operation == "error" {
        let Some(context) = object.get_creation_context(scope) else {
            return;
        };
        let Some(getters) = context.get_slot::<ErrorGetters>() else {
            return;
        };
        if getters.0.len() != 5 {
            return;
        }
        let scope = std::pin::pin!(v8::TryCatch::new(scope));
        let scope = &mut scope.init();
        let mut values = Vec::with_capacity(5);
        for getter in &getters.0 {
            let getter = dom_ownership::local(scope, getter);
            let Some(value) = getter.call(scope, object.into(), &[]) else {
                return;
            };
            values.push(value);
        }
        rv.set(v8::Array::new_with_elements(scope, &values).into());
        return;
    }
    if operation == "event" {
        if let Some(record) = event_record(scope, object) {
            rv.set(record.into());
        }
        return;
    }
    let family = if operation.starts_with("target") {
        "target"
    } else if operation.starts_with("signal") {
        "signal"
    } else {
        "controller"
    };
    let name = v8::String::new(scope, &format!("turbo-dom.Abort.{family}")).unwrap();
    let key = v8::Private::for_api(scope, Some(name));
    if operation.ends_with("Set") {
        object.set_private(scope, key, args.get(2));
    }
    if let Some(value) = object.get_private(scope, key) {
        rv.set(value);
    }
}
pub(super) fn install(scope: &mut v8::PinScope) {
    let global = scope.get_current_context().global(scope);
    bind_method(scope, global, "__windowListenerNative", records);
    run_js(scope, include_str!("event_listeners.js"));
    for (name, family) in [
        ("__captureWindowEventPhase", 0),
        ("__captureListenerOptions", 1),
        ("__captureSignalAborted", 2),
        ("__captureWindowEventInterfaces", 3),
        ("__captureSignalSubscription", 4),
    ] {
        let key = v8::String::new(scope, name).unwrap();
        if let Some(function) = global
            .get(scope, key.into())
            .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
        {
            let Some(function) = dom_ownership::helper_function(scope, function) else {
                return;
            };
            let context = scope.get_current_context();
            match family {
                0 => {
                    context.set_slot(std::rc::Rc::new(WindowPhases(function)));
                }
                1 => {
                    context.set_slot(std::rc::Rc::new(ListenerParser(function)));
                }
                2 => {
                    context.set_slot(std::rc::Rc::new(SignalReader(function)));
                }
                3 => {
                    context.set_slot(std::rc::Rc::new(WindowInterfaces(function)));
                }
                _ => {
                    context.set_slot(std::rc::Rc::new(SignalSubscription(function)));
                }
            }
        }
        global.delete(scope, key.into());
    }
    capture_window_event_interfaces(scope);
}
fn owner<'s>(scope: &mut v8::PinScope<'s, '_>) -> Option<v8::Local<'s, v8::Context>> {
    let owner = DOM_CONTEXTS
        .with(|contexts| contexts.borrow().get(&DOM_ID.with(|id| id.get())).cloned())?;
    Some(dom_ownership::local(scope, &owner))
}
pub(super) fn window<'s>(scope: &mut v8::PinScope<'s, '_>) -> Option<v8::Local<'s, v8::Object>> {
    if !DOM.with(|dom| {
        dom.borrow()
            .as_ref()
            .is_some_and(|dom| dom.browsing_context)
    }) {
        return None;
    }
    Some(owner(scope)?.global(scope))
}
pub(super) fn phase(scope: &mut v8::PinScope, event: v8::Local<v8::Object>, capture: bool) {
    let Some(context) = owner(scope) else {
        return;
    };
    let Some(dispatcher) = context.get_slot::<WindowPhases>() else {
        return;
    };
    let function = dom_ownership::local(scope, &dispatcher.0);
    let flag = v8::Boolean::new(scope, capture);
    let receiver = v8::undefined(scope).into();
    function.call(scope, receiver, &[event.into(), flag.into()]);
}
pub(super) fn options<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: &v8::FunctionCallbackArguments,
    remove: bool,
    passive: bool,
) -> Option<v8::Local<'s, v8::Object>> {
    let context = scope.get_current_context();
    let parser = context.get_slot::<ListenerParser>()?;
    let function = dom_ownership::local(scope, &parser.0);
    let remove = v8::Boolean::new(scope, remove);
    let passive = v8::Boolean::new(scope, passive);
    let receiver = v8::undefined(scope).into();
    function
        .call(
            scope,
            receiver,
            &[
                args.get(0),
                args.get(1),
                args.get(2),
                remove.into(),
                passive.into(),
            ],
        )?
        .to_object(scope)
}
pub(super) fn aborted(scope: &mut v8::PinScope, signal: &v8::Weak<v8::Object>) -> bool {
    let Some(signal) = signal.to_local(scope) else {
        return true;
    };
    let context = signal
        .get_creation_context(scope)
        .unwrap_or_else(|| scope.get_current_context());
    let Some(reader) = context.get_slot::<SignalReader>() else {
        return true;
    };
    let function = dom_ownership::local(scope, &reader.0);
    let receiver = v8::undefined(scope).into();
    function
        .call(scope, receiver, &[signal.into()])
        .is_none_or(|value| value.is_true())
}

/// Capture trusted post-bootstrap interface getters without exposing a page setter.
pub fn capture_window_event_interfaces(scope: &mut v8::PinScope) {
    let owner = scope
        .get_current_context()
        .get_slot::<DomContextId>()
        .map_or(0, |id| id.0);
    let _owner = DomScope::enter(owner);
    range_contents::capture_interfaces(scope);
    let context = scope.get_current_context();
    let Some(updater) = context.get_slot::<WindowInterfaces>() else {
        return;
    };
    let function = dom_ownership::local(scope, &updater.0);
    let global = context.global(scope);
    let values = v8::Object::new(scope);
    let key = v8::String::new(scope, "DOMException").unwrap();
    if let Some(exception) = global
        .get(scope, key.into())
        .filter(|value| value.is_function())
    {
        set_prop(scope, values, "DOMException", exception);
    }
    for (interface, names) in [
        (
            "AbortSignal",
            &["aborted", "addEventListener", "removeEventListener"][..],
        ),
        (
            "ErrorEvent",
            &["message", "filename", "lineno", "colno", "error"][..],
        ),
    ] {
        let key = v8::String::new(scope, interface).unwrap();
        let Some(constructor) = global
            .get(scope, key.into())
            .and_then(|value| value.to_object(scope))
        else {
            continue;
        };
        let key = v8::String::new(scope, "prototype").unwrap();
        let Some(prototype) = constructor
            .get(scope, key.into())
            .and_then(|value| value.to_object(scope))
        else {
            continue;
        };
        let mut functions = Vec::with_capacity(names.len());
        let mut getters = Vec::new();
        for (index, name) in names.iter().enumerate() {
            let key = v8::String::new(scope, name).unwrap();
            let value = if interface == "AbortSignal" && index > 0 {
                prototype.get(scope, key.into())
            } else {
                prototype
                    .get_own_property_descriptor(scope, key.into())
                    .and_then(|value| value.to_object(scope))
                    .and_then(|descriptor| {
                        let key = v8::String::new(scope, "get").unwrap();
                        descriptor.get(scope, key.into())
                    })
            };
            if let Some(value) = value.filter(|value| value.is_function()) {
                functions.push(value);
                if interface == "ErrorEvent" {
                    let function = v8::Local::<v8::Function>::try_from(value).unwrap();
                    let Some(function) = dom_ownership::helper_function(scope, function) else {
                        return;
                    };
                    getters.push(function);
                }
            } else {
                functions.push(v8::undefined(scope).into());
            }
        }
        if interface == "ErrorEvent" {
            context.set_slot(std::rc::Rc::new(ErrorGetters(getters)));
        }
        let functions = v8::Array::new_with_elements(scope, &functions);
        set_prop(scope, values, interface, functions.into());
    }
    let receiver = v8::undefined(scope).into();
    function.call(scope, receiver, &[values.into()]);
}

#[derive(Clone)]
pub(super) struct Listener {
    pub callback: v8::Weak<v8::Object>,
    pub id: u32,
    pub node: v8::Weak<v8::Object>,
    pub capture: bool,
    pub once: bool,
    pub passive: bool,
    pub signal: Option<v8::Weak<v8::Object>>,
    pub removed: std::rc::Rc<std::cell::Cell<bool>>,
    pub abort_hook: Option<v8::Weak<v8::Function>>,
}
fn subscription(
    scope: &mut v8::PinScope,
    signal: &v8::Weak<v8::Object>,
    hook: &v8::Weak<v8::Function>,
    add: bool,
) {
    let Some(signal) = signal.to_local(scope) else {
        return;
    };
    let Some(context) = signal.get_creation_context(scope) else {
        return;
    };
    let Some(dispatcher) = context.get_slot::<SignalSubscription>() else {
        return;
    };
    let Some(function) = dispatcher.0.to_local(scope) else {
        return;
    };
    let Some(hook) = hook.to_local(scope) else {
        return;
    };
    let flag = v8::Boolean::new(scope, add);
    let receiver = v8::undefined(scope).into();
    function.call(scope, receiver, &[signal.into(), hook.into(), flag.into()]);
}
thread_local! {
    static NODE_SUBSCRIPTIONS: RefCell<HashMap<u32,v8::Weak<v8::Object>>> = RefCell::new(HashMap::new());
    static NEXT_LISTENER: std::cell::Cell<u32> = const { std::cell::Cell::new(1) };
}
#[derive(Default)]
pub(super) struct IsolationState {
    subscriptions: HashMap<u32, v8::Weak<v8::Object>>,
    next_listener: u32,
}
pub(super) fn isolation_available() -> bool {
    NODE_SUBSCRIPTIONS.try_with(|_| ()).is_ok()
}
pub(super) fn take_isolation_state() -> IsolationState {
    IsolationState {
        subscriptions: NODE_SUBSCRIPTIONS.with(|rows| std::mem::take(&mut *rows.borrow_mut())),
        next_listener: NEXT_LISTENER.with(|next| next.replace(1)),
    }
}
pub(super) fn put_isolation_state(state: IsolationState) {
    NODE_SUBSCRIPTIONS.with(|rows| *rows.borrow_mut() = state.subscriptions);
    NEXT_LISTENER.with(|next| next.set(state.next_listener.max(1)));
}
pub(super) fn reset_node_subscriptions() {
    let _ = NODE_SUBSCRIPTIONS.try_with(|state| state.borrow_mut().clear());
}
pub(super) fn own_listener<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    node: v8::Local<'s, v8::Object>,
    callback: v8::Local<'s, v8::Object>,
    signal: Option<v8::Local<'s, v8::Object>>,
) -> Option<u32> {
    let id = NEXT_LISTENER.with(|counter| {
        let id = counter.get();
        counter.set(id.checked_add(1)?);
        Some(id)
    })?;
    let record = v8::Object::new(scope);
    let null = v8::null(scope);
    record.set_prototype(scope, null.into())?;
    let key = v8::String::new(scope, "callback")?;
    record.create_data_property(scope, key.into(), callback.into())?;
    if let Some(signal) = signal {
        let key = v8::String::new(scope, "signal")?;
        record.create_data_property(scope, key.into(), signal.into())?;
    }
    dom_ownership::retain_listener(scope, node, id, record)?;
    Some(id)
}
pub(super) fn detach(scope: &mut v8::PinScope, listener: &Listener) {
    NODE_SUBSCRIPTIONS.with(|state| {
        state.borrow_mut().remove(&listener.id);
    });
    if let Some(node) = listener.node.to_local(scope) {
        dom_ownership::release_listener(scope, node, listener.id);
    }
    if let (Some(signal), Some(hook)) = (&listener.signal, &listener.abort_hook) {
        subscription(scope, signal, hook, false);
    }
}
fn abort_listener(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let Some(id) = args.data().uint32_value(scope) else {
        return;
    };
    let node = NODE_SUBSCRIPTIONS.with(|state| {
        state
            .borrow()
            .get(&id)
            .and_then(|node| node.to_local(scope))
    });
    let Some(node) = node else {
        NODE_SUBSCRIPTIONS.with(|state| {
            state.borrow_mut().remove(&id);
        });
        return;
    };
    let _dom = DomScope::for_object(scope, node);
    let removed = LISTENERS.with(|state| {
        let mut state = state.borrow_mut();
        let mut removed = Vec::new();
        for list in state.values_mut() {
            list.retain(|listener| {
                if listener.id == id {
                    listener.removed.set(true);
                    removed.push(listener.clone());
                    false
                } else {
                    true
                }
            });
        }
        removed
    });
    for listener in removed {
        detach(scope, &listener);
    }
}
pub(super) fn attach(
    scope: &mut v8::PinScope,
    node: v8::Local<v8::Object>,
    _kind: &str,
    listener: &mut Listener,
) {
    let Some(signal) = &listener.signal else {
        return;
    };
    let Some(signal_object) = signal.to_local(scope) else {
        return;
    };
    let Some(context) = signal_object.get_creation_context(scope) else {
        return;
    };
    let scope = &mut v8::ContextScope::new(scope, context);
    let data = v8::Integer::new_from_unsigned(scope, listener.id);
    let Some(hook) = v8::Function::builder(abort_listener)
        .data(data.into())
        .build(scope)
    else {
        return;
    };
    NODE_SUBSCRIPTIONS.with(|state| {
        state
            .borrow_mut()
            .insert(listener.id, v8::Weak::new(scope, node));
    });
    let hook = v8::Weak::new(scope, hook);
    subscription(scope, signal, &hook, true);
    listener.abort_hook = Some(hook);
}

pub(super) fn clear_context(context: v8::Local<v8::Context>) {
    context.remove_slot::<WindowPhases>();
    context.remove_slot::<ListenerParser>();
    context.remove_slot::<SignalReader>();
    context.remove_slot::<WindowInterfaces>();
    context.remove_slot::<SignalSubscription>();
    context.remove_slot::<ErrorGetters>();
}
pub(super) fn sweep_collected_listeners(scope: &mut v8::PinScope) {
    fn remove_dead(
        listeners: &mut HashMap<(Handle, String), Vec<Listener>>,
        removed: &mut Vec<Listener>,
    ) {
        listeners.retain(|_, list| {
            list.retain(|listener| {
                if listener.node.is_empty() {
                    listener.removed.set(true);
                    removed.push(listener.clone());
                    false
                } else {
                    true
                }
            });
            !list.is_empty()
        });
    }
    let mut removed = Vec::new();
    LISTENERS.with(|state| remove_dead(&mut state.borrow_mut(), &mut removed));
    OTHER_DOMS.with(|states| {
        for state in states.borrow_mut().values_mut() {
            remove_dead(&mut state.listeners, &mut removed);
        }
    });
    for listener in removed {
        detach(scope, &listener);
    }
    NODE_SUBSCRIPTIONS.with(|state| state.borrow_mut().retain(|_, node| !node.is_empty()));
}
#[cfg(test)]
pub(super) fn subscription_count() -> usize {
    NODE_SUBSCRIPTIONS.with(|state| state.borrow().len())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn foreign_events_and_nodes_keep_their_native_window_owner() {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let parent = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, parent);
        super::super::install(scope);
        run_js(scope, "globalThis.hits=[];addEventListener('foreign',event=>hits.push('parent'));window.onerror=function(message,source,line,column,error){hits.push([message,source,line,column,error===child.failure,arguments.length]);return true};");
        let child = v8::Context::new(scope, Default::default());
        child.set_security_token(parent.get_security_token(scope));
        {
            let scope = &mut v8::ContextScope::new(scope, child);
            install_independent_html(scope, "<!doctype html><body><span id=target></span>");
            run_js(scope, "globalThis.hits=[];globalThis.node=document.getElementById('target');globalThis.eventTarget=new EventTarget();globalThis.targetHits=[];eventTarget.addEventListener('generic',function(event){targetHits.push([this===eventTarget,event.eventPhase])});globalThis.failure=new Error('foreign');globalThis.errorEvent=new ErrorEvent('error',{message:'foreign',filename:'child.js',lineno:9,colno:3,error:failure,cancelable:true});addEventListener('foreign',function(event){hits.push([event.currentTarget===window,event.target===node,event.composedPath().at(-1)===window])},true);Object.defineProperty(document,'defaultView',{value:{}});Object.defineProperty(ErrorEvent.prototype,'message',{get(){throw new Error('guest getter')}});");
        }
        let scope = &mut v8::ContextScope::new(scope, parent);
        let global = parent.global(scope);
        let child_global = child.global(scope);
        set_prop(scope, global, "child", child_global.into());
        let source = v8::String::new(scope, "child.node.dispatchEvent(new Event('foreign',{bubbles:true}));const cancelled=!window.dispatchEvent(child.errorEvent);let signalled=0;const signal=new child.AbortController();const callback=()=>signalled++;addEventListener('foreign-signal',callback,{signal:signal.signal});signal.abort();addEventListener('foreign-signal',callback);dispatchEvent(new Event('foreign-signal'));EventTarget.prototype.dispatchEvent.call(child.eventTarget,new Event('generic'));JSON.stringify([hits,child.hits,cancelled,signalled,child.targetHits]);").unwrap();
        let result = v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap();
        assert_eq!(
            result.to_rust_string_lossy(scope),
            r#"[[["foreign","child.js",9,3,true,5]],[[true,true,true]],true,1,[[true,2]]]"#
        );
        reset();
    }
    #[test]
    fn window_listener_failure_preserves_event_state() {
        fixture(
            r#"(()=>{let rows=[];const order=[];window.addEventListener('order',()=>order.push('bubble'));window.addEventListener('order',()=>order.push('capture'),true);window.dispatchEvent(new Event('order'));if(order.join()!=='bubble,capture')throw new Error('direct Window target order '+order);window.addEventListener('direct',()=>{throw new Error('listener failure')},true);window.addEventListener('direct',e=>rows.push([e.currentTarget===window,e.eventPhase,e.isTrusted,e.composedPath().length]));const event=new Event('direct');window.dispatchEvent(event);if(JSON.stringify(rows)!=='[[true,2,false,1]]'||event.currentTarget!==null||event.eventPhase!==0)throw new Error('listener cleanup '+JSON.stringify(rows));window.addEventListener('immediate',e=>{rows.push('capture');e.stopImmediatePropagation()},true);window.addEventListener('immediate',()=>rows.push('bad'));window.dispatchEvent(new Event('immediate'));if(rows.at(-1)!=='capture')throw new Error('immediate');let brand;try{window.dispatchEvent(Object.create(Event.prototype))}catch(error){brand=error.name}if(brand!=='TypeError')throw new Error('forged event');const node=document.createElement('div'),NativeException=DOMException;let intercepted=0;DOMException=function(){intercepted++;return new Error('fake')};let native=false;try{node.dispatchEvent(new Event(''))}catch(error){native=error instanceof NativeException&&error.name==='InvalidStateError'}if(!native||intercepted)throw new Error('public constructor intercepted dispatch');return true})()"#,
        );
    }
    #[test]
    fn refreshed_private_abort_errors_keep_their_realm_and_genuine_brand() {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope,isolate);
        let parent = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, parent);
        super::super::install(scope);
        let replacement="globalThis.HostException=class DOMException extends globalThis.DOMException {};globalThis.DOMException=HostException";
        run_js(scope, replacement);
        capture_window_event_interfaces(scope);
        let child = v8::Context::new(scope, Default::default());
        child.set_security_token(parent.get_security_token(scope));
        {
            let scope = &mut v8::ContextScope::new(scope, child);
            install_independent_html(scope, "<!doctype html><body>");
            run_js(scope, replacement);
            capture_window_event_interfaces(scope);
            run_js(scope,"globalThis.controller=new AbortController();globalThis.DOMException=function(){throw new Error('guest exception constructor')};controller.abort();");
        }
        let scope = &mut v8::ContextScope::new(scope, parent);
        let global = parent.global(scope);
        let child_global = child.global(scope);
        set_prop(scope, global, "child", child_global.into());
        let code=v8::String::new(scope,"(()=>{DOMException=function(){throw new Error('guest exception constructor')};const controller=new AbortController(),node=document.createElement('div');let calls=0;node.addEventListener('abort-test',()=>calls++,{once:true,signal:child.controller.signal});node.dispatchEvent(new Event('abort-test'));controller.abort();const reason=controller.signal.reason,foreign=child.controller.signal.reason;return reason instanceof HostException&&reason.name==='AbortError'&&reason.code===20&&foreign instanceof child.HostException&&!(foreign instanceof HostException)&&calls===0&&reason.message==='signal is aborted without reason'})()").unwrap();
        assert!(v8::Script::compile(scope, code, None)
            .unwrap()
            .run(scope)
            .unwrap()
            .is_true());
        reset();
    }
    #[test]
    fn projected_node_constructor_tracks_the_public_listener_backed_event_target() {
        fixture(
            r#"(()=>{if(Object.getPrototypeOf(Node)!==EventTarget||Object.getPrototypeOf(Node.prototype)!==EventTarget.prototype||Node.prototype.constructor!==Node||EventTarget.prototype.constructor!==EventTarget||Object.getPrototypeOf(Element)!==Node||Object.getPrototypeOf(Document)!==Node||Object.getPrototypeOf(AbortSignal)!==EventTarget)throw new Error('constructor inheritance');const node=document.createElement('div');let called=0;EventTarget.prototype.addEventListener.call(node,'linked',()=>called++);node.dispatchEvent(new Event('linked'));if(called!==1||!(node instanceof EventTarget)||Object.getOwnPropertyDescriptor(Node,'prototype').writable)throw new Error('node prototype changed');return true})()"#,
        );
    }
    #[test]
    fn generic_targets_and_abort_events_have_private_real_listener_state() {
        fixture(
            r#"(()=>{const target=new EventTarget(),rows=[];target.addEventListener('probe',e=>rows.push(['bubble',e.eventPhase,e.currentTarget===target,e.target===target,e.composedPath().length]));target.addEventListener('probe',e=>rows.push(['capture',e.eventPhase,e.currentTarget===target,e.target===target,e.composedPath().length]),true);target.dispatchEvent(new Event('probe'));if(JSON.stringify(rows)!=='[["bubble",2,true,true,1],["capture",2,true,true,1]]'||Reflect.ownKeys(target).length)throw new Error('target '+JSON.stringify(rows));let called=0;const controller=new AbortController(),callback=()=>called++;controller.signal.onabort=callback;controller.signal.addEventListener('abort',callback);let abort;controller.signal.addEventListener('abort',e=>abort=[e.isTrusted,e.eventPhase,e.currentTarget===controller.signal,e.target===controller.signal]);controller.abort();controller.abort();if(called!==2||JSON.stringify(abort)!=='[true,2,true,true]'||Reflect.ownKeys(controller.signal).length||controller.signal.reason.name!=='AbortError'||controller.signal.reason.code!==20||!(controller.signal.reason instanceof DOMException))throw new Error('abort '+called+' '+JSON.stringify(abort)+' '+Reflect.ownKeys(controller.signal));class Custom extends EventTarget{}const custom=new Custom();custom.addEventListener('x',()=>called++);custom.dispatchEvent(new Event('x'));if(!(custom instanceof EventTarget)||Reflect.ownKeys(custom).length||called!==3)throw new Error('subclass');return true})()"#,
        );
    }
    #[test]
    fn generic_target_options_abort_removal_stop_and_opaque_branding() {
        fixture(
            r#"(()=>{const target=new EventTarget();let calls=0;const once=()=>{calls++;target.dispatchEvent(new Event('once'))};target.addEventListener('once',once,{once:true});target.dispatchEvent(new Event('once'));if(calls!==1)throw new Error('once');let passive;target.addEventListener('passive',e=>{e.preventDefault();e.returnValue=false;passive=e.defaultPrevented},{passive:true});if(!target.dispatchEvent(new Event('passive',{cancelable:true}))||passive)throw new Error('passive');const controller=new AbortController(),callback=()=>calls++;target.addEventListener('signal',callback,{signal:controller.signal});controller.abort();target.addEventListener('signal',callback);target.dispatchEvent(new Event('signal'));if(calls!==2)throw new Error('signal');let rows=[];target.addEventListener('stop',e=>{rows.push('capture');e.stopPropagation()},true);target.addEventListener('stop',()=>rows.push('bubble'));target.dispatchEvent(new Event('stop'));if(rows.join()!=='capture,bubble')throw new Error('stop');rows=[];target.addEventListener('immediate',e=>{rows.push('capture');e.stopImmediatePropagation()},true);target.addEventListener('immediate',()=>rows.push('bad'));target.dispatchEvent(new Event('immediate'));if(rows.join()!=='capture')throw new Error('immediate');const forged=Object.create(EventTarget.prototype);let error;try{EventTarget.prototype.addEventListener.call(forged,'x',()=>{})}catch(caught){error=caught.name}if(error!=='TypeError')throw new Error('forged');const nativeSlice=Array.prototype.slice,nativePush=Array.prototype.push;Array.prototype.slice=Array.prototype.push=function(){throw new Error('guest intrinsic')};const protectedTarget=new EventTarget();protectedTarget.addEventListener('x',()=>calls++);protectedTarget.dispatchEvent(new Event('x'));Array.prototype.slice=nativeSlice;Array.prototype.push=nativePush;if(calls!==3||Reflect.ownKeys(protectedTarget).length)throw new Error('private array poisoning');return true})()"#,
        );
    }
    #[test]
    fn hostile_array_and_object_hooks_never_observe_private_listener_records() {
        fixture(
            r#"(()=>{const define=Object.defineProperty,descriptor=Object.getOwnPropertyDescriptor,iterator=Symbol.iterator,species=Symbol.species;let cases=0;function scenario(install,restore){let hooks=0,calls=0,aborts=0;const poison=()=>{hooks++;throw new Error('private record observed')};const target=new EventTarget(),controller=new AbortController(),event=new Event('owned'),listener=()=>calls++;try{install(poison);controller.signal.addEventListener('abort',()=>aborts++,{once:true});target.addEventListener('owned',listener,{signal:controller.signal});target.dispatchEvent(event);target.removeEventListener('owned',listener);target.addEventListener('owned',listener,{once:true});target.dispatchEvent(event);target.dispatchEvent(event);window.addEventListener('owned',listener,{signal:controller.signal});window.dispatchEvent(event);controller.abort('reason');window.dispatchEvent(event);if(hooks||calls!==3||aborts!==1||Reflect.ownKeys(target).length||Reflect.ownKeys(controller.signal).length)throw new Error('privacy '+hooks+' '+calls+' '+aborts);cases++;}finally{restore();window.removeEventListener('owned',listener)}}const zero=descriptor(Array.prototype,'0');scenario(poison=>define(Array.prototype,'0',{configurable:true,set:poison}),()=>{if(zero)define(Array.prototype,'0',zero);else delete Array.prototype[0]});const constructor=descriptor(Array.prototype,'constructor');scenario(poison=>{const replacement=Object.create(null);define(replacement,species,{get:poison});define(Array.prototype,'constructor',{...constructor,value:replacement})},()=>define(Array.prototype,'constructor',constructor));const iter=descriptor(Array.prototype,iterator);scenario(poison=>define(Array.prototype,iterator,{...iter,value:poison}),()=>define(Array.prototype,iterator,iter));const listeners=descriptor(Object.prototype,'listeners'),handlers=descriptor(Object.prototype,'handlers');scenario(poison=>{define(Object.prototype,'listeners',{configurable:true,set:poison});define(Object.prototype,'handlers',{configurable:true,set:poison})},()=>{if(listeners)define(Object.prototype,'listeners',listeners);else delete Object.prototype.listeners;if(handlers)define(Object.prototype,'handlers',handlers);else delete Object.prototype.handlers});return cases===4})()"#,
        );
    }
    #[test]
    fn native_node_signal_data_and_window_error_arguments_bypass_guest_index_setters() {
        fixture(
            r#"(()=>{const node=document.createElement('div');document.body.append(node);const controller=new AbortController();let hooks=0,calls=0,errors=0;const callback=()=>calls++;window.onerror=function(message,source,line,column,error){if(message==='authored'&&source==='fixture.js'&&line===4&&column===2&&error===failure)errors++;return true};const failure=new Error('authored');Object.defineProperty(Array.prototype,'0',{configurable:true,set(){hooks++}});let returned;try{node.addEventListener('private',callback,{signal:controller.signal});controller.abort('reason');node.addEventListener('private',callback);const event=new ErrorEvent('error',{message:'authored',filename:'fixture.js',lineno:4,colno:2,error:failure,cancelable:true});returned=window.dispatchEvent(event)}finally{delete Array.prototype[0];window.onerror=null}node.dispatchEvent(new Event('private'));if(hooks!==0||calls!==1||errors!==1||returned!==false)throw new Error('native array privacy '+hooks+' '+calls+' '+errors+' '+returned);return true})()"#,
        );
    }
    #[test]
    fn native_dom_snapshots_and_event_paths_bypass_guest_array_hooks() {
        fixture(
            r#"(()=>{const node=document.createElement('div'),child=document.createElement('i');node.setAttribute('authored','value');node.append(child);document.body.append(node);const define=Object.defineProperty,constructor=Object.getOwnPropertyDescriptor(Array.prototype,'constructor'),iterator=Object.getOwnPropertyDescriptor(Array.prototype,Symbol.iterator);let hooks=0,calls=0,path;node.addEventListener('owned',e=>{calls++;path=e.composedPath();path.length=0;if(e.composedPath()[0]!==node)throw new Error('private path changed')});define(Array.prototype,'0',{configurable:true,set(){hooks++}});define(Array.prototype,'constructor',{configurable:true,get(){hooks++;throw new Error('private array species '+new Error().stack)}});define(Array.prototype,Symbol.iterator,{configurable:true,value(){hooks++;throw new Error('private array iterator')}});let snapshots;try{snapshots=node.children.item(0)===child&&node.childNodes.item(0)===child&&node.attributes.item(0).name==='authored'&&node.querySelectorAll('i').item(0)===child&&document.querySelectorAll('i').item(0)===child&&Object.keys(node.children).join()==='0';node.dispatchEvent(new Event('owned',{bubbles:true}));}finally{delete Array.prototype[0];define(Array.prototype,'constructor',constructor);define(Array.prototype,Symbol.iterator,iterator)}if(hooks!==0||calls!==1||!snapshots||path.length!==0)throw new Error('DOM array privacy '+hooks+' '+calls+' '+snapshots);return true})()"#,
        );
    }
    fn fixture(source: &str) {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        super::super::install(scope);
        let inert = parse_html_document(scope, "<!doctype html><body><div></div>", false);
        let global = context.global(scope);
        set_prop(scope, global, "inert", inert.into());
        let scope = std::pin::pin!(v8::TryCatch::new(scope));
        let scope = &mut scope.init();
        let source = v8::String::new(scope, source).unwrap();
        let result = v8::Script::compile(scope, source, None).and_then(|script| script.run(scope));
        if result.is_none() {
            panic!(
                "event fixture: {:?}",
                scope
                    .exception()
                    .map(|error| error.to_rust_string_lossy(scope))
            );
        }
        assert!(result.unwrap().is_true());
        reset();
    }
    #[test]
    fn window_document_node_phases_path_and_cleanup() {
        fixture(
            r#"(()=>{const node=document.createElement('span');document.body.append(node);let rows=[];const add=(target,name,capture)=>target.addEventListener('probe',function(e){rows.push([name,e.eventPhase,e.currentTarget===this,e.target===node,e.composedPath().at(-1)===window]);},capture);add(window,'window-capture',true);add(document,'document-capture',true);add(node,'target-bubble',false);add(node,'target-capture',true);add(document,'document-bubble',false);add(window,'window-bubble',false);const event=new Event('probe',{bubbles:true});node.dispatchEvent(event);const wanted=[['window-capture',1,true,true,true],['document-capture',1,true,true,true],['target-capture',2,true,true,true],['target-bubble',2,true,true,true],['document-bubble',3,true,true,true],['window-bubble',3,true,true,true]];if(JSON.stringify(rows)!==JSON.stringify(wanted))throw new Error(JSON.stringify(rows));if(event.currentTarget!==null||event.eventPhase!==0||event.composedPath().length!==0||event.target!==node)throw new Error('cleanup');return true})()"#,
        );
    }
    #[test]
    fn nonbubbling_resource_error_reaches_window_capture_load_excludes_window() {
        fixture(
            r#"(()=>{const node=document.createElement('script');document.body.append(node);let rows=[];for(const type of ['error','load'])for(const [target,name] of [[window,'window'],[document,'document'],[node,'node']])for(const capture of [true,false])target.addEventListener(type,e=>rows.push([type,name,capture,e.eventPhase]),capture);node.dispatchEvent(new Event('error'));node.dispatchEvent(new Event('load',{bubbles:true}));const wanted=[['error','window',true,1],['error','document',true,1],['error','node',true,2],['error','node',false,2],['load','document',true,1],['load','node',true,2],['load','node',false,2],['load','document',false,3]];if(JSON.stringify(rows)!==JSON.stringify(wanted))throw new Error(JSON.stringify(rows));return true})()"#,
        );
    }
    #[test]
    fn listeners_once_removal_passive_objects_and_stop_contracts() {
        fixture(
            r#"(()=>{const node=document.createElement('div');document.body.append(node);let rows=[];const second=()=>rows.push('second');node.addEventListener('remove',()=>{rows.push('first');node.removeEventListener('remove',second)});node.addEventListener('remove',second);node.dispatchEvent(new Event('remove'));let calls=0;const once=()=>{calls++;node.dispatchEvent(new Event('once'))};node.addEventListener('once',once,{once:true});node.dispatchEvent(new Event('once'));node.dispatchEvent(new Event('once'));if(calls!==1||rows.join()!=='first')throw new Error('once/removal '+rows+calls);node.addEventListener('passive',e=>{e.preventDefault();e.returnValue=false},{passive:true});const passive=new Event('passive',{cancelable:true});if(!node.dispatchEvent(passive)||passive.defaultPrevented)throw new Error('passive');let objectThis=false;const object={handleEvent(e){objectThis=this===object&&e.currentTarget===node}};node.addEventListener('object',object);node.dispatchEvent(new Event('object'));if(!objectThis)throw new Error('object');rows=[];node.addEventListener('stop',e=>{rows.push('target-capture');e.stopPropagation()},true);node.addEventListener('stop',()=>rows.push('target-bubble'));window.addEventListener('stop',()=>rows.push('window-bubble'));node.dispatchEvent(new Event('stop',{bubbles:true}));if(rows.join()!=='target-capture,target-bubble')throw new Error('target stop '+rows);rows=[];window.addEventListener('blocked',e=>{rows.push('window');e.stopPropagation()},true);document.addEventListener('blocked',()=>rows.push('document'),true);node.addEventListener('blocked',()=>rows.push('node'));node.dispatchEvent(new Event('blocked',{bubbles:true}));if(rows.join()!=='window')throw new Error('ancestor stop '+rows);return true})()"#,
        );
    }
    #[test]
    fn abort_signals_remove_native_and_window_listeners_before_readdition() {
        fixture(
            r#"(()=>{let rows=[];const node=document.createElement('div');document.body.append(node);for(const [target,name] of [[node,'node'],[window,'window']]){const controller=new AbortController(),callback=()=>rows.push(name);target.addEventListener('signal',callback,{signal:controller.signal});controller.abort('reason');target.addEventListener('signal',callback);target.dispatchEvent(new Event('signal'));if(!controller.signal.aborted||controller.signal.reason!=='reason')throw new Error('state');let reason;try{controller.signal.throwIfAborted()}catch(error){reason=error}if(reason!=='reason')throw new Error('reason');target.removeEventListener('signal',callback);target.addEventListener('signal',()=>rows.push('bad'),{signal:AbortSignal.abort()});target.dispatchEvent(new Event('signal'));}if(rows.join()!=='node,window')throw new Error(rows);let queried=0;const signal={get aborted(){queried++;return false}};try{node.addEventListener('x',()=>{},{signal});throw new Error('accepted fake')}catch(error){if(error.name!=='TypeError')throw error}if(queried)throw new Error('guest getter');return true})()"#,
        );
    }
    #[test]
    fn window_onerror_five_arguments_and_ordinary_handler_cancellation() {
        fixture(
            r#"(()=>{const failure=new Error('boom'),rows=[];window.addEventListener('error',()=>rows.push('before'));window.onerror=function(message,source,line,column,error){rows.push([message,source,line,column,error===failure,arguments.length,this===window]);return true};window.addEventListener('error',()=>rows.push('after'));const event=new ErrorEvent('error',{message:'boom',filename:'fixture.js',lineno:4,colno:7,error:failure,cancelable:true});if(window.dispatchEvent(event)||!event.defaultPrevented)throw new Error('onerror cancellation');if(JSON.stringify(rows)!==JSON.stringify(['before',['boom','fixture.js',4,7,true,5,true],'after']))throw new Error(JSON.stringify(rows));let ordinary;window.onerror=function(e){ordinary=[arguments.length,e instanceof Event];return false};const resource=new Event('error',{cancelable:true});if(window.dispatchEvent(resource)||JSON.stringify(ordinary)!=='[1,true]')throw new Error('ordinary error');window.onmessage=function(e){return false};const message=new Event('message',{cancelable:true});if(window.dispatchEvent(message))throw new Error('ordinary cancellation');window.onerror=null;let calls=0;const callback=()=>calls++;window.onmessage=callback;addEventListener('message',callback);dispatchEvent(new Event('message'));removeEventListener('message',callback);dispatchEvent(new Event('message'));if(calls!==3)throw new Error('handler listener identity '+calls);window.onmessage=null;return true})()"#,
        );
    }
    #[test]
    fn dispatch_receiver_options_and_detached_document_security() {
        fixture(
            r#"(()=>{let rows=[];const node=document.createElement('div');document.body.append(node);const options={get capture(){rows.push('capture');return false},get once(){rows.push('once');return true},get passive(){rows.push('passive');return true},get signal(){rows.push('signal');return null}};const fn=()=>{};node.addEventListener('x',fn,options);if(rows.join()!=='capture,once,passive,signal')throw new Error(rows);rows=[];node.removeEventListener('x',fn,options);if(rows.join()!=='capture')throw new Error('remove options '+rows);let count=0;addEventListener('bare',()=>count++);EventTarget.prototype.dispatchEvent.call(window,new Event('bare'));if(count!==1)throw new Error('bare');const fake={};Object.defineProperty(inert,'defaultView',{value:fake});window.addEventListener('inert',()=>count++);inert.body.firstChild.dispatchEvent(new Event('inert',{bubbles:true}));if(count!==1)throw new Error('inert window');let reached=0;window.addEventListener('shadow',()=>reached++);const host=document.createElement('div');document.body.append(host);const shadow=host.attachShadow({mode:'closed'}),inside=document.createElement('span');shadow.append(inside);inside.dispatchEvent(new Event('shadow',{bubbles:true,composed:false}));if(reached)throw new Error('shadow privacy');return true})()"#,
        );
    }
    #[test]
    fn window_storage_handler_is_a_real_listener_slot() {
        fixture(
            r#"(()=>{const d=Object.getOwnPropertyDescriptor(window,'onstorage');if(onstorage!==null||!d.get||!d.set||!d.enumerable||!d.configurable)throw new Error('storage handler descriptor');const rows=[];addEventListener('storage',()=>rows.push('before'));onstorage=function(event){rows.push(this===window&&event.type==='storage'?'handler':'bad')};addEventListener('storage',()=>rows.push('after'));dispatchEvent(new Event('storage'));onstorage=()=>rows.push('replaced');dispatchEvent(new Event('storage'));onstorage={};if(onstorage!==null)throw new Error('noncallable handler');dispatchEvent(new Event('storage'));if(rows.join()!=='before,handler,after,before,replaced,after,before,after')throw new Error('storage listener order '+rows);let error;try{d.get.call({})}catch(e){error=e.name}if(error!=='TypeError')throw new Error('storage handler brand');return true})()"#,
        );
    }
}
