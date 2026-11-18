//! Native auxiliary WindowProxy identity and cross-origin access admission.
//!
//! Network navigation, resource policy and message delivery belong to the host.
//! This module only dispatches those capabilities; missing capabilities fail
//! explicitly rather than pretending that a navigation or message succeeded.
use super::NamedMutationResult;
use std::cell::RefCell;
use std::rc::Rc;

const _: () = assert!(v8::FORK_WINDOW_PROXY);

pub type LocationHost = for<'s, 'i> fn(
    &mut v8::PinScope<'s, 'i>,
    v8::Local<'s, v8::Context>,
) -> Option<v8::Local<'s, v8::Value>>;
pub type NavigationHost = for<'s, 'i> fn(
    &mut v8::PinScope<'s, 'i>,
    v8::Local<'s, v8::Context>,
    v8::Local<'s, v8::Context>,
    v8::Local<'s, v8::Value>,
) -> bool;
pub type MessageHost = for<'s, 'i> fn(
    &mut v8::PinScope<'s, 'i>,
    v8::Local<'s, v8::Context>,
    v8::Local<'s, v8::Context>,
    v8::Local<'s, v8::Value>,
    v8::Local<'s, v8::Value>,
    v8::Local<'s, v8::Value>,
) -> bool;
pub type ActionHost = for<'s, 'i> fn(
    &mut v8::PinScope<'s, 'i>,
    v8::Local<'s, v8::Context>,
    v8::Local<'s, v8::Context>,
    bool,
) -> bool;
pub type ChildrenHost = for<'s, 'i> fn(
    &mut v8::PinScope<'s, 'i>,
    v8::Local<'s, v8::Context>,
) -> Vec<v8::Local<'s, v8::Context>>;

/// Trusted capabilities for a native auxiliary context. Callbacks must enforce
/// their own navigation, structured-clone, target-origin and resource policies.
#[derive(Clone, Copy, Default)]
pub struct HostHooks {
    pub location: Option<LocationHost>,
    pub navigate: Option<NavigationHost>,
    pub post_message: Option<MessageHost>,
    /// `true` requests focus; `false` requests blur.
    pub focus: Option<ActionHost>,
    pub children: Option<ChildrenHost>,
}

/// Install capabilities before publishing a context to guest code. The slots
/// contain function pointers only, never isolate-rooting persistent handles.
pub fn install_host_hooks(context: v8::Local<v8::Context>, hooks: HostHooks) {
    context.set_slot(Rc::new(hooks));
}

fn access_allowed<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    accessing: v8::Local<'s, v8::Context>,
    accessed: v8::Local<'s, v8::Object>,
    _data: v8::Local<'s, v8::Value>,
) -> bool {
    accessed.get_creation_context(scope).is_some_and(|owner| {
        owner
            .get_security_token(scope)
            .strict_equals(accessing.get_security_token(scope))
    })
}

struct WindowDocument {
    proxy: v8::Weak<v8::Object>,
    lexical: v8::Weak<v8::Object>,
    document: v8::Weak<v8::Value>,
}

