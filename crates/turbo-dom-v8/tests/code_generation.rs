//! Native callback controls over the official, unchanged V8 150 archive.
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Once;

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
  code: &str,
) -> v8::Local<'s, v8::Value> {
  let code = v8::String::new(scope, code).unwrap();
  v8::Script::compile(scope, code, None)
    .unwrap()
    .run(scope)
    .unwrap()
}

#[derive(Clone, Copy)]
enum Mode {
  Allow,
  Deny,
  Rewrite,
  Throw,
  Terminate,
}
struct Policy {
  mode: Cell<Mode>,
  calls: Cell<usize>,
}
impl Policy {
  fn new(mode: Mode) -> Rc<Self> {
    Rc::new(Self {
      mode: Cell::new(mode),
      calls: Cell::new(0),
    })
  }
}

fn authorize<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  context: v8::Local<'s, v8::Context>,
  _source: v8::Local<'s, v8::Value>,
  _is_code_like: bool,
) -> v8::ModifyCodeGenerationFromStringsResult<'s> {
  assert_eq!(context, scope.get_current_context());
  let policy = context.get_slot::<Policy>().unwrap();
  policy.calls.set(policy.calls.get() + 1);
  let mut result = v8::ModifyCodeGenerationFromStringsResult::default();
  match policy.mode.get() {
    Mode::Allow => result.codegen_allowed = true,
    Mode::Deny => {}
    Mode::Rewrite => {
      result.codegen_allowed = true;
      result.modified_source =
        Some(v8::String::new(scope, "'rewritten 😀'").unwrap());
    }
    Mode::Throw => {
      let message = v8::String::new(scope, "callback exception").unwrap();
      let exception = v8::Exception::type_error(scope, message);
      scope.throw_exception(exception);
    }
    Mode::Terminate => {
      scope.terminate_execution();
      result.codegen_allowed = true;
    }
  }
  result
}

#[test]
fn native_authorization_preserves_lexical_identity_and_context_dispatch() {
  init();
  let isolate = &mut v8::Isolate::new(Default::default());
  isolate.set_modify_code_generation_from_strings_callback(authorize);
  v8::scope!(let scope, isolate);
  let allowed = v8::Context::new(scope, Default::default());
  let allowed_policy = Policy::new(Mode::Allow);
  allowed.set_slot(allowed_policy.clone());
  allowed.set_allow_generation_from_strings(false);
  let denied = v8::Context::new(scope, Default::default());
  let denied_policy = Policy::new(Mode::Deny);
  denied.set_slot(denied_policy.clone());
  denied.set_allow_generation_from_strings(false);
  let fast = v8::Context::new(scope, Default::default());
  fast.set_slot(Policy::new(Mode::Deny));
  {
    let scope = &mut v8::ContextScope::new(scope, allowed);
    let identity = eval(
      scope,
      r#"(() => {
      const originalEval = eval, originalFunction = Function;
      let local = 17;
      const direct = eval('local + 1');
      const indirect = (0, eval)('typeof local');
      const f = Function('return 23');
      const async = Object.getPrototypeOf(async function(){}).constructor('return 24');
      const generator = Object.getPrototypeOf(function*(){}).constructor('yield 25');
      const asyncGenerator = Object.getPrototypeOf(async function*(){}).constructor('yield 26');
      const object = { toString() { throw Error('must not coerce'); } };
      return direct===18 && indirect==='undefined' && f()===23 &&
        Object.prototype.toString.call(f)==='[object Function]' &&
        Object.prototype.toString.call(async)==='[object AsyncFunction]' &&
        generator().next().value===25 &&
        Object.prototype.toString.call(asyncGenerator)==='[object AsyncGeneratorFunction]' &&
        eval(object)===object && eval(42)===42 && eval===originalEval && Function===originalFunction;
    })()"#,
    );
    assert!(identity.is_true());
  }
  assert!(allowed_policy.calls.get() >= 6);
  {
    let scope = &mut v8::ContextScope::new(scope, denied);
    assert!(eval(scope, r#"(() => {
      try { eval('1'); return false; } catch(e) { if (!(e instanceof EvalError)) return false; }
      try { Function('return 2'); return false; } catch(e) { return e instanceof EvalError; }
    })()"#).is_true());
  }
  assert_eq!(denied_policy.calls.get(), 2);
  {
    let scope = &mut v8::ContextScope::new(scope, fast);
    assert_eq!(eval(scope, "eval('29')").int32_value(scope), Some(29));
    assert_eq!(fast.get_slot::<Policy>().unwrap().calls.get(), 0);
    scope.clear_modify_code_generation_from_strings_callback();
  }
  {
    let scope = &mut v8::ContextScope::new(scope, allowed);
    assert!(eval(scope, "(() => {try {eval('1');return false} catch(e) {return e instanceof EvalError}})()").is_true());
  }
}

