//! Typed mutation results selected by each native V8 callback signature.

/// Write success through the callback result type selected by the native V8 binding.
/// Both the default void result and native SDK Boolean result remain strongly typed.
pub trait NamedMutationResult {
    /// Report whether an intercepted mutation was accepted.
    fn set_success(&mut self, success: bool);
}
impl NamedMutationResult for v8::ReturnValue<'_, ()> {
    fn set_success(&mut self, success: bool) {
        self.set_bool(success);
    }
}
impl NamedMutationResult for v8::ReturnValue<'_, v8::Boolean> {
    fn set_success(&mut self, success: bool) {
        self.set_bool(success);
    }
}
