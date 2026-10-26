//! The caller's language mode as a setter/definer/deleter interceptor sees it.
//!
//! Blink's interceptors throw their own TypeError only when V8 says the
//! operation must throw (strict [[Set]]/[[Delete]], Object.defineProperty) and
//! otherwise report success or failure quietly (sloppy writes and the Reflect
//! forms). Published Rusty V8 (149.3/150.4) answers
//! `PropertyCallbackArguments::should_throw_on_error` through a
//! `PropertyCallbackInfo<Value>` instantiation, which V8 folds to `false`.
//!
//! Two sources, selected by the `native-sdk-frame-read` cargo feature:
//! - with it (today's default, required while the crate links the published
//!   `turbo-surf-v8`, and later by native SDKs without the fix) the binding
//!   reads the frame itself, exactly as V8's inline
//!   `PropertyCallbackInfo<void>::ShouldThrowOnError` does;
//! - without it the binding calls the accessor of the `turbo-dom-v8` fork, which
//!   carries the fix; a compile-time assertion on `v8::FORK_SHOULD_THROW_FIX`
//!   rejects a V8 without it instead of silently losing strict-mode TypeErrors.
//!
//! The frame read also compiles for tests and `test-support` (the
//! accessor-vs-frame-read agreement test and its embedder copies).

/// Only valid inside a setter, definer or deleter interceptor.
pub(super) fn should_throw(
    scope: &mut v8::PinScope<'_, '_>,
    args: &v8::PropertyCallbackArguments<'_>,
) -> bool {
    #[cfg(feature = "native-sdk-frame-read")]
    {
        frame_read_should_throw(scope, args)
    }
    #[cfg(not(feature = "native-sdk-frame-read"))]
    {
        args.should_throw_on_error()
    }
}

#[cfg(not(feature = "native-sdk-frame-read"))]
const _: () = assert!(v8::FORK_SHOULD_THROW_FIX);

#[cfg(any(test, feature = "native-sdk-frame-read", feature = "test-support"))]
extern "C" {
    #[cfg_attr(
        not(target_env = "msvc"),
        link_name = "_ZN2v88internal18ShouldThrowOnErrorEPNS0_7IsolateE"
    )]
    #[cfg_attr(
        target_env = "msvc",
        link_name = "?ShouldThrowOnError@internal@v8@@YA_NPEAVIsolate@12@@Z"
    )]
    fn v8_internal_should_throw_on_error(isolate: v8::UnsafeRawIsolatePtr) -> bool;
}

/// V8's `kDontThrow` (0) and `kInferShouldThrowMode` (2) Smis, with and
/// without pointer compression; any other value is `kThrowOnError`.
#[cfg(any(test, feature = "native-sdk-frame-read", feature = "test-support"))]
fn decode(slot: usize) -> Option<bool> {
    match slot {
        0 => Some(false),
        4 | 0x2_0000_0000 => None,
        _ => Some(true),
    }
}

