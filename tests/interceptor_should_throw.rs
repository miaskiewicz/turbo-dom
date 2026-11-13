//! Interceptors observe the caller's ShouldThrowOnError mode: true for strict
//! [[Set]]/[[Delete]] and every Object.defineProperty, false for sloppy writes,
//! the Reflect forms and non-mutating callbacks. Upstream Rusty V8 read the mode
//! through a `PropertyCallbackInfo<Value>` instantiation, which V8 folds to false;
//! turbo-dom-v8's function.rs fix reads the frame slot instead. On stock rusty_v8
//! the setter, definer and deleter tests fail.
use std::cell::RefCell;
use std::sync::Once;

thread_local! {
    static SEEN: RefCell<Vec<(String, bool)>> = const { RefCell::new(Vec::new()) };
}

fn initialize() {
  static INIT: Once = Once::new();
  INIT.call_once(|| {
    v8::V8::initialize_platform(
      v8::new_default_platform(0, false).make_shared(),
    );
    v8::V8::initialize();
  });
}

fn record(operation: &str, args: &v8::PropertyCallbackArguments) {
  let mode = args.should_throw_on_error();
  SEEN.with(|seen| seen.borrow_mut().push((operation.to_owned(), mode)));
}

fn named_getter<'s>(
  _: &mut v8::PinScope<'s, '_>,
  key: v8::Local<'s, v8::Name>,
  args: v8::PropertyCallbackArguments<'s>,
  _: v8::ReturnValue<v8::Value>,
) -> v8::Intercepted {
  if key.is_string() {
    record("named-get", &args);
  }
  v8::Intercepted::kNo
}
fn named_setter<'s>(
  _: &mut v8::PinScope<'s, '_>,
  _: v8::Local<'s, v8::Name>,
  _: v8::Local<'s, v8::Value>,
  args: v8::PropertyCallbackArguments<'s>,
  _: v8::ReturnValue<()>,
) -> v8::Intercepted {
  record("named-set", &args);
  v8::Intercepted::kNo
}
fn named_definer<'s>(
  _: &mut v8::PinScope<'s, '_>,
  _: v8::Local<'s, v8::Name>,
  _: &v8::PropertyDescriptor,
  args: v8::PropertyCallbackArguments<'s>,
  _: v8::ReturnValue<()>,
) -> v8::Intercepted {
  record("named-define", &args);
  v8::Intercepted::kNo
}
fn named_deleter<'s>(
  _: &mut v8::PinScope<'s, '_>,
  _: v8::Local<'s, v8::Name>,
  args: v8::PropertyCallbackArguments<'s>,
  _: v8::ReturnValue<v8::Boolean>,
) -> v8::Intercepted {
  record("named-delete", &args);
  v8::Intercepted::kNo
}
fn indexed_setter<'s>(
  _: &mut v8::PinScope<'s, '_>,
  _: u32,
  _: v8::Local<'s, v8::Value>,
  args: v8::PropertyCallbackArguments<'s>,
  _: v8::ReturnValue<()>,
) -> v8::Intercepted {
  record("indexed-set", &args);
  v8::Intercepted::kNo
}
fn indexed_definer<'s>(
  _: &mut v8::PinScope<'s, '_>,
  _: u32,
  _: &v8::PropertyDescriptor,
  args: v8::PropertyCallbackArguments<'s>,
  _: v8::ReturnValue<()>,
) -> v8::Intercepted {
  record("indexed-define", &args);
  v8::Intercepted::kNo
}
fn indexed_deleter<'s>(
  _: &mut v8::PinScope<'s, '_>,
  _: u32,
  args: v8::PropertyCallbackArguments<'s>,
  _: v8::ReturnValue<v8::Boolean>,
) -> v8::Intercepted {
  record("indexed-delete", &args);
  v8::Intercepted::kNo
}

fn observed(source: &str) -> Vec<(String, bool)> {
  initialize();
  SEEN.with(|seen| seen.borrow_mut().clear());
  let isolate = &mut v8::Isolate::new(Default::default());
  v8::scope!(let scope, isolate);
  let context = v8::Context::new(scope, Default::default());
  let scope = &mut v8::ContextScope::new(scope, context);
  let template = v8::ObjectTemplate::new(scope);
  template.set_named_property_handler(
    v8::NamedPropertyHandlerConfiguration::new()
      .getter(named_getter)
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

fn modes(operation: &str, values: &[bool]) -> Vec<(String, bool)> {
  values
    .iter()
    .map(|mode| (operation.to_owned(), *mode))
    .collect()
}

#[test]
fn setters_see_strict_assignment_but_not_sloppy_or_reflect_set() {
  let seen = observed(
        "target.a = 1; (function () { 'use strict'; target.b = 1; })(); Reflect.set(target, 'c', 1);
         target[0] = 1; (function () { 'use strict'; target[1] = 1; })(); Reflect.set(target, 2, 1);",
    );
  let setters: Vec<_> = seen
    .into_iter()
    .filter(|(operation, _)| operation.ends_with("-set"))
    .collect();
  let mut expected = modes("named-set", &[false, true, false]);
  expected.extend(modes("indexed-set", &[false, true, false]));
  assert_eq!(setters, expected);
}

#[test]
fn definers_throw_for_object_define_property_only() {
  let seen = observed(
    "Object.defineProperty(target, 'a', {value: 1, configurable: true});
         Reflect.defineProperty(target, 'b', {value: 1, configurable: true});
         Object.defineProperty(target, 0, {value: 1, configurable: true});
         Reflect.defineProperty(target, 1, {value: 1, configurable: true});",
  );
  let mut expected = modes("named-define", &[true, false]);
  expected.extend(modes("indexed-define", &[true, false]));
  assert_eq!(
    seen
      .into_iter()
      .filter(|(operation, _)| operation.ends_with("-define"))
      .collect::<Vec<_>>(),
    expected
  );
}

#[test]
fn deleters_see_strict_delete_but_not_sloppy_or_reflect_delete() {
  let seen = observed(
        "delete target.a; (function () { 'use strict'; delete target.b; })(); Reflect.deleteProperty(target, 'c');
         delete target[0]; (function () { 'use strict'; delete target[1]; })(); Reflect.deleteProperty(target, 2);",
    );
  let mut expected = modes("named-delete", &[false, true, false]);
  expected.extend(modes("indexed-delete", &[false, true, false]));
  assert_eq!(seen, expected);
}

#[test]
fn getters_never_report_should_throw() {
  let seen =
    observed("(function () { 'use strict'; return target.a; })(); target.b;");
  assert_eq!(seen, modes("named-get", &[false, false]));
}
