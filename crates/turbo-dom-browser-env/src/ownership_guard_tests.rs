//! Budget interruption must survive private postmutation guards.
use super::*;

#[test]
fn subtree_ownership_maps_use_the_document_realm_after_foreign_calls() {
    tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    {
        v8::scope!(let scope, isolate);
        let main = v8::Context::new(scope, Default::default());
        let foreign = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, main);
        install_with_html(scope, "<body>sentinel");
        let (fragment, node) =
            with_tree_mut(|tree| (tree.create_document_fragment(), tree.create_element("div")))
                .unwrap();
        let fragment_object = wrap(scope, fragment);
        wrap(scope, node);
        let scope = &mut v8::ContextScope::new(scope, foreign);
        with_tree_mut(|tree| tree.append_child(fragment, node));
        dom_ownership::reconcile_current(scope);
        let name = v8::String::new(scope, "turbo-dom.ownership.component-owner").unwrap();
        let key = v8::Private::for_api(scope, Some(name));
        let component = fragment_object
            .get_private(scope, key)
            .unwrap()
            .to_object(scope)
            .unwrap();
        let name = v8::String::new(scope, "turbo-dom.ownership.component-nodes").unwrap();
        let key = v8::Private::for_api(scope, Some(name));
        let members = component
            .get_private(scope, key)
            .unwrap()
            .to_object(scope)
            .unwrap();
        assert_eq!(component.get_creation_context(scope), Some(main));
        assert_eq!(members.get_creation_context(scope), Some(main));
        reset();
    }
    clear_interface_records(isolate);
}

struct HelperPublication(std::cell::Cell<Option<bool>>);
fn interrupted_helper(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _: v8::ReturnValue,
) {
    let function = v8::Local::<v8::Function>::try_from(args.get(0)).unwrap();
    let source = v8::String::new(scope, "1").unwrap();
    let script = v8::Script::compile(scope, source, None).unwrap();
    scope.terminate_execution();
    let published = dom_ownership::helper_function(scope, function).is_some();
    scope
        .get_slot::<HelperPublication>()
        .unwrap()
        .0
        .set(Some(published));
    let _ = script.run(scope);
}

#[test]
fn interrupted_private_helper_is_not_published_and_can_be_retried() {
    tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, "<body>sentinel");
    scope.set_slot(HelperPublication(std::cell::Cell::new(None)));
    let global = context.global(scope);
    bind_method(scope, global, "interruptHelper", interrupted_helper);
    v8::tc_scope!(let catch, scope);
    let source = v8::String::new(catch, "interruptHelper(()=>true); 'resumed'").unwrap();
    let result = v8::Script::compile(catch, source, None).and_then(|script| script.run(catch));
    assert!(result.is_none());
    assert!(catch.has_terminated() || catch.is_execution_terminating());
    assert_eq!(
        catch.get_slot::<HelperPublication>().unwrap().0.get(),
        Some(false)
    );
    catch.cancel_terminate_execution();
    catch.reset();
    let source = v8::String::new(catch, "(()=>true)").unwrap();
    let function = v8::Script::compile(catch, source, None)
        .unwrap()
        .run(catch)
        .unwrap();
    let function = v8::Local::<v8::Function>::try_from(function).unwrap();
    assert!(dom_ownership::helper_function(catch, function).is_some());
    reset();
}

fn materialize_termination(scope: &mut v8::PinScope) {
    let source = v8::String::new(scope, "1").unwrap();
    let script = v8::Script::compile(scope, source, None).unwrap();
    scope.terminate_execution();
    let _ = script.run(scope);
}

fn interrupt_method(
    scope: &mut v8::PinScope,
    _: v8::FunctionCallbackArguments,
    _: v8::ReturnValue,
) {
    materialize_termination(scope);
}
fn getter(
    _: &mut v8::PinScope,
    _: v8::Local<v8::Name>,
    _: v8::PropertyCallbackArguments,
    _: v8::ReturnValue,
) {
}
fn interrupt_setter(
    scope: &mut v8::PinScope,
    _: v8::Local<v8::Name>,
    _: v8::Local<v8::Value>,
    _: v8::PropertyCallbackArguments,
    _: v8::ReturnValue<()>,
) {
    materialize_termination(scope);
}
#[test]
fn native_method_guard_preserves_materialized_termination() {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, "<body>sentinel");
    let template = method_template(scope, "interrupt", interrupt_method);
    let function = template.get_function(scope).unwrap();
    let key = v8::String::new(scope, "interrupt").unwrap();
    context
        .global(scope)
        .create_data_property(scope, key.into(), function.into());
    v8::tc_scope!(let catch, scope);
    let source = v8::String::new(catch, "interrupt(); 'resumed'").unwrap();
    let result = v8::Script::compile(catch, source, None).and_then(|script| script.run(catch));
    assert!(result.is_none());
    assert!(catch.has_terminated() || catch.is_execution_terminating());
    catch.cancel_terminate_execution();
    catch.reset();
    reset();
}
#[test]
fn native_setter_guard_preserves_materialized_termination() {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, "<body>sentinel");
    let template = v8::ObjectTemplate::new(scope);
    tmpl_accessor(scope, template, "value", getter, interrupt_setter);
    let object = template.new_instance(scope).unwrap();
    let key = v8::String::new(scope, "boxed").unwrap();
    context
        .global(scope)
        .create_data_property(scope, key.into(), object.into());
    v8::tc_scope!(let catch, scope);
    let source = v8::String::new(catch, "boxed.value=1; 'resumed'").unwrap();
    let result = v8::Script::compile(catch, source, None).and_then(|script| script.run(catch));
    assert!(result.is_none());
    assert!(catch.has_terminated() || catch.is_execution_terminating());
    catch.cancel_terminate_execution();
    catch.reset();
    reset();
}