/// Only valid inside a setter, definer or deleter interceptor, whose frame
/// stores the should-throw Smi right after the holder slot.
#[cfg(any(test, feature = "native-sdk-frame-read", feature = "test-support"))]
pub(crate) fn frame_read_should_throw(
    scope: &mut v8::PinScope<'_, '_>,
    args: &v8::PropertyCallbackArguments<'_>,
) -> bool {
    let holder = args.holder();
    // SAFETY: `holder` is the frame's kHolderIndex slot; kShouldThrowOnErrorIndex
    // is the next slot for every mutating callback (V8 14.9 and 15.0 frames).
    let slot = unsafe { *(&*holder as *const v8::Object as *const usize).add(1) };
    decode(slot).unwrap_or_else(|| {
        let isolate: &v8::Isolate = scope;
        // SAFETY: V8's exported helper inspects the current JS frame's language
        // mode; the raw pointer is the live isolate this callback runs on.
        unsafe { v8_internal_should_throw_on_error(isolate.as_raw_isolate_ptr()) }
    })
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    #[test]
    fn decodes_both_smi_encodings() {
        assert_eq!(super::decode(0), Some(false));
        assert_eq!(super::decode(2), Some(true));
        assert_eq!(super::decode(1 << 32), Some(true));
        assert_eq!(super::decode(4), None);
        assert_eq!(super::decode(2 << 32), None);
    }

    thread_local! {
        /// (operation, frame read, V8 accessor) per intercepted call.
        static SEEN: RefCell<Vec<(&'static str, bool, bool)>> = const { RefCell::new(Vec::new()) };
    }

    fn record(
        operation: &'static str,
        scope: &mut v8::PinScope<'_, '_>,
        args: &v8::PropertyCallbackArguments<'_>,
    ) {
        let frame = super::frame_read_should_throw(scope, args);
        let accessor = args.should_throw_on_error();
        SEEN.with(|seen| seen.borrow_mut().push((operation, frame, accessor)));
    }

    fn named_setter<'s>(
        scope: &mut v8::PinScope<'s, '_>,
        _: v8::Local<'s, v8::Name>,
        _: v8::Local<'s, v8::Value>,
        args: v8::PropertyCallbackArguments<'s>,
        _: v8::ReturnValue<()>,
    ) -> v8::Intercepted {
        record("named-set", scope, &args);
        v8::Intercepted::kNo
    }
    fn named_definer<'s>(
        scope: &mut v8::PinScope<'s, '_>,
        _: v8::Local<'s, v8::Name>,
        _: &v8::PropertyDescriptor,
        args: v8::PropertyCallbackArguments<'s>,
        _: v8::ReturnValue<()>,
    ) -> v8::Intercepted {
        record("named-define", scope, &args);
        v8::Intercepted::kNo
    }
    fn named_deleter<'s>(
        scope: &mut v8::PinScope<'s, '_>,
        _: v8::Local<'s, v8::Name>,
        args: v8::PropertyCallbackArguments<'s>,
        _: v8::ReturnValue<v8::Boolean>,
    ) -> v8::Intercepted {
        record("named-delete", scope, &args);
        v8::Intercepted::kNo
    }
    fn indexed_setter<'s>(
        scope: &mut v8::PinScope<'s, '_>,
        _: u32,
        _: v8::Local<'s, v8::Value>,
        args: v8::PropertyCallbackArguments<'s>,
        _: v8::ReturnValue<()>,
    ) -> v8::Intercepted {
        record("indexed-set", scope, &args);
        v8::Intercepted::kNo
    }
    fn indexed_definer<'s>(
        scope: &mut v8::PinScope<'s, '_>,
        _: u32,
        _: &v8::PropertyDescriptor,
        args: v8::PropertyCallbackArguments<'s>,
        _: v8::ReturnValue<()>,
    ) -> v8::Intercepted {
        record("indexed-define", scope, &args);
        v8::Intercepted::kNo
    }
    fn indexed_deleter<'s>(
        scope: &mut v8::PinScope<'s, '_>,
        _: u32,
        args: v8::PropertyCallbackArguments<'s>,
        _: v8::ReturnValue<v8::Boolean>,
    ) -> v8::Intercepted {
        record("indexed-delete", scope, &args);
        v8::Intercepted::kNo
    }

    /// Strict/sloppy/Reflect forms of every mutating interceptor, named and
    /// indexed, with the mode Chrome's interceptors see for each. A definer that
    /// does not intercept falls through to the setter, so each case only keeps
    /// the records of its own operation.
    const CASES: [(&str, &str, &[(&str, bool)]); 3] = [
        (
            "target.a = 1; (function () { 'use strict'; target.b = 1; })(); Reflect.set(target, 'c', 1);
             target[0] = 1; (function () { 'use strict'; target[1] = 1; })(); Reflect.set(target, 2, 1);",
            "-set",
            &[
                ("named-set", false),
                ("named-set", true),
                ("named-set", false),
                ("indexed-set", false),
                ("indexed-set", true),
                ("indexed-set", false),
            ],
        ),
        (
            "Object.defineProperty(target, 'a', {value: 1, configurable: true});
             Reflect.defineProperty(target, 'b', {value: 1, configurable: true});
             Object.defineProperty(target, 0, {value: 1, configurable: true});
             Reflect.defineProperty(target, 1, {value: 1, configurable: true});",
            "-define",
            &[
                ("named-define", true),
                ("named-define", false),
                ("indexed-define", true),
                ("indexed-define", false),
            ],
        ),
        (
            "delete target.a; (function () { 'use strict'; delete target.b; })(); Reflect.deleteProperty(target, 'c');
             delete target[0]; (function () { 'use strict'; delete target[1]; })(); Reflect.deleteProperty(target, 2);",
            "-delete",
            &[
                ("named-delete", false),
                ("named-delete", true),
                ("named-delete", false),
                ("indexed-delete", false),
                ("indexed-delete", true),
                ("indexed-delete", false),
            ],
        ),
    ];

    fn observed(source: &str) -> Vec<(&'static str, bool, bool)> {
        crate::browser_env::tests::init_v8();
        SEEN.with(|seen| seen.borrow_mut().clear());
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        let template = v8::ObjectTemplate::new(scope);
        template.set_named_property_handler(
            v8::NamedPropertyHandlerConfiguration::new()
                .setter(named_setter)
                .definer(named_definer)
                .deleter(named_deleter),
        );
        template.set_indexed_property_handler(
            v8::IndexedPropertyHandlerConfiguration::new()
                .setter(indexed_setter)
                .definer(indexed_definer)
                .deleter(indexed_deleter),
        );
        let object = template.new_instance(scope).unwrap();
        let key = v8::String::new(scope, "target").unwrap();
        context.global(scope).set(scope, key.into(), object.into());
        let code = v8::String::new(scope, source).unwrap();
        v8::Script::compile(scope, code, None)
            .unwrap()
            .run(scope)
            .unwrap();
        SEEN.with(|seen| seen.borrow().clone())
    }

    #[test]
    fn frame_read_matches_the_callers_language_mode() {
        for (source, suffix, expected) in CASES {
            let frame: Vec<_> = observed(source)
                .into_iter()
                .filter(|(operation, _, _)| operation.ends_with(suffix))
                .map(|(operation, frame, _)| (operation, frame))
                .collect();
            assert_eq!(frame, expected, "{suffix}");
        }
    }

    #[test]
    #[cfg_attr(
        feature = "native-sdk-frame-read",
        ignore = "the linked V8 lacks the fork's ShouldThrowOnError fix (native-sdk-frame-read)"
    )]
    fn fork_accessor_agrees_with_frame_read() {
        for (source, _, _) in CASES {
            for (operation, frame, accessor) in observed(source) {
                assert_eq!(accessor, frame, "{operation}");
            }
        }
    }
}
