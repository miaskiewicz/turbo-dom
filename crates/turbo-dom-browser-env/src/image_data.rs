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