/// Guard the canonical native Document through this context's stable proxy.
/// Install after the independent DOM and origin, before any guest execution.
/// Retained functions of an old lexical realm must not read a replacement
/// document after that proxy commits a different origin.
pub fn install_document(scope: &mut v8::PinScope, context: v8::Local<v8::Context>) -> bool {
    if context.get_slot::<WindowDocument>().is_some() {
        return true;
    }
    let scope = &mut v8::ContextScope::new(scope, context);
    let proxy = context.global(scope);
    let name = v8::String::new(scope, "turbo-dom.ownership.window-document").unwrap();
    let key = v8::Private::for_api(scope, Some(name));
    let Some(document) = proxy
        .get_private(scope, key)
        .filter(|value| !value.is_undefined())
    else {
        return false;
    };
    let Some(lexical) = proxy
        .get_prototype(scope)
        .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
    else {
        return false;
    };
    let token = context.get_security_token(scope);
    let values = v8::Array::new_with_elements(scope, &[proxy.into(), document, token]);
    let name = v8::String::new(scope, "turbo-dom.auxiliary.lexical-window").unwrap();
    let key = v8::Private::for_api(scope, Some(name));
    if lexical.set_private(scope, key, values.into()) != Some(true) {
        return false;
    }
    let getter = v8::Function::builder(ext!(FunctionCallback, document_get))
        .constructor_behavior(v8::ConstructorBehavior::Throw)
        .build(scope)
        .unwrap();
    let name = v8::String::new(scope, "get document").unwrap();
    getter.set_name(name);
    let undefined = v8::undefined(scope);
    let mut descriptor = v8::PropertyDescriptor::new_from_get_set(getter.into(), undefined.into());
    descriptor.set_enumerable(true);
    descriptor.set_configurable(false);
    let name = v8::String::new(scope, "document").unwrap();
    if proxy.define_property(scope, name.into(), &descriptor) != Some(true) {
        return false;
    }
    context.set_slot(Rc::new(WindowDocument {
        proxy: v8::Weak::new(scope, proxy),
        lexical: v8::Weak::new(scope, lexical),
        document: v8::Weak::new(scope, document),
    }));
    true
}

fn document_get<'s, 'i>(
    scope: &mut v8::PinScope<'s, 'i>,
    args: v8::FunctionCallbackArguments<'s>,
    mut result: v8::ReturnValue<'s>,
) {
    let source = scope.get_current_context();
    let source_record = source.get_slot::<WindowDocument>();
    let lexical_receiver = source_record
        .as_ref()
        .and_then(|record| record.lexical.to_local(scope))
        .is_some_and(|lexical| lexical.strict_equals(args.this().into()));
    let window = if lexical_receiver {
        source_record.and_then(|record| record.proxy.to_local(scope))
    } else {
        Some(args.this())
    };
    let Some(window) = window else { return };
    let Some(active) = window.get_creation_context(scope) else {
        return;
    };
    if !active.global(scope).strict_equals(window.into()) {
        let message = v8::String::new(scope, "Illegal invocation").unwrap();
        let error = v8::Exception::type_error(scope, message);
        scope.throw_exception(error);
        return;
    }
    if !source
        .get_security_token(scope)
        .strict_equals(active.get_security_token(scope))
    {
        deny(scope);
        return;
    }
    let document = active
        .get_slot::<WindowDocument>()
        .and_then(|record| record.document.to_local(scope));
    if let Some(document) = document {
        result.set(document);
    }
}

struct ExceptionConstructor(v8::Weak<v8::Function>);

/// Capture the pristine realm's DOMException after installing its DOM and before
/// guest code. Denied access never reads a replaceable global constructor.
pub fn capture_exception(scope: &mut v8::PinScope, context: v8::Local<v8::Context>) -> bool {
    if context.get_slot::<ExceptionConstructor>().is_some() {
        return true;
    }
    let scope = &mut v8::ContextScope::new(scope, context);
    let global = context.global(scope);
    let name = v8::String::new(scope, "DOMException").unwrap();
    let Some(constructor) = global
        .get(scope, name.into())
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
    else {
        return false;
    };
    let Some(lexical) = global
        .get_prototype(scope)
        .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
    else {
        return false;
    };
    let name = v8::String::new(scope, "turbo-dom.auxiliary.exception").unwrap();
    let key = v8::Private::for_api(scope, Some(name));
    if lexical.set_private(scope, key, constructor.into()) != Some(true) {
        return false;
    }
    context.set_slot(Rc::new(ExceptionConstructor(v8::Weak::new(
        scope,
        constructor,
    ))));
    true
}

fn named_exception(scope: &mut v8::PinScope, name: &str, text: &str) {
    let context = scope.get_current_context();
    let message = v8::String::new(scope, text).unwrap();
    let error_name = v8::String::new(scope, name).unwrap();
    if let Some(constructor) = context.get_slot::<ExceptionConstructor>() {
        if let Some(constructor) = constructor.0.to_local(scope) {
            if let Some(error) =
                constructor.new_instance(scope, &[message.into(), error_name.into()])
            {
                scope.throw_exception(error.into());
            }
            return;
        }
    }
    let error = v8::Exception::error(scope, message);
    let object = v8::Local::<v8::Object>::try_from(error).unwrap();
    let key = v8::String::new(scope, "name").unwrap();
    object.create_data_property(scope, key.into(), error_name.into());
    scope.throw_exception(error);
}