#[test]
fn native_replacement_is_live_and_utf16_preserving() {
  init();
  let isolate = &mut v8::Isolate::new(Default::default());
  isolate.set_modify_code_generation_from_strings_callback(authorize);
  v8::scope!(let scope, isolate);
  let context = v8::Context::new(scope, Default::default());
  context.set_slot(Policy::new(Mode::Rewrite));
  context.set_allow_generation_from_strings(false);
  let scope = &mut v8::ContextScope::new(scope, context);
  assert_eq!(
    eval(scope, "eval('original')").to_rust_string_lossy(scope),
    "rewritten 😀"
  );
}

#[test]
fn native_denial_uses_context_evalerror_message_and_observes_pending_throw() {
  init();
  let isolate = &mut v8::Isolate::new(Default::default());
  isolate.set_modify_code_generation_from_strings_callback(authorize);
  v8::scope!(let scope, isolate);
  let context = v8::Context::new(scope, Default::default());
  let policy = Policy::new(Mode::Deny);
  context.set_slot(policy.clone());
  context.set_allow_generation_from_strings(false);
  let scope = &mut v8::ContextScope::new(scope, context);
  let message = v8::String::new(scope, "CSP native denial 😀").unwrap();
  context.set_error_message_for_code_generation_from_strings(message);
  for code in ["eval('1')", "Function('return 1')"] {
    let code = format!(
      "(() => {{try {{{code}; return 'unexpected'}} catch(e) {{return e.name+': '+e.message}}}})()"
    );
    assert_eq!(
      eval(scope, &code).to_rust_string_lossy(scope),
      "EvalError: CSP native denial 😀"
    );
  }
  policy.mode.set(Mode::Throw);
  for code in ["eval('2')", "Function('return 2')"] {
    let code = format!(
      "(() => {{try {{{code}; return 'unexpected'}} catch(e) {{return e.name+': '+e.message}}}})()"
    );
    let observed = eval(scope, &code).to_rust_string_lossy(scope);
    assert_eq!(observed, "TypeError: callback exception");
  }
}

#[test]
fn native_callback_does_not_clear_termination() {
  init();
  let isolate = &mut v8::Isolate::new(Default::default());
  isolate.set_modify_code_generation_from_strings_callback(authorize);
  v8::scope!(let scope, isolate);
  let context = v8::Context::new(scope, Default::default());
  context.set_slot(Policy::new(Mode::Terminate));
  context.set_allow_generation_from_strings(false);
  let scope = &mut v8::ContextScope::new(scope, context);
  let code = v8::String::new(scope, "eval('1')").unwrap();
  let script = v8::Script::compile(scope, code, None).unwrap();
  let scope = std::pin::pin!(v8::TryCatch::new(scope));
  let scope = &mut scope.init();
  assert!(script.run(scope).is_none());
  assert!(scope.has_terminated());
  assert!(scope.is_execution_terminating());
  scope.cancel_terminate_execution();
}

#[test]
fn native_callback_registration_does_not_root_retired_context_slots() {
  init();
  let isolate = &mut v8::Isolate::new(Default::default());
  isolate.set_modify_code_generation_from_strings_callback(authorize);
  let retired = {
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let policy = Policy::new(Mode::Allow);
    let retired = Rc::downgrade(&policy);
    context.set_slot(policy);
    context.set_allow_generation_from_strings(false);
    let scope = &mut v8::ContextScope::new(scope, context);
    assert_eq!(eval(scope, "eval('31')").int32_value(scope), Some(31));
    retired
  };
  isolate.low_memory_notification();
  isolate.low_memory_notification();
  assert!(
    retired.upgrade().is_none(),
    "native callback must not root a retired context's policy"
  );
}
