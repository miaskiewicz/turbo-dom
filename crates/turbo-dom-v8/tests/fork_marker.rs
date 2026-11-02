//! turbo-dom-v8 exports a compile-time marker so bindings can refuse a stock
//! rusty_v8 that lacks the interceptor ShouldThrowOnError fix.

const _: () = assert!(v8::FORK_SHOULD_THROW_FIX);

#[test]
fn fork_should_throw_fix_marker_is_set() {
  assert!(std::hint::black_box(v8::FORK_SHOULD_THROW_FIX));
}