fn deny(scope: &mut v8::PinScope) {
    named_exception(scope, "SecurityError", "Blocked cross-origin window access");
}

fn unsupported(scope: &mut v8::PinScope) {
    named_exception(
        scope,
        "NotSupportedError",
        "The host did not admit this window capability",
    );
}

/// Register this isolate callback when installing native access checks. It does
/// not read guest DOMException/global properties while handling denied access.
pub fn failed_access(
    scope: &mut v8::PinScope,
    _target: v8::Local<v8::Object>,
    _kind: v8::AccessType,
    _data: v8::Local<v8::Value>,
) {
    deny(scope);
}

fn allowed_symbol(scope: &mut v8::PinScope, name: v8::Local<v8::Name>) -> bool {
    name.is_symbol()
        && [
            v8::Symbol::get_to_string_tag(scope),
            v8::Symbol::get_has_instance(scope),
            v8::Symbol::get_is_concat_spreadable(scope),
        ]
        .iter()
        .any(|symbol| symbol.strict_equals(name.into()))
}

const KEYS: &[&str] = &[
    "window",
    "self",
    "location",
    "closed",
    "frames",
    "length",
    "top",
    "opener",
    "parent",
    "blur",
    "close",
    "focus",
    "postMessage",
    "then",
];

pub(super) fn children<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    context: v8::Local<'s, v8::Context>,
) -> Vec<v8::Local<'s, v8::Context>> {
    context
        .get_slot::<HostHooks>()
        .and_then(|hooks| hooks.children)
        .map_or_else(Vec::new, |hook| hook(scope, context))
}

fn invoke_action<'s, 'i>(
    scope: &mut v8::PinScope<'s, 'i>,
    args: v8::FunctionCallbackArguments<'s>,
    _result: v8::ReturnValue<'s>,
) {
    let Ok(data) = v8::Local::<v8::Array>::try_from(args.data()) else {
        return;
    };
    let Some(window) = data
        .get_index(scope, 0)
        .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
    else {
        return;
    };
    let Some(action) = data.get_index(scope, 1) else {
        return;
    };
    let Some(target) = window.get_creation_context(scope) else {
        return;
    };
    let incumbent = v8::Global::new(scope, scope.get_incumbent_context());
    let source = v8::Local::new(scope, &incumbent);
    match action.to_rust_string_lossy(scope).as_str() {
        "close" => {
            super::auxiliary_contexts::request_close(scope, target);
        }
        "postMessage" => {
            let callback = target
                .get_slot::<HostHooks>()
                .and_then(|hooks| hooks.post_message);
            if !callback.is_some_and(|callback| {
                callback(scope, source, target, args.get(0), args.get(1), args.get(2))
            }) {
                unsupported(scope);
            }
        }
        action => {
            let callback = target.get_slot::<HostHooks>().and_then(|hooks| hooks.focus);
            if !callback.is_some_and(|callback| callback(scope, source, target, action == "focus"))
            {
                unsupported(scope);
            }
        }
    }
}

struct CachedFunction {
    target: v8::Weak<v8::Object>,
    function: v8::Weak<v8::Function>,
    name: String,
}
#[derive(Default)]
struct CallerFunctions(RefCell<Vec<CachedFunction>>);

