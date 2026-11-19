//! ImageData's internal record follows its object across genuine V8 realms. A private
//! symbol, rather than a realm-local WeakMap or a page-visible property, owns its brand.
use super::*;

struct Key(v8::Global<v8::Private>);

fn key<'s>(scope: &mut v8::PinScope<'s, '_>) -> v8::Local<'s, v8::Private> {
    if let Some(key) = scope.get_slot::<std::rc::Rc<Key>>().cloned() {
        return v8::Local::new(scope, &key.0);
    }
    let private = v8::Private::new(scope, None);
    let global = v8::Global::new(scope, private);
    scope.set_slot(std::rc::Rc::new(Key(global)));
    private
}

/// Engine-only get/set captured and removed from the global during installation.
pub(super) fn record(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let Ok(object) = v8::Local::<v8::Object>::try_from(args.get(0)) else {
        return;
    };
    let private = key(scope);
    if args.length() == 2 {
        object.set_private(scope, private, args.get(1));
    } else if let Some(value) = object.get_private(scope, private) {
        rv.set(value);
    }
}

/// Allocate the native view independently of when (or whether) the embedding exposes
/// the JavaScript Float16Array constructor. The captured JS ArrayBuffer allocator
/// performs the allocation and its normal range checks before entering this bridge.
pub(super) fn float16(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let Ok(buffer) = v8::Local::<v8::ArrayBuffer>::try_from(args.get(0)) else {
        return;
    };
    let length = buffer.byte_length() / 2;
    if let Some(view) = v8::Float16Array::new(scope, buffer, 0, length) {
        // SAFETY: V8 created a genuine typed-array object, which is a Value.
        // The binding's Float16Array type lacks the usual direct From upcast.
        rv.set(unsafe { v8::Local::<v8::Value>::cast_unchecked(view) });
    }
}

struct CanvasResetKey(v8::Global<v8::Private>);

fn canvas_reset_key<'s>(scope: &mut v8::PinScope<'s, '_>) -> v8::Local<'s, v8::Private> {
    if let Some(key) = scope.get_slot::<std::rc::Rc<CanvasResetKey>>().cloned() {
        return v8::Local::new(scope, &key.0);
    }
    let private = v8::Private::new(scope, None);
    scope.set_slot(std::rc::Rc::new(CanvasResetKey(v8::Global::new(
        scope, private,
    ))));
    private
}

/// Captured before guest execution; only genuine HTML canvas elements own reset hooks.
pub(super) fn canvas_reset(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let Ok(object) = v8::Local::<v8::Object>::try_from(args.get(0)) else {
        return;
    };
    let _dom = DomScope::for_object(scope, object);
    let genuine = handle_of(scope, object).is_some_and(|node| {
        with_tree(|tree| {
            tree.node_type_id(node) == 1
                && tree.namespace_id(node) == 0
                && tree.local_name(node) == Some("canvas")
        })
        .unwrap_or(false)
    });
    if !genuine {
        return;
    }
    if args.length() == 2 && args.get(1).is_function() {
        let private = canvas_reset_key(scope);
        object.set_private(scope, private, args.get(1));
    }
    rv.set_bool(true);
}

pub(super) fn canvas_attribute_changed<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    object: v8::Local<'s, v8::Object>,
    name: &str,
) {
    if !matches!(name, "width" | "height") {
        return;
    }
    let private = canvas_reset_key(scope);
    let Some(callback) = object
        .get_private(scope, private)
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
    else {
        return;
    };
    let receiver = v8::undefined(scope).into();
    callback.call(scope, receiver, &[]);
}

pub(super) fn is_proxy(
    _scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    rv.set_bool(args.get(0).is_proxy());
}

/// set/removeAttribute address the first qualified-name match, which can belong to
/// another namespace. Such a match is not the HTML canvas dimension attribute.
pub(super) fn qualified_dimension_mutation(node: Handle, name: &[u16]) -> bool {
    if name != [119, 105, 100, 116, 104] && name != [104, 101, 105, 103, 104, 116] {
        return true;
    }
    with_tree(|tree| {
        tree.attributes_units(node)
            .into_iter()
            .find(|attribute| attribute.qualified_name == name)
            .is_some_and(|attribute| attribute.namespace.is_none())
    })
    .unwrap_or(false)
}
