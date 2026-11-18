//! Native WindowProxy primitives against the unchanged official V8 SDK.
use std::sync::Once;
use std::sync::atomic::{AtomicUsize, Ordering};

fn init() {
  static INIT: Once = Once::new();
  INIT.call_once(|| {
    v8::V8::initialize_platform(
      v8::new_default_platform(0, false).make_shared(),
    );
    v8::V8::initialize();
  });
}

fn eval<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  source: &str,
) -> v8::Local<'s, v8::Value> {
  let source = v8::String::new(scope, source).unwrap();
  v8::Script::compile(scope, source, None)
    .unwrap()
    .run(scope)
    .unwrap()
}

fn access(
  _scope: &mut v8::PinScope,
  _accessing: v8::Local<v8::Context>,
  _object: v8::Local<v8::Object>,
  data: v8::Local<v8::Value>,
) -> bool {
  assert!(
    data.is_undefined(),
    "absent callback data must be a nonempty undefined handle"
  );
  false
}

fn cross_get<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  name: v8::Local<'s, v8::Name>,
  _args: v8::PropertyCallbackArguments<'s>,
  mut result: v8::ReturnValue<v8::Value>,
) -> v8::Intercepted {
  if name.to_rust_string_lossy(scope) == "allowed" {
    result.set_int32(17);
  } else {
    let message =
      v8::String::new(scope, "Blocked cross-origin window access").unwrap();
    let error = v8::Exception::error(scope, message);
    scope.throw_exception(error);
  }
  v8::Intercepted::kYes
}

fn cross_index<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  index: u32,
  args: v8::PropertyCallbackArguments<'s>,
  mut result: v8::ReturnValue<v8::Value>,
) -> v8::Intercepted {
  assert_eq!(
    args.data().to_rust_string_lossy(scope),
    "indexed handler data"
  );
  result.set_uint32(index + 7);
  v8::Intercepted::kYes
}

#[test]
fn access_check_invokes_only_the_explicit_cross_origin_handler() {
  init();
  let isolate = &mut v8::Isolate::new(v8::CreateParams::default());
  v8::scope!(let scope, isolate);
  let caller = v8::Context::new(scope, v8::ContextOptions::default());
  let scope = &mut v8::ContextScope::new(scope, caller);
  let template = v8::ObjectTemplate::new(scope);
  let indexed_data = v8::String::new(scope, "indexed handler data").unwrap();
  template.set_access_check_callback_and_handler(
    access,
    v8::NamedPropertyHandlerConfiguration::new().getter(cross_get),
    v8::IndexedPropertyHandlerConfiguration::new()
      .getter(cross_index)
      .data(indexed_data.into()),
    None,
  );
  let target = v8::Context::new(
    scope,
    v8::ContextOptions {
      global_template: Some(template),
      ..Default::default()
    },
  );
  {
    let target_scope = &mut v8::ContextScope::new(scope, target);
    assert_eq!(
      eval(target_scope, "globalThis.secret=42; secret")
        .int32_value(target_scope),
      Some(42)
    );
  }
  let global = caller.global(scope);
  let key = v8::String::new(scope, "other").unwrap();
  let other = target.global(scope);
  assert_eq!(global.set(scope, key.into(), other.into()), Some(true));
  assert_eq!(eval(scope, "other.allowed").int32_value(scope), Some(17));
  assert_eq!(eval(scope, "other[0]").int32_value(scope), Some(7));
  assert!(eval(scope, "(()=>{try{other.secret;return false}catch(e){return e.message==='Blocked cross-origin window access'}})()").is_true());
}

static FAILED_CHECKS: AtomicUsize = AtomicUsize::new(0);
static FAILED_KIND: AtomicUsize = AtomicUsize::new(usize::MAX);
static FAILED_DATA: AtomicUsize = AtomicUsize::new(0);
fn failed_access(
  scope: &mut v8::PinScope,
  _target: v8::Local<v8::Object>,
  kind: v8::AccessType,
  data: v8::Local<v8::Value>,
) {
  FAILED_KIND.store(kind as usize, Ordering::Relaxed);
  FAILED_DATA.store(usize::from(data.is_undefined()), Ordering::Relaxed);
  FAILED_CHECKS.fetch_add(1, Ordering::Relaxed);
  let text = v8::String::new(scope, "Native access denied").unwrap();
  let error = v8::Exception::error(scope, text);
  scope.throw_exception(error);
}

#[test]
fn failed_access_callback_can_raise_the_native_security_exception() {
  init();
  FAILED_CHECKS.store(0, Ordering::Relaxed);
  let isolate = &mut v8::Isolate::new(v8::CreateParams::default());
  isolate.set_failed_access_check_callback(failed_access);
  v8::scope!(let scope, isolate);
  let caller = v8::Context::new(scope, v8::ContextOptions::default());
  let scope = &mut v8::ContextScope::new(scope, caller);
  let template = v8::ObjectTemplate::new(scope);
  template.set_access_check_callback_and_handler(
    access,
    v8::NamedPropertyHandlerConfiguration::new(),
    v8::IndexedPropertyHandlerConfiguration::new(),
    None,
  );
  let target = v8::Context::new(
    scope,
    v8::ContextOptions {
      global_template: Some(template),
      ..Default::default()
    },
  );
  let global = caller.global(scope);
  let key = v8::String::new(scope, "other").unwrap();
  let other = target.global(scope);
  global.set(scope, key.into(), other.into());
  assert!(eval(scope, "(()=>{try{other.secret;return false}catch(e){return e.message==='Native access denied'}})()").is_true());
  assert_eq!(FAILED_CHECKS.load(Ordering::Relaxed), 1);
  assert_eq!(
    FAILED_KIND.load(Ordering::Relaxed),
    v8::AccessType::Has as usize
  );
  assert_eq!(FAILED_DATA.load(Ordering::Relaxed), 1);
}

#[test]
fn navigation_reuses_the_real_global_proxy_and_replaces_its_backing_context() {
  init();
  let isolate = &mut v8::Isolate::new(v8::CreateParams::default());
  v8::scope!(let scope, isolate);
  let first = v8::Context::new(scope, v8::ContextOptions::default());
  let proxy = first.global(scope);
  {
    let scope = &mut v8::ContextScope::new(scope, first);
    assert_eq!(
      eval(scope, "globalThis.oldOnly=42; oldOnly").int32_value(scope),
      Some(42)
    );
  }
  first.detach_global();
  let second = v8::Context::new(
    scope,
    v8::ContextOptions {
      global_object: Some(proxy.into()),
      ..Default::default()
    },
  );
  assert!(proxy.strict_equals(second.global(scope).into()));
  let scope = &mut v8::ContextScope::new(scope, second);
  assert_eq!(proxy.get_creation_context(scope), Some(second));
  assert!(eval(scope, "typeof oldOnly === 'undefined'").is_true());
  assert_eq!(
    eval(scope, "globalThis.newOnly=99; newOnly").int32_value(scope),
    Some(99)
  );
}