fn cached_function<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    target: v8::Local<'s, v8::Object>,
    name: &str,
    callback: v8::FunctionCallback,
    data: v8::Local<'s, v8::Value>,
    length: i32,
) -> Option<v8::Local<'s, v8::Function>> {
    // Functions belong to the accessing realm. Sharing a cache between callers
    // would expose another origin's Function constructor. Weak rows preserve
    // observable identity without rooting closed target windows or caller realms.
    let caller = scope.get_current_context();
    let cache = caller.get_slot::<CallerFunctions>().unwrap_or_else(|| {
        let cache = Rc::new(CallerFunctions::default());
        caller.set_slot(cache.clone());
        cache
    });
    {
        let mut rows = cache.0.borrow_mut();
        rows.retain(|row| !row.target.is_empty() && !row.function.is_empty());
        for row in rows.iter().filter(|row| row.name == name) {
            if row
                .target
                .to_local(scope)
                .is_some_and(|value| value.strict_equals(target.into()))
            {
                if let Some(function) = row.function.to_local(scope) {
                    return Some(function);
                }
            }
        }
    }
    let function = v8::Function::builder_raw(super::external_refs::record_raw(callback))
        .data(data)
        .length(length)
        .constructor_behavior(v8::ConstructorBehavior::Throw)
        .build(scope)?;
    let display_name = v8::String::new(scope, name)?;
    function.set_name(display_name);
    cache.0.borrow_mut().push(CachedFunction {
        target: v8::Weak::new(scope, target),
        function: v8::Weak::new(scope, function),
        name: name.to_owned(),
    });
    Some(function)
}

fn cached_action<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    window: v8::Local<'s, v8::Object>,
    action: &str,
) -> Option<v8::Local<'s, v8::Function>> {
    let name = v8::String::new(scope, action)?;
    let data = v8::Array::new_with_elements(scope, &[window.into(), name.into()]);
    cached_function(
        scope,
        window,
        action,
        v8::MapFnTo::map_fn_to(invoke_action),
        data.into(),
        if action == "postMessage" { 1 } else { 0 },
    )
}

fn cross_value<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    window: v8::Local<'s, v8::Object>,
    property: &str,
) -> Option<v8::Local<'s, v8::Value>> {
    let context = window.get_creation_context(scope)?;
    Some(match property {
        "window" | "self" | "frames" | "parent" | "top" => window.into(),
        "closed" => v8::Boolean::new(
            scope,
            super::auxiliary_contexts::is_closed(context).unwrap_or(true),
        )
        .into(),
        "opener" => super::auxiliary_contexts::opener(scope, context)
            .map(Into::into)
            .unwrap_or_else(|| v8::null(scope).into()),
        "length" => {
            let length = children(scope, context).len();
            v8::Number::new(scope, length as f64).into()
        }
        "then" => v8::undefined(scope).into(),
        "location" => {
            let callback = context
                .get_slot::<HostHooks>()
                .and_then(|hooks| hooks.location);
            match callback.and_then(|callback| callback(scope, context)) {
                Some(value) => value,
                None => {
                    unsupported(scope);
                    return None;
                }
            }
        }
        "close" | "postMessage" | "focus" | "blur" => {
            cached_action(scope, window, property)?.into()
        }
        _ => {
            deny(scope);
            return None;
        }
    })
}

fn named_child<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    window: v8::Local<'s, v8::Object>,
    name: v8::Local<'s, v8::Name>,
) -> Option<v8::Local<'s, v8::Context>> {
    let name = v8::Local::<v8::String>::try_from(name).ok()?;
    if name.length() == 0 {
        return None;
    }
    let mut units = vec![0; name.length()];
    name.write_v2(scope, 0, &mut units, v8::WriteFlags::empty());
    let parent = window.get_creation_context(scope)?;
    children(scope, parent)
        .into_iter()
        .find(|&child| super::auxiliary_contexts::matches_child_name(scope, child, parent, &units))
}

fn may_name_child(property: &str) -> bool {
    // `then` is a cross-origin fallback, not a genuine own Window property.
    property == "then" || !KEYS.contains(&property)
}

fn child_descriptor<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    child: v8::Local<'s, v8::Context>,
    enumerable: bool,
) -> v8::Local<'s, v8::Object> {
    let descriptor = v8::Object::new(scope);
    let no = v8::Boolean::new(scope, false);
    let yes = v8::Boolean::new(scope, true);
    let enumerable = v8::Boolean::new(scope, enumerable);
    let window = child.global(scope);
    descriptor_field(scope, descriptor, "value", window.into());
    descriptor_field(scope, descriptor, "writable", no.into());
    descriptor_field(scope, descriptor, "enumerable", enumerable.into());
    descriptor_field(scope, descriptor, "configurable", yes.into());
    descriptor
}

