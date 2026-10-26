//! The caller's language mode as a setter/definer/deleter interceptor sees it.
//!
//! Blink's interceptors throw their own TypeError only when V8 says the
//! operation must throw (strict [[Set]]/[[Delete]], Object.defineProperty) and
//! otherwise report success or failure quietly (sloppy writes and the Reflect
//! forms). Published Rusty V8 (149.3/150.4) answers
//! `PropertyCallbackArguments::should_throw_on_error` through a
//! `PropertyCallbackInfo<Value>` instantiation, which V8 folds to `false`.
//! turbo-test's vendored Rusty V8 carries the fix; turbo-surf links the
//! published 150.4 package, so the binding reads the frame itself, exactly as
//! V8's inline `PropertyCallbackInfo<void>::ShouldThrowOnError` does.

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
fn decode(slot: usize) -> Option<bool> {
    match slot {
        0 => Some(false),
        4 | 0x2_0000_0000 => None,
        _ => Some(true),
    }
}

/// Only valid inside a setter, definer or deleter interceptor, whose frame
/// stores the should-throw Smi right after the holder slot.
pub(super) fn should_throw(
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
    #[test]
    fn decodes_both_smi_encodings() {
        assert_eq!(super::decode(0), Some(false));
        assert_eq!(super::decode(2), Some(true));
        assert_eq!(super::decode(1 << 32), Some(true));
        assert_eq!(super::decode(4), None);
        assert_eq!(super::decode(2 << 32), None);
    }
}
