//! V8-private Web IDL records follow their owning objects across realms.
//! The isolate retains only a context-free key, never a page's map or functions.
use super::*;
use std::rc::Rc;

struct Records {
    key: v8::Global<v8::Private>,
}
fn initialize(scope: &mut v8::PinScope) {
    if scope.get_slot::<Rc<Records>>().is_some() || scope.is_execution_terminating() {
        return;
    }
    let key = v8::Private::new(scope, None);
    scope.set_slot(Rc::new(Records {
        key: v8::Global::new(scope, key),
    }));
}
pub fn get<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    object: v8::Local<'s, v8::Object>,
) -> Option<v8::Local<'s, v8::Value>> {
    if scope.is_execution_terminating() {
        return None;
    }
    let records = scope.get_slot::<Rc<Records>>().cloned()?;
    let key = v8::Local::new(scope, &records.key);
    object
        .get_private(scope, key)
        .filter(|value| !value.is_undefined())
}
pub fn set<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    object: v8::Local<'s, v8::Object>,
    record: v8::Local<'s, v8::Value>,
) -> bool {
    if scope.is_execution_terminating() {
        return false;
    }
    let Some(records) = scope.get_slot::<Rc<Records>>().cloned() else {
        return false;
    };
    let key = v8::Local::new(scope, &records.key);
    object.set_private(scope, key, record) == Some(true)
}
fn get_callback<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    if let Ok(object) = v8::Local::<v8::Object>::try_from(args.get(0)) {
        if let Some(value) = get(scope, object) {
            rv.set(value);
        }
    }
}
fn set_callback<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    _rv: v8::ReturnValue,
) {
    if let Ok(object) = v8::Local::<v8::Object>::try_from(args.get(0)) {
        set(scope, object, args.get(1));
    }
}
pub fn clear(isolate: &mut v8::Isolate) {
    isolate.remove_slot::<Rc<Records>>();
}
pub fn register(scope: &mut v8::PinScope) {
    let global = scope.get_current_context().global(scope);
    initialize(scope);
    bind_method(scope, global, "__getCollectionRecord", get_callback);
    bind_method(scope, global, "__setCollectionRecord", set_callback);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn private_records_ignore_guest_proxy_hooks_and_reset_without_retaining_a_realm() {
        super::super::tests::init_v8();
        let mut isolate = v8::Isolate::new(Default::default());
        let context = {
            v8::scope!(let scope,&mut isolate);
            let context = v8::Context::new(scope, Default::default());
            let scope = &mut v8::ContextScope::new(scope, context);
            initialize(scope);
            let code=v8::String::new(scope,"globalThis.hooks=0;[Object.freeze({}),new Proxy({},{get(){hooks++;throw Error('proxy hook');},defineProperty(){hooks++;throw Error('proxy hook');}})]").unwrap();
            let values = v8::Script::compile(scope, code, None)
                .unwrap()
                .run(scope)
                .unwrap()
                .to_object(scope)
                .unwrap();
            for index in 0..2 {
                let object = values
                    .get_index(scope, index)
                    .unwrap()
                    .to_object(scope)
                    .unwrap();
                let value = v8::Integer::new(scope, 17);
                assert!(set(scope, object, value.into()));
                assert_eq!(get(scope, object).unwrap().int32_value(scope), Some(17));
                let key = v8::String::new(scope, "hooks").unwrap();
                assert_eq!(
                    context
                        .global(scope)
                        .get(scope, key.into())
                        .unwrap()
                        .int32_value(scope),
                    Some(0)
                );
            }
            let object = values
                .get_index(scope, 0)
                .unwrap()
                .to_object(scope)
                .unwrap();
            clear(scope);
            assert!(get(scope, object).is_none());
            initialize(scope);
            assert!(get(scope, object).is_none());
            v8::Weak::new(scope, context)
        };
        for _ in 0..3 {
            isolate.low_memory_notification();
        }
        assert!(context.is_empty());
        clear(&mut isolate);
    }
    #[test]
    fn private_record_lookups_preserve_a_materialized_termination() {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope,isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        initialize(scope);
        let object = v8::Object::new(scope);
        let value = v8::Integer::new(scope, 7);
        assert!(set(scope, object, value.into()));
        v8::tc_scope!(let scope,scope);
        scope.terminate_execution();
        let code = v8::String::new(scope, "1").unwrap();
        let _ = v8::Script::compile(scope, code, None).and_then(|script| script.run(scope));
        assert!(get(scope, object).is_none());
        assert!(!set(scope, object, value.into()));
        assert!(scope.has_terminated() || scope.is_execution_terminating());
        scope.cancel_terminate_execution();
        scope.reset();
        assert_eq!(get(scope, object).unwrap().int32_value(scope), Some(7));
        clear(scope);
    }
}