fn cross_get<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    name: v8::Local<'s, v8::Name>,
    args: v8::PropertyCallbackArguments<'s>,
    mut result: v8::ReturnValue<v8::Value>,
) -> v8::Intercepted {
    if name.is_symbol() {
        if allowed_symbol(scope, name) {
            result.set(v8::undefined(scope).into());
        } else {
            deny(scope);
        }
    } else {
        let property = name.to_rust_string_lossy(scope);
        if may_name_child(&property) {
            if let Some(child) = named_child(scope, args.holder(), name) {
                result.set(child.global(scope).into());
                return v8::Intercepted::kYes;
            }
        }
        if let Some(value) = cross_value(scope, args.holder(), &property) {
            result.set(value);
        }
    }
    v8::Intercepted::kYes
}

fn cross_set<'s, 'i, T: 'static>(
    scope: &mut v8::PinScope<'s, 'i>,
    name: v8::Local<'s, v8::Name>,
    value: v8::Local<'s, v8::Value>,
    args: v8::PropertyCallbackArguments<'s>,
    mut result: v8::ReturnValue<T>,
) -> v8::Intercepted
where
    for<'cb> v8::ReturnValue<'cb, T>: NamedMutationResult,
{
    if name.is_symbol() || name.to_rust_string_lossy(scope) != "location" {
        deny(scope);
        return v8::Intercepted::kYes;
    }
    let incumbent = v8::Global::new(scope, scope.get_incumbent_context());
    let source = v8::Local::new(scope, &incumbent);
    let Some(target) = args.holder().get_creation_context(scope) else {
        return v8::Intercepted::kYes;
    };
    let callback = target
        .get_slot::<HostHooks>()
        .and_then(|hooks| hooks.navigate);
    let accepted = callback.is_some_and(|callback| callback(scope, source, target, value));
    result.set_success(accepted);
    if !accepted {
        unsupported(scope);
    }
    v8::Intercepted::kYes
}

fn cross_index<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    index: u32,
    args: v8::PropertyCallbackArguments<'s>,
    mut result: v8::ReturnValue<v8::Value>,
) -> v8::Intercepted {
    if let Some(context) = args.holder().get_creation_context(scope) {
        if let Some(child) = children(scope, context).get(index as usize) {
            result.set(child.global(scope).into());
            return v8::Intercepted::kYes;
        }
    }
    deny(scope);
    v8::Intercepted::kYes
}

fn cross_index_query<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    index: u32,
    args: v8::PropertyCallbackArguments<'s>,
    mut result: v8::ReturnValue<v8::Integer>,
) -> v8::Intercepted {
    if args
        .holder()
        .get_creation_context(scope)
        .is_some_and(|context| children(scope, context).get(index as usize).is_some())
    {
        result.set_int32(v8::PropertyAttribute::READ_ONLY.as_u32() as i32);
    } else {
        deny(scope);
    }
    v8::Intercepted::kYes
}

fn cross_index_descriptor<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    index: u32,
    args: v8::PropertyCallbackArguments<'s>,
    mut result: v8::ReturnValue,
) -> v8::Intercepted {
    let child = args
        .holder()
        .get_creation_context(scope)
        .and_then(|context| children(scope, context).get(index as usize).copied());
    if let Some(child) = child {
        let descriptor = child_descriptor(scope, child, true);
        result.set(descriptor.into());
    } else {
        deny(scope);
    }
    v8::Intercepted::kYes
}

fn cross_index_keys<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::PropertyCallbackArguments<'s>,
    mut result: v8::ReturnValue<v8::Array>,
) {
    let count = args
        .holder()
        .get_creation_context(scope)
        .map_or(0, |context| children(scope, context).len());
    let keys: Vec<_> = (0..count)
        .map(|index| v8::Integer::new_from_unsigned(scope, index as u32).into())
        .collect();
    result.set(v8::Array::new_with_elements(scope, &keys));
}

