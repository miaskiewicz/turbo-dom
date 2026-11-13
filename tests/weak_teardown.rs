//! Native slot destructors must reset weak handles while their isolate is live.
use std::{cell::Cell, rc::Rc};
struct WeakSlot {
  handle: v8::Weak<v8::Function>,
  live_drops: Rc<Cell<usize>>,
}
impl Drop for WeakSlot {
  fn drop(&mut self) {
    if !self.handle.is_empty() {
      self.live_drops.set(self.live_drops.get() + 1);
    }
  }
}
#[test]
fn native_isolate_and_context_slots_reset_weak_handles_before_handle_invalidation()
 {
  v8::V8::initialize_platform(
    v8::new_unprotected_default_platform(0, false).make_shared(),
  );
  v8::V8::initialize();
  for _ in 0..16 {
    let drops = Rc::new(Cell::new(0));
    let mut isolate = v8::Isolate::new(Default::default());
    {
      let scope = std::pin::pin!(v8::HandleScope::new(&mut isolate));
      let scope = &mut scope.init();
      let context = v8::Context::new(scope, Default::default());
      let scope = &mut v8::ContextScope::new(scope, context);
      let source = v8::String::new(
        scope,
        "globalThis.retained = function retained() {}; retained",
      )
      .unwrap();
      let script = v8::Script::compile(scope, source, None).unwrap();
      let function =
        v8::Local::<v8::Function>::try_from(script.run(scope).unwrap())
          .unwrap();
      context.set_slot(Rc::new(WeakSlot {
        handle: v8::Weak::new(scope, function),
        live_drops: drops.clone(),
      }));
      let handle = v8::Weak::new(scope, function);
      scope.set_slot(WeakSlot {
        handle,
        live_drops: drops.clone(),
      });
    }
    drop(isolate);
    assert_eq!(
      drops.get(),
      2,
      "both destructors must see live weak handles before final V8 disposal"
    );
  }
}
