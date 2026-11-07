//! Smoke test for `PinScope::get_incumbent_context()`.
//!
//! The method is exposed for HTML's incumbent-realm concept (postMessage
//! `event.source`; turbo-surf#90). Here we only exercise the binding end to
//! end on real V8: with no JavaScript on the stack and a single entered
//! context, V8's `Isolate::GetIncumbentContext()` returns that entered
//! context. Driving nested cross-realm JS to exercise the incumbent
//! fallback paths is turbo-surf's `window_message_source.rs` oracle; this
//! crate's job is to prove the FFI round-trips a `Local<Context>`.

use std::sync::Once;

static PLATFORM: Once = Once::new();

fn initialize_v8() {
  PLATFORM.call_once(|| {
    v8::V8::initialize_platform(
      v8::new_unprotected_default_platform(0, false).make_shared(),
    );
    v8::V8::initialize();
  });
}

#[test]
fn get_incumbent_context_matches_entered_context_without_js_on_the_stack() {
  initialize_v8();
  let mut isolate = v8::Isolate::new(Default::default());
  let scope = std::pin::pin!(v8::HandleScope::new(&mut isolate));
  let scope = &mut scope.init();
  let context = v8::Context::new(scope, Default::default());
  let scope = &mut v8::ContextScope::new(scope, context);
  let current = scope.get_current_context();
  let incumbent = scope.get_incumbent_context();
  assert_eq!(current, incumbent);
}