fn cross_query<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    name: v8::Local<'s, v8::Name>,
    _args: v8::PropertyCallbackArguments<'s>,
    mut result: v8::ReturnValue<v8::Integer>,
) -> v8::Intercepted {
    let property = (!name.is_symbol()).then(|| name.to_rust_string_lossy(scope));
    if property.as_deref() == Some("then") {
        // V8 rejects direct membership while skipping this non-enumerable
        // cross-origin fallback when filtering Object.keys().
        return v8::Intercepted::kNo;
    }
    if allowed_symbol(scope, name) || property.as_deref().is_some_and(|name| KEYS.contains(&name)) {
        result.set_int32(v8::PropertyAttribute::DONT_ENUM.as_u32() as i32);
    } else {
        deny(scope);
    }
    v8::Intercepted::kYes
}

fn cross_keys<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    _args: v8::PropertyCallbackArguments<'s>,
    mut result: v8::ReturnValue<v8::Array>,
) {
    let mut keys = Vec::with_capacity(KEYS.len() + 3);
    for key in KEYS {
        keys.push(v8::String::new(scope, key).unwrap().into());
    }
    for key in [
        v8::Symbol::get_to_string_tag(scope),
        v8::Symbol::get_has_instance(scope),
        v8::Symbol::get_is_concat_spreadable(scope),
    ] {
        keys.push(key.into());
    }
    result.set(v8::Array::new_with_elements(scope, &keys));
}

fn property_get<'s, 'i>(
    scope: &mut v8::PinScope<'s, 'i>,
    args: v8::FunctionCallbackArguments<'s>,
    mut result: v8::ReturnValue<'s>,
) {
    let Ok(data) = v8::Local::<v8::Array>::try_from(args.data()) else {
        return;
    };
    let Some(window) = data
        .get_index(scope, 0)
        .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
    else {
        return;
    };
    if !window.strict_equals(args.this().into()) {
        let message = v8::String::new(scope, "Illegal invocation").unwrap();
        let error = v8::Exception::type_error(scope, message);
        scope.throw_exception(error);
        return;
    }
    let Some(name) = data.get_index(scope, 1) else {
        return;
    };
    let property = name.to_rust_string_lossy(scope);
    if let Some(value) = cross_value(scope, window, &property) {
        result.set(value);
    }
}

fn location_set<'s, 'i>(
    scope: &mut v8::PinScope<'s, 'i>,
    args: v8::FunctionCallbackArguments<'s>,
    _result: v8::ReturnValue<'s>,
) {
    let Ok(window) = v8::Local::<v8::Object>::try_from(args.data()) else {
        return;
    };
    if !window.strict_equals(args.this().into()) {
        let message = v8::String::new(scope, "Illegal invocation").unwrap();
        let error = v8::Exception::type_error(scope, message);
        scope.throw_exception(error);
        return;
    }
    let Some(target) = window.get_creation_context(scope) else {
        return;
    };
    let incumbent = v8::Global::new(scope, scope.get_incumbent_context());
    let source = v8::Local::new(scope, &incumbent);
    let callback = target
        .get_slot::<HostHooks>()
        .and_then(|hooks| hooks.navigate);
    if !callback.is_some_and(|callback| callback(scope, source, target, args.get(0))) {
        unsupported(scope);
    }
}

fn descriptor_field<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    object: v8::Local<'s, v8::Object>,
    name: &str,
    value: v8::Local<'s, v8::Value>,
) {
    let key = v8::String::new(scope, name).unwrap();
    object.create_data_property(scope, key.into(), value);
}

