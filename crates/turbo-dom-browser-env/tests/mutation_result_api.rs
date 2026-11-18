#![cfg(feature = "host-internals")]

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Once;
use turbo_dom_browser_env::host_internals::NamedMutationResult;

#[derive(Default)]
struct Calls(RefCell<Vec<(String, bool)>>);

fn setter<'s, 'i, T: 'static>(
    scope: &mut v8::PinScope<'s, 'i>,
    name: v8::Local<'s, v8::Name>,
    value: v8::Local<'s, v8::Value>,
    _: v8::PropertyCallbackArguments<'s>,
    mut result: v8::ReturnValue<T>,
) -> v8::Intercepted
where
    for<'cb> v8::ReturnValue<'cb, T>: NamedMutationResult,
{
    let name = name.to_rust_string_lossy(scope);
    let accepted = value.is_true();
    scope
        .get_slot::<Rc<Calls>>()
        .unwrap()
        .0
        .borrow_mut()
        .push((name, accepted));
    result.set_success(accepted);
    v8::Intercepted::kYes
}

fn deleter<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    name: v8::Local<'s, v8::Name>,
    _: v8::PropertyCallbackArguments<'s>,
    mut result: v8::ReturnValue<v8::Boolean>,
) -> v8::Intercepted {
    let name = name.to_rust_string_lossy(scope);
    let accepted = name == "acceptedDelete";
    scope
        .get_slot::<Rc<Calls>>()
        .unwrap()
        .0
        .borrow_mut()
        .push((name, accepted));
    result.set_success(accepted);
    v8::Intercepted::kYes
}

#[test]
fn curated_typed_mutation_bridge_reports_native_setter_admission() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        v8::V8::initialize_platform(v8::new_default_platform(0, false).make_shared());
        v8::V8::initialize();
    });
    let mut isolate = v8::Isolate::new(Default::default());
    let calls = Rc::new(Calls::default());
    isolate.set_slot(Rc::clone(&calls));
    {
        v8::scope!(let scope, &mut isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        let template = v8::ObjectTemplate::new(scope);
        // The mapping infers the actual callback result ABI; no version switch.
        template.set_named_property_handler(
            v8::NamedPropertyHandlerConfiguration::new()
                .setter(setter)
                .deleter(deleter),
        );
        let target = template.new_instance(scope).unwrap();
        let key = v8::String::new(scope, "target").unwrap();
        context.global(scope).set(scope, key.into(), target.into());
        let source = v8::String::new(scope, "Reflect.set(target,'accepted',true)===true&&Reflect.set(target,'rejected',false)===false&&Reflect.deleteProperty(target,'acceptedDelete')===true&&Reflect.deleteProperty(target,'rejectedDelete')===false").unwrap();
        let result = v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap();
        assert!(result.is_true());
        assert_eq!(
            *calls.0.borrow(),
            vec![
                ("accepted".to_owned(), true),
                ("rejected".to_owned(), false),
                ("acceptedDelete".to_owned(), true),
                ("rejectedDelete".to_owned(), false)
            ]
        );
    }
    isolate.remove_slot::<Rc<Calls>>();
}
