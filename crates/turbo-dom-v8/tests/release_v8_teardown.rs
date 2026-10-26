use std::cell::Cell;
use std::rc::Rc;
use std::sync::Once;

fn initialize() {
  static INIT: Once = Once::new();
  INIT.call_once(|| {
    v8::V8::initialize_platform(
      v8::new_default_platform(0, false).make_shared(),
    );
    v8::V8::initialize();
  });
}

struct WeakOwner {
  weak: Option<v8::Weak<v8::Object>>,
  handle: v8::IsolateHandle,
  retired: Rc<Cell<bool>>,
}

impl Drop for WeakOwner {
  fn drop(&mut self) {
    assert!(self.handle.cancel_terminate_execution());
    drop(self.weak.take());
    self.retired.set(true);
  }
}

fn check_retirement(snapshot: bool) {
  initialize();
  let retired = Rc::new(Cell::new(false));
  let finalized = Rc::new(Cell::new(false));
  let mut isolate = if snapshot {
    v8::Isolate::snapshot_creator(None, None)
  } else {
    v8::Isolate::new(Default::default())
  };
  let surviving = isolate.thread_safe_handle();
  {
    v8::scope!(let scope, &mut isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    if snapshot {
      scope.set_default_context(context);
    }
    let object = v8::Object::new(scope);
    let called = finalized.clone();
    let weak = v8::Weak::with_finalizer(
      scope,
      object,
      Box::new(move |_| called.set(true)),
    );
    scope.set_slot(WeakOwner {
      weak: Some(weak),
      handle: surviving.clone(),
      retired: retired.clone(),
    });
  }
  if snapshot {
    let blob = isolate
      .create_blob(v8::FunctionCodeHandling::Clear)
      .unwrap();
    assert!(!blob.is_empty());
  } else {
    drop(isolate);
  }
  assert!(retired.get());
  assert!(
    !finalized.get(),
    "retired Weak callback must stay unregistered"
  );
  assert!(!surviving.cancel_terminate_execution());
}

#[test]
fn normal_isolate_retires_owned_weak_handles_before_disposal() {
  check_retirement(false);
}

#[test]
fn snapshot_consumption_retires_owned_weak_handles_before_disposal() {
  check_retirement(true);
}