fn cross_descriptor<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    name: v8::Local<'s, v8::Name>,
    args: v8::PropertyCallbackArguments<'s>,
    mut result: v8::ReturnValue,
) -> v8::Intercepted {
    let property = (!name.is_symbol()).then(|| name.to_rust_string_lossy(scope));
    if property.as_deref().is_some_and(may_name_child) {
        if let Some(child) = named_child(scope, args.holder(), name) {
            let descriptor = child_descriptor(scope, child, false);
            result.set(descriptor.into());
            return v8::Intercepted::kYes;
        }
    }
    if !allowed_symbol(scope, name) && !property.as_deref().is_some_and(|name| KEYS.contains(&name))
    {
        deny(scope);
        return v8::Intercepted::kYes;
    }
    let descriptor = v8::Object::new(scope);
    let no = v8::Boolean::new(scope, false);
    let yes = v8::Boolean::new(scope, true);
    descriptor_field(scope, descriptor, "enumerable", no.into());
    descriptor_field(scope, descriptor, "configurable", yes.into());
    if let Some(property) = property.as_deref().filter(|name| {
        matches!(
            *name,
            "window"
                | "self"
                | "location"
                | "closed"
                | "frames"
                | "length"
                | "top"
                | "opener"
                | "parent"
        )
    }) {
        let key = v8::String::new(scope, property).unwrap();
        let data = v8::Array::new_with_elements(scope, &[args.holder().into(), key.into()]);
        let Some(getter) = cached_function(
            scope,
            args.holder(),
            &format!("get {property}"),
            v8::MapFnTo::map_fn_to(property_get),
            data.into(),
            0,
        ) else {
            return v8::Intercepted::kYes;
        };
        descriptor_field(scope, descriptor, "get", getter.into());
        if property == "location" {
            let Some(setter) = cached_function(
                scope,
                args.holder(),
                "set location",
                v8::MapFnTo::map_fn_to(location_set),
                args.holder().into(),
                1,
            ) else {
                return v8::Intercepted::kYes;
            };
            descriptor_field(scope, descriptor, "set", setter.into());
        }
    } else {
        let value = property
            .as_deref()
            .and_then(|name| cross_value(scope, args.holder(), name))
            .unwrap_or_else(|| v8::undefined(scope).into());
        descriptor_field(scope, descriptor, "value", value);
        descriptor_field(scope, descriptor, "writable", no.into());
    }
    result.set(descriptor.into());
    v8::Intercepted::kYes
}

fn illegal_constructor<'s, 'i>(
    scope: &mut v8::PinScope<'s, 'i>,
    args: v8::FunctionCallbackArguments<'s>,
    _result: v8::ReturnValue<'s>,
) {
    let message = if args.new_target().is_undefined() {
        "Illegal constructor"
    } else {
        "Failed to construct 'Window': Illegal constructor"
    };
    let message = v8::String::new(scope, message).unwrap();
    let error = v8::Exception::type_error(scope, message);
    scope.throw_exception(error);
}

/// Global template for auxiliary top-level windows. Do not use this template for
/// child frames: their parent/top/index semantics need a host frame integration.
pub fn template<'s>(scope: &mut v8::PinScope<'s, '_, ()>) -> v8::Local<'s, v8::ObjectTemplate> {
    let constructor = v8::FunctionTemplate::new(scope, ext!(FunctionCallback, illegal_constructor));
    let name = v8::String::new(scope, "Window").unwrap();
    constructor.set_class_name(name);
    constructor.read_only_prototype();
    let template = constructor.instance_template(scope);
    template.set_with_attr(
        name.into(),
        constructor.into(),
        v8::PropertyAttribute::DONT_ENUM,
    );
    template.set_access_check_callback_and_handler(
        ext!(AccessCheckCallback, access_allowed),
        v8::NamedPropertyHandlerConfiguration::new()
            .getter(ext!(NamedPropertyGetterCallback, cross_get))
            .setter(ext!(NamedPropertySetterCallback, cross_set))
            .query(ext!(NamedPropertyQueryCallback, cross_query))
            .descriptor(ext!(NamedPropertyDescriptorCallback, cross_descriptor))
            .enumerator(ext!(NamedPropertyEnumeratorCallback, cross_keys)),
        v8::IndexedPropertyHandlerConfiguration::new()
            .getter(ext!(IndexedPropertyGetterCallback, cross_index))
            .query(ext!(IndexedPropertyQueryCallback, cross_index_query))
            .enumerator(ext!(IndexedPropertyEnumeratorCallback, cross_index_keys))
            .descriptor(ext!(
                IndexedPropertyDescriptorCallback,
                cross_index_descriptor
            )),
        None,
    );
    // Native-only layout: guest prototype changes cannot manufacture these fields.
    template.set_internal_field_count(2);
    template.set_immutable_proto();
    template
}

/// Whether this active global uses the producer's native Window template.
/// Hosts must reserve its two internal fields; ordinary Deno and legacy SDK
/// templates are unsupported until their construction delegates to `template`.
pub fn is_window_context(scope: &mut v8::PinScope, context: v8::Local<v8::Context>) -> bool {
    let global = context.global(scope);
    global.get_creation_context(scope) == Some(context) && global.internal_field_count() == 2
}

pub(super) fn adopt(scope: &mut v8::PinScope, context: v8::Local<v8::Context>) -> bool {
    if !is_window_context(scope, context) {
        return false;
    }
    context.set_slot(Rc::new(ManagedProxy));
    true
}

struct ManagedProxy;

/// Only contexts allocated by the genuine native template receive this marker.
pub fn is_managed(context: v8::Local<v8::Context>) -> bool {
    context.get_slot::<ManagedProxy>().is_some()
}

#[derive(Clone)]
struct PreviousOrigin {
    context: v8::Weak<v8::Context>,
    proxy: v8::Weak<v8::Object>,
    token: v8::Weak<v8::Value>,
}
#[derive(Default)]
struct PreviousOrigins(RefCell<Vec<PreviousOrigin>>);

fn remember_origin(scope: &mut v8::PinScope, context: v8::Local<v8::Context>) {
    let registry = scope
        .get_slot::<Rc<PreviousOrigins>>()
        .cloned()
        .unwrap_or_else(|| {
            let registry = Rc::new(PreviousOrigins::default());
            scope.set_slot(registry.clone());
            registry
        });
    let proxy = context.global(scope);
    let token = context.get_security_token(scope);
    // Keep the token through a private edge owned by the old lexical global,
    // never through an isolate-rooting native persistent context handle.
    let lexical = proxy
        .get_prototype(scope)
        .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok());
    let Some(lexical) = lexical else { return };
    let name = v8::String::new(scope, "turbo-dom.auxiliary.lexical-origin").unwrap();
    let key = v8::Private::for_api(scope, Some(name));
    if lexical.set_private(scope, key, token) != Some(true) {
        return;
    }
    let record = PreviousOrigin {
        context: v8::Weak::new(scope, context),
        proxy: v8::Weak::new(scope, proxy),
        token: v8::Weak::new(scope, token),
    };
    let mut records = registry.0.borrow_mut();
    records.retain(|record| !record.context.is_empty());
    records.push(record);
}

/// After assigning the committed document's origin token, restore access only
/// for retained lexical contexts whose original origin matches that token.
/// Every differently-originated old lexical context stays revoked.
pub fn synchronize_origins(scope: &mut v8::PinScope, active: v8::Local<v8::Context>) {
    let proxy = active.global(scope);
    let token = active.get_security_token(scope);
    let records = scope
        .get_slot::<Rc<PreviousOrigins>>()
        .map(|registry| registry.0.borrow().clone())
        .unwrap_or_default();
    for record in records {
        let (Some(context), Some(previous), Some(origin)) = (
            record.context.to_local(scope),
            record.proxy.to_local(scope),
            record.token.to_local(scope),
        ) else {
            continue;
        };
        if context == active || !proxy.strict_equals(previous.into()) {
            continue;
        }
        if token.strict_equals(origin) {
            context.set_security_token(origin);
        } else {
            context.use_default_security_token();
        }
    }
}

/// Allocate a genuine native WindowProxy, optionally reattaching a previously
/// detached proxy for an accepted navigation. The host must install its origin,
/// DOM, capabilities and document environment before exposing the new context.
/// Retained lexical contexts are revoked immediately; the host may restore an
/// old context's token only after verifying a same-origin committed navigation.
pub fn create_context<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    previous: Option<v8::Local<'s, v8::Context>>,
) -> v8::Local<'s, v8::Context> {
    let global = previous.map(|context| {
        let global = context.global(scope);
        remember_origin(scope, context);
        context.use_default_security_token();
        context.detach_global();
        global
    });
    let template = self::template(scope);
    let context = v8::Context::new(
        scope,
        v8::ContextOptions {
            global_template: Some(template),
            global_object: global.map(Into::into),
            ..Default::default()
        },
    );
    context.set_slot(Rc::new(ManagedProxy));
    context
}
