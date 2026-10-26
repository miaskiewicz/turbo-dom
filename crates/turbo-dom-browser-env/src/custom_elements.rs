//! Custom constructors return genuine cached native nodes from their definition.
use super::*;
use std::rc::Rc;

struct DefinitionsActive;
struct RetiredContext;

pub(super) fn retire_context(context: v8::Local<v8::Context>) {
    context.set_slot(Rc::new(RetiredContext));
}

fn object_active(scope: &mut v8::PinScope, object: v8::Local<v8::Object>) -> bool {
    object
        .get_creation_context(scope)
        .is_some_and(|context| context.get_slot::<RetiredContext>().is_none())
}

fn active(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut result: v8::ReturnValue,
) {
    let value = v8::Local::<v8::Object>::try_from(args.get(0))
        .is_ok_and(|value| object_active(scope, value));
    result.set_bool(value);
}

struct ConstructionFrame {
    constructor: v8::Weak<v8::Function>,
    node: v8::Weak<v8::Object>,
    used: bool,
}

#[derive(Default)]
struct ConstructionFrames(RefCell<Vec<ConstructionFrame>>);

struct ConstructionGuard(Rc<ConstructionFrames>);

impl Drop for ConstructionGuard {
    fn drop(&mut self) {
        self.0 .0.borrow_mut().pop();
    }
}

pub(super) fn recording() -> bool {
    DOM.with(|state| {
        state
            .borrow()
            .as_ref()
            .is_some_and(|state| state.custom_elements_active)
    })
}
fn activate() {
    DOM.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            state.custom_elements_active = true;
            state.tree.start_recording();
        }
    });
}
fn type_error(scope: &mut v8::PinScope, message: &str) {
    if let Some(message) = v8::String::new(scope, message) {
        let error = v8::Exception::type_error(scope, message);
        scope.throw_exception(error);
    }
}

fn upgrade(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut result: v8::ReturnValue,
) {
    let Ok(constructor) = v8::Local::<v8::Function>::try_from(args.get(0)) else {
        type_error(scope, "Invalid custom element constructor");
        return;
    };
    let Ok(node) = v8::Local::<v8::Object>::try_from(args.get(1)) else {
        type_error(scope, "Invalid custom element node");
        return;
    };
    let _owner = DomScope::for_object(scope, node);
    if scope.get_slot::<DefinitionsActive>().is_some() {
        activate();
    }
    if handle_of(scope, node).is_none() {
        type_error(scope, "Invalid custom element node");
        return;
    }
    let frames = scope
        .get_slot::<Rc<ConstructionFrames>>()
        .cloned()
        .unwrap_or_else(|| {
            let frames = Rc::new(ConstructionFrames::default());
            scope.set_slot(frames.clone());
            frames
        });
    frames.0.borrow_mut().push(ConstructionFrame {
        constructor: v8::Weak::new(scope, constructor),
        node: v8::Weak::new(scope, node),
        used: false,
    });
    // Local handles pin both values during guest construction. The native stack
    // holds only weak handles and is unwound even when V8 skips guest finally.
    let _guard = ConstructionGuard(frames);
    if let Some(value) = constructor.new_instance(scope, &[]) {
        if args.get(2).is_false() || value.strict_equals(node.into()) {
            result.set(value.into());
        } else {
            type_error(
                scope,
                "Custom element constructor returned a different object",
            );
        }
    }
}

fn construct(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut result: v8::ReturnValue,
) {
    if !args.is_construct_call()
        || scope
            .get_current_context()
            .get_slot::<RetiredContext>()
            .is_some()
    {
        type_error(scope, "Illegal constructor");
        return;
    }
    let Ok(delegate) = v8::Local::<v8::Function>::try_from(args.data()) else {
        return;
    };
    let prototype = args
        .this()
        .get_prototype(scope)
        .unwrap_or_else(|| v8::undefined(scope).into());
    let frames = scope.get_slot::<Rc<ConstructionFrames>>().cloned();
    let existing = frames.as_ref().and_then(|frames| {
        let mut frames = frames.0.borrow_mut();
        let frame = frames.last_mut()?;
        let constructor = frame.constructor.to_local(scope)?;
        if !constructor.strict_equals(args.new_target()) {
            return None;
        }
        if frame.used {
            type_error(scope, "Custom element is already constructed");
            return Some(None);
        }
        frame.used = true;
        Some(frame.node.to_local(scope))
    });
    if matches!(existing, Some(None)) {
        return;
    }
    let existing: v8::Local<v8::Value> = existing
        .flatten()
        .map_or_else(|| v8::undefined(scope).into(), Into::into);
    let receiver = v8::undefined(scope);
    if let Some(value) = delegate.call(
        scope,
        receiver.into(),
        &[args.new_target(), prototype, existing],
    ) {
        result.set(value);
    }
}
fn constructor(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut result: v8::ReturnValue,
) {
    let Ok(delegate) = v8::Local::<v8::Function>::try_from(args.get(0)) else {
        return;
    };
    let name = args.get(1).to_rust_string_lossy(scope);
    if let Some(function) = v8::Function::builder(construct)
        .data(delegate.into())
        .length(0)
        .build(scope)
    {
        if let Some(name) = v8::String::new(scope, &name) {
            function.set_name(name);
        }
        result.set(function.into());
    }
}
fn nodes(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut result: v8::ReturnValue,
) {
    let Ok(node) = v8::Local::<v8::Object>::try_from(args.get(0)) else {
        return;
    };
    let _dom = DomScope::for_object(scope, node);
    if scope.get_slot::<DefinitionsActive>().is_some() {
        activate();
    }
    let Some(handle) = handle_of(scope, node) else {
        return;
    };
    let handles = with_tree(|tree| {
        let mut children = Vec::new();
        if let Some(shadow) = tree.shadow_root(handle) {
            children.push(shadow);
        }
        children.extend(tree.children(handle));
        children
    })
    .unwrap_or_default();
    result.set(node_array(scope, handles).into());
}
fn is_key<'s>(scope: &mut v8::PinScope<'s, '_>) -> v8::Local<'s, v8::Private> {
    let name = v8::String::new(scope, "turbo.custom-elements.is").unwrap();
    v8::Private::for_api(scope, Some(name))
}
fn private(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut result: v8::ReturnValue,
) {
    let Ok(node) = v8::Local::<v8::Object>::try_from(args.get(0)) else {
        return;
    };
    let key = if args.get(2).is_string() {
        let family = args.get(2).to_rust_string_lossy(scope);
        let name = v8::String::new(scope, &format!("turbo.custom-elements.{family}")).unwrap();
        v8::Private::for_api(scope, Some(name))
    } else {
        is_key(scope)
    };
    if args.length() > 1 && !args.get(1).is_undefined() {
        node.set_private(scope, key, args.get(1));
    }
    if let Some(value) = node.get_private(scope, key) {
        result.set(value);
    }
}
pub(super) fn is_value(scope: &mut v8::PinScope, node: v8::Local<v8::Object>) -> Option<Vec<u16>> {
    let key = is_key(scope);
    let value = node.get_private(scope, key)?;
    if !value.is_string() {
        return None;
    }
    let value = value.to_string(scope)?;
    Some(string_code_units(scope, value))
}
fn report(scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, _result: v8::ReturnValue) {
    report_value(scope, args.get(0));
}
pub(super) fn report_value(scope: &mut v8::PinScope, error: v8::Local<v8::Value>) {
    let context = scope.get_current_context();
    let Some(builder) = context.get_slot::<ErrorHook>() else {
        return;
    };
    let Some(builder) = builder.0.to_local(scope) else {
        return;
    };
    let message = v8::Exception::create_message(scope, error);
    let text = message.get(scope);
    let filename = message
        .get_script_resource_name(scope)
        .unwrap_or_else(|| v8::undefined(scope).into());
    let line =
        v8::Integer::new_from_unsigned(scope, message.get_line_number(scope).unwrap_or(0) as u32);
    let column = v8::Integer::new_from_unsigned(scope, message.get_start_column() as u32 + 1);
    let receiver = v8::undefined(scope);
    let Some(event) = builder
        .call(
            scope,
            receiver.into(),
            &[error, text.into(), filename, line.into(), column.into()],
        )
        .and_then(|value| value.to_object(scope))
    else {
        return;
    };
    let target = context.global(scope);
    dispatch_host_event(scope, event, target);
}
struct ErrorHook(v8::Weak<v8::Function>);
#[derive(Default)]
struct DefinitionFrames(RefCell<Vec<v8::Weak<v8::Object>>>);
struct DefinitionGuard(Rc<DefinitionFrames>);
impl Drop for DefinitionGuard {
    fn drop(&mut self) {
        self.0 .0.borrow_mut().pop();
    }
}
fn defining(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut result: v8::ReturnValue,
) {
    let Ok(registry) = v8::Local::<v8::Object>::try_from(args.get(0)) else {
        return;
    };
    let frames = scope
        .get_slot::<Rc<DefinitionFrames>>()
        .cloned()
        .unwrap_or_else(|| {
            let frames = Rc::new(DefinitionFrames::default());
            scope.set_slot(frames.clone());
            frames
        });
    if args.length() == 1 {
        result.set_bool(frames.0.borrow().iter().any(|weak| {
            weak.to_local(scope)
                .is_some_and(|value| value.strict_equals(registry.into()))
        }));
        return;
    }
    scope.set_slot(DefinitionsActive);
    activate();
    let Ok(callback) = v8::Local::<v8::Function>::try_from(args.get(1)) else {
        return;
    };
    frames.0.borrow_mut().push(v8::Weak::new(scope, registry));
    let _guard = DefinitionGuard(frames);
    let receiver = v8::undefined(scope);
    if let Some(value) = callback.call(scope, receiver.into(), &[]) {
        result.set(value);
    }
}
struct MutationHook(v8::Weak<v8::Function>);
struct CloneHook(v8::Weak<v8::Function>);
struct ParsedHook(v8::Weak<v8::Function>);

fn mutation_hook<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    node: v8::Local<'_, v8::Object>,
) -> Option<v8::Local<'s, v8::Function>> {
    node.get_creation_context(scope)?
        .get_slot::<MutationHook>()?
        .0
        .to_local(scope)
}
#[derive(Default)]
struct ReactionFrames(RefCell<Vec<Rc<ReactionQueue>>>);
struct ReactionQueue {
    events: v8::Weak<v8::Array>,
    order: RefCell<Option<v8::Weak<v8::Array>>>,
    callbacks: RefCell<Option<v8::Weak<v8::Map>>>,
    collecting: std::cell::Cell<bool>,
    count: std::cell::Cell<usize>,
}
pub(super) struct ReactionScope<'s> {
    frames: Rc<ReactionFrames>,
    entry: Rc<ReactionQueue>,
    queue: v8::Local<'s, v8::Array>,
}
impl Drop for ReactionScope<'_> {
    fn drop(&mut self) {
        self.entry.collecting.set(false);
        self.frames.0.borrow_mut().pop();
    }
}
fn append(scope: &mut v8::PinScope, array: v8::Local<v8::Array>, value: v8::Local<v8::Value>) {
    let key = v8::String::new(scope, &array.length().to_string()).unwrap();
    array.create_data_property(scope, key.into(), value);
}
pub(super) fn reactions<'s>(scope: &mut v8::PinScope<'s, '_>) -> ReactionScope<'s> {
    let frames = scope
        .get_slot::<Rc<ReactionFrames>>()
        .cloned()
        .unwrap_or_else(|| {
            let frames = Rc::new(ReactionFrames::default());
            scope.set_slot(frames.clone());
            frames
        });
    let queue = v8::Array::new(scope, 0);
    let entry = Rc::new(ReactionQueue {
        events: v8::Weak::new(scope, queue),
        order: RefCell::new(None),
        callbacks: RefCell::new(None),
        collecting: std::cell::Cell::new(false),
        count: std::cell::Cell::new(0),
    });
    frames.0.borrow_mut().push(entry.clone());
    ReactionScope {
        frames,
        entry,
        queue,
    }
}
fn react(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut result: v8::ReturnValue,
) {
    result.set_bool(false);
    let entry = scope
        .get_slot::<Rc<ReactionFrames>>()
        .and_then(|frames| frames.0.borrow().last().cloned());
    let Some(entry) = entry.filter(|entry| entry.collecting.get()) else {
        return;
    };
    let (Some(order), Some(callbacks)) = (
        entry
            .order
            .borrow()
            .as_ref()
            .and_then(|value| value.to_local(scope)),
        entry
            .callbacks
            .borrow()
            .as_ref()
            .and_then(|value| value.to_local(scope)),
    ) else {
        return;
    };
    if entry.count.get() >= 4096 {
        scope.terminate_execution();
        return;
    }
    entry.count.set(entry.count.get() + 1);
    let node = args.get(0);
    let existing = callbacks
        .get(scope, node)
        .and_then(|value| v8::Local::<v8::Array>::try_from(value).ok());
    let queue = existing.unwrap_or_else(|| {
        // Reactions belong to the element, not the current CEReactions frame.
        // A callback can enter another frame while its element still has older
        // reactions queued; that frame must drain the older reactions first.
        let frames = scope
            .get_slot::<Rc<ReactionFrames>>()
            .map(|frames| frames.0.borrow().clone())
            .unwrap_or_default();
        let pending = frames.iter().rev().skip(1).find_map(|frame| {
            let callbacks = frame.callbacks.borrow().as_ref()?.to_local(scope)?;
            callbacks
                .get(scope, node)
                .and_then(|value| v8::Local::<v8::Array>::try_from(value).ok())
        });
        let queue = pending.unwrap_or_else(|| {
            let cursor = v8::Integer::new_from_unsigned(scope, 1);
            v8::Array::new_with_elements(scope, &[cursor.into()])
        });
        callbacks.set(scope, node, queue.into());
        append(scope, order, node);
        queue
    });
    let record =
        v8::Array::new_with_elements(scope, &[args.get(1), args.get(2), args.get(3), args.get(4)]);
    append(scope, queue, record.into());
    result.set_bool(true);
}
impl ReactionScope<'_> {
    pub(super) fn flush(&self, scope: &mut v8::PinScope) {
        if self.queue.length() == 0 {
            return;
        }
        let order = v8::Array::new(scope, 0);
        let callbacks = v8::Map::new(scope);
        *self.entry.order.borrow_mut() = Some(v8::Weak::new(scope, order));
        *self.entry.callbacks.borrow_mut() = Some(v8::Weak::new(scope, callbacks));
        self.entry.collecting.set(true);
        let mut index = 0;
        while index < self.queue.length() && !scope.is_execution_terminating() {
            if let Some(event) = self
                .queue
                .get_index(scope, index)
                .and_then(|value| value.to_object(scope))
            {
                deliver_mutation(scope, event);
            }
            index += 1;
        }
        self.entry.collecting.set(false);
        for index in 0..order.length() {
            if scope.is_execution_terminating() {
                return;
            }
            let Some(node) = order.get_index(scope, index) else {
                continue;
            };
            let Some(queue) = callbacks
                .get(scope, node)
                .and_then(|value| v8::Local::<v8::Array>::try_from(value).ok())
            else {
                continue;
            };
            loop {
                if scope.is_execution_terminating() {
                    return;
                }
                let Some(index) = queue
                    .get_index(scope, 0)
                    .and_then(|value| value.uint32_value(scope))
                    .filter(|index| *index < queue.length())
                else {
                    break;
                };
                // Advance before guest code: reentrant delivery shares the cursor.
                let next = v8::Integer::new_from_unsigned(scope, index + 1);
                let key = v8::String::new(scope, "0").unwrap();
                queue.create_data_property(scope, key.into(), next.into());
                let Some(record) = queue
                    .get_index(scope, index)
                    .and_then(|value| v8::Local::<v8::Array>::try_from(value).ok())
                else {
                    continue;
                };
                let Some(function) = record
                    .get_index(scope, 0)
                    .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
                else {
                    continue;
                };
                let inactive_definition = [2, 3].into_iter().any(|index| {
                    record
                        .get_index(scope, index)
                        .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
                        .is_some_and(|value| !object_active(scope, value))
                });
                if inactive_definition || !object_active(scope, function.into()) {
                    continue;
                }
                let Some(arguments) = record
                    .get_index(scope, 1)
                    .and_then(|value| v8::Local::<v8::Array>::try_from(value).ok())
                else {
                    continue;
                };
                let values = (0..arguments.length())
                    .filter_map(|index| arguments.get_index(scope, index))
                    .collect::<Vec<_>>();
                let Some(context) = function.get_creation_context(scope) else {
                    continue;
                };
                let scope = &mut v8::ContextScope::new(scope, context);
                let caught = std::pin::pin!(v8::TryCatch::new(scope));
                let scope = &mut caught.init();
                if function.call(scope, node, &values).is_none() {
                    if scope.has_terminated() {
                        scope.rethrow();
                        return;
                    }
                    if let Some(error) = scope.exception() {
                        scope.reset();
                        report_value(scope, error);
                    }
                }
            }
        }
    }
}
pub(super) fn mutation(scope: &mut v8::PinScope, event: v8::Local<v8::Object>) {
    if scope.get_slot::<DefinitionsActive>().is_none() {
        return;
    }
    let queue = scope.get_slot::<Rc<ReactionFrames>>().and_then(|frames| {
        frames
            .0
            .borrow()
            .last()
            .and_then(|entry| entry.events.to_local(scope))
    });
    if let Some(queue) = queue {
        if queue.length() >= 4096 {
            scope.terminate_execution();
            return;
        }
        append(scope, queue, event.into());
    } else {
        deliver_mutation(scope, event);
    }
}
fn deliver_mutation(scope: &mut v8::PinScope, event: v8::Local<v8::Object>) {
    if scope.is_execution_terminating() {
        return;
    }
    let key = v8::String::new(scope, "target").unwrap();
    let Some(node) = event
        .get(scope, key.into())
        .and_then(|value| value.to_object(scope))
    else {
        return;
    };
    let Some(function) = mutation_hook(scope, node) else {
        return;
    };
    let receiver = v8::undefined(scope);
    function.call(scope, receiver.into(), &[event.into()]);
}
pub(super) fn parsed_fragment(
    scope: &mut v8::PinScope,
    fragment: v8::Local<v8::Object>,
    context: v8::Local<v8::Object>,
) {
    association_hook(scope, fragment, false, Some(context));
    let _owner = DomScope::for_object(scope, fragment);
    let Some(handle) = handle_of(scope, fragment) else {
        return;
    };
    let handles = with_tree(|tree| tree.children(handle)).unwrap_or_default();
    if handles.is_empty() {
        return;
    }
    let event = v8::Object::new(scope);
    let kind = v8::String::new(scope, "childList").unwrap();
    set_prop(scope, event, "type", kind.into());
    set_prop(scope, event, "target", fragment.into());
    let added = node_array(scope, handles);
    set_prop(scope, event, "addedNodes", added.into());
    let removed = v8::Array::new(scope, 0);
    set_prop(scope, event, "removedNodes", removed.into());
    let reactions = reactions(scope);
    mutation(scope, event);
    reactions.flush(scope);
}
pub(super) fn parsed(scope: &mut v8::PinScope, node: v8::Local<v8::Object>) {
    association_hook(scope, node, false, None);
}
pub(super) fn adopting(scope: &mut v8::PinScope, node: v8::Local<v8::Object>) {
    association_hook(scope, node, true, None);
}
fn association_hook(
    scope: &mut v8::PinScope,
    node: v8::Local<v8::Object>,
    adopting: bool,
    parser_context: Option<v8::Local<v8::Object>>,
) {
    let Some(function) = node
        .get_creation_context(scope)
        .and_then(|context| context.get_slot::<ParsedHook>())
        .and_then(|hook| hook.0.to_local(scope))
    else {
        return;
    };
    let Some(context) = function.get_creation_context(scope) else {
        return;
    };
    let scope = &mut v8::ContextScope::new(scope, context);
    let _owner = DomScope::for_object(scope, node);
    let undefined = v8::undefined(scope);
    let adopting = v8::Boolean::new(scope, adopting);
    let context = parser_context.map_or_else(|| v8::undefined(scope).into(), Into::into);
    function.call(
        scope,
        undefined.into(),
        &[node.into(), adopting.into(), context],
    );
}
pub(super) fn moved(scope: &mut v8::PinScope, node: v8::Local<v8::Object>) {
    let event = v8::Object::new(scope);
    let null = v8::null(scope);
    event.set_prototype(scope, null.into());
    let kind = v8::String::new(scope, "move").unwrap();
    set_prop(scope, event, "type", kind.into());
    set_prop(scope, event, "target", node.into());
    mutation(scope, event);
}
pub(super) fn adopted(scope: &mut v8::PinScope, node: v8::Local<v8::Object>) {
    if scope.get_slot::<DefinitionsActive>().is_some() {
        activate();
    }
    if scope.is_execution_terminating() {
        return;
    }
    let event = v8::Object::new(scope);
    let null = v8::null(scope);
    event.set_prototype(scope, null.into());
    let value = v8::String::new(scope, "adoption").unwrap();
    set_prop(scope, event, "type", value.into());
    set_prop(scope, event, "target", node.into());
    mutation(scope, event);
}
pub(super) fn cloned(
    scope: &mut v8::PinScope,
    source: v8::Local<v8::Object>,
    clone: v8::Local<v8::Object>,
    deep: bool,
) {
    let Some(context) = source.get_creation_context(scope) else {
        return;
    };
    let Some(hook) = context.get_slot::<CloneHook>() else {
        return;
    };
    let Some(function) = hook.0.to_local(scope) else {
        return;
    };
    let receiver = v8::undefined(scope);
    let deep = v8::Boolean::new(scope, deep);
    function.call(
        scope,
        receiver.into(),
        &[source.into(), clone.into(), deep.into()],
    );
}
fn bind_native<F>(scope: &mut v8::PinScope, global: v8::Local<v8::Object>, name: &str, callback: F)
where
    F: v8::MapFnTo<v8::FunctionCallback>,
{
    if let Some(function) = v8::FunctionTemplate::builder(callback)
        .constructor_behavior(v8::ConstructorBehavior::Throw)
        .build(scope)
        .get_function(scope)
    {
        set_prop(scope, global, name, function.into());
    }
}
pub(super) fn install(scope: &mut v8::PinScope) {
    let global = scope.get_current_context().global(scope);
    bind_native(scope, global, "__customElementConstructor", constructor);
    bind_native(scope, global, "__customElementChildren", nodes);
    bind_native(scope, global, "__customElementUpgrade", upgrade);
    bind_native(scope, global, "__customElementPrivate", private);
    bind_native(scope, global, "__customElementReport", report);
    bind_native(scope, global, "__customElementDefining", defining);
    bind_native(scope, global, "__customElementReact", react);
    bind_native(scope, global, "__customElementActive", active);
    run_js(scope, include_str!("custom_elements.js"));
    for (name, family) in [
        ("__captureCustomElementMutation", 0),
        ("__captureCustomElementClone", 1),
        ("__captureCustomElementError", 2),
        ("__captureCustomElementParsed", 3),
    ] {
        let key = v8::String::new(scope, name).unwrap();
        if let Some(function) = global
            .get(scope, key.into())
            .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
        {
            let Some(function) = dom_ownership::helper_function(scope, function) else {
                return;
            };
            let context = scope.get_current_context();
            match family {
                0 => {
                    context.set_slot(Rc::new(MutationHook(function)));
                }
                1 => {
                    context.set_slot(Rc::new(CloneHook(function)));
                }
                2 => {
                    context.set_slot(Rc::new(ErrorHook(function)));
                }
                _ => {
                    context.set_slot(Rc::new(ParsedHook(function)));
                }
            }
        }
        global.delete(scope, key.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup<'s>(scope: &mut v8::PinScope<'s, '_>) {
        install_with_html(scope, "<!doctype html><body></body>");
        let global = scope.get_current_context().global(scope);
        bind_method(scope, global, "nativeTestConstructor", constructor);
        bind_method(scope, global, "nativeTestUpgrade", upgrade);
    }

    fn check_chrome_probe(probe: &str, oracle: &str) {
        check_chrome_probe_with_consumers(probe, oracle, false);
    }

    fn check_chrome_probe_with_consumers(probe: &str, oracle: &str, consumers: bool) {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(scope, "<!doctype html><body></body>");
        let mut contexts = vec![context];
        if probe.contains("child.eval(") || probe.contains("child.document") {
            let child = v8::Context::new(scope, Default::default());
            child.set_security_token(context.get_security_token(scope));
            {
                let scope = &mut v8::ContextScope::new(scope, child);
                install_independent_html(scope, "<!doctype html><body></body>");
            }
            contexts.push(child);
            let global = context.global(scope);
            let child = child.global(scope);
            set_prop(scope, global, "child", child.into());
        }
        if probe.contains("quirksDocument") {
            let quirks = parse_html_document(scope, "<body><div id=quirks></div>", false);
            let global = context.global(scope);
            set_prop(scope, global, "quirksDocument", quirks.into());
        }
        if consumers {
            for context in contexts {
                let scope = &mut v8::ContextScope::new(scope, context);
                let key = v8::String::new(scope, "document").unwrap();
                let document = context
                    .global(scope)
                    .get(scope, key.into())
                    .unwrap()
                    .to_object(scope)
                    .unwrap();
                let _owner = DomScope::for_object(scope, document);
                // Production installs a resource consumer in every frame. Its
                // captured prototype query reenters native mutation fanout.
                let source = v8::String::new(
                    scope,
                    r#"(() => {
                    const query = Document.prototype.querySelectorAll;
                    return () => Reflect.apply(query, document, ['base[href]']).length;
                })()"#,
                )
                .unwrap();
                let callback = v8::Script::compile(scope, source, None)
                    .unwrap()
                    .run(scope)
                    .unwrap();
                let callback = v8::Local::<v8::Function>::try_from(callback).unwrap();
                assert!(install_mutation_consumer(scope, document, callback));
            }
        }
        let expected = serde_json::to_string(oracle).unwrap();
        let source = format!("({}).then(value=>{{const expected=JSON.parse({expected}).result;globalThis.customResult=Object.keys(expected).filter(key=>JSON.stringify(value[key])!==JSON.stringify(expected[key])).map(key=>key+': '+JSON.stringify(value[key])+' EXPECTED '+JSON.stringify(expected[key])).join('\\n')||'PASS';}},error=>globalThis.customResult=String(error.stack));",probe.trim_end().trim_end_matches(';'));
        let code = v8::String::new(scope, &source).unwrap();
        v8::tc_scope!(let scope, scope);
        v8::Script::compile(scope, code, None)
            .and_then(|script| script.run(scope))
            .unwrap_or_else(|| {
                panic!("{}", scope.exception().unwrap().to_rust_string_lossy(scope))
            });
        scope.perform_microtask_checkpoint();
        let code = v8::String::new(scope, "customResult").unwrap();
        let result = v8::Script::compile(scope, code, None)
            .unwrap()
            .run(scope)
            .unwrap()
            .to_rust_string_lossy(scope);
        assert_eq!(result, "PASS");
        reset();
    }
    #[test]
    fn element_registry_ownership_moves_and_adoption_match_independent_chrome() {
        check_chrome_probe(
            include_str!("element_registry_probe.js"),
            include_str!("element_registry_chrome155.json"),
        );
    }
    #[test]
    fn custom_element_registry_and_lifecycle_match_independent_chrome() {
        check_chrome_probe(
            include_str!("custom_elements_probe.js"),
            include_str!("custom_elements_chrome155.json"),
        );
    }
    #[test]
    fn custom_element_move_before_uses_element_reaction_queues_matching_chrome() {
        check_chrome_probe(
            include_str!("custom_elements_moves_probe.js"),
            include_str!("custom_elements_moves_chrome155.json"),
        );
    }
    #[test]
    fn custom_element_cross_realm_adoption_and_borrowing_match_independent_chrome() {
        check_chrome_probe(
            include_str!("custom_elements_realms_probe.js"),
            include_str!("custom_elements_realms_chrome155.json"),
        );
    }
    #[test]
    fn adoption_with_reentrant_resource_consumers_matches_independent_chrome() {
        check_chrome_probe_with_consumers(
            include_str!("custom_elements_realms_probe.js"),
            include_str!("custom_elements_realms_chrome155.json"),
            true,
        );
    }

    #[test]
    fn contextual_parser_and_observer_lifecycle_match_independent_chrome() {
        check_chrome_probe_with_consumers(
            include_str!("dom_parser_observer_lifecycle_probe.js"),
            include_str!("dom_parser_observer_lifecycle_chrome155.json"),
            true,
        );
    }

    #[test]
    fn callback_realms_errors_and_reentrant_adoption_match_independent_chrome() {
        check_chrome_probe_with_consumers(
            include_str!("custom_elements_callbacks_probe.js"),
            include_str!("custom_elements_callbacks_chrome155.json"),
            true,
        );
    }

    fn retire_current_for_test(
        scope: &mut v8::PinScope,
        _args: v8::FunctionCallbackArguments,
        _result: v8::ReturnValue,
    ) {
        retire_context(scope.get_current_context());
    }

    #[test]
    fn already_queued_callbacks_stop_when_the_definition_realm_retires() {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(scope, "<!doctype html><body></body>");
        let global = context.global(scope);
        bind_method(
            scope,
            global,
            "stopDefinitionRealm",
            retire_current_for_test,
        );
        let actual = eval(
            scope,
            r#"globalThis.queuedRows=[];
          class Queued extends HTMLElement {
            static observedAttributes=['a','b'];
            constructor(){super();queuedRows.push('constructor');}
            attributeChangedCallback(name){queuedRows.push(name);if(name==='a')stopDefinitionRealm();}
            connectedCallback(){queuedRows.push('connected');}
          }
          customElements.define('x-retired-queued',Queued);
          document.body.innerHTML='<x-retired-queued a="one" b="two"></x-retired-queued>';
          JSON.stringify(queuedRows)"#,
        );
        assert_eq!(actual.to_rust_string_lossy(scope), r#"["constructor","a"]"#);
        reset();
    }

    #[test]
    fn retired_definition_context_retains_registry_but_suppresses_automatic_lifecycle() {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let main = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, main);
        install_with_html(scope, "<!doctype html><body></body>");
        let callback = eval(
            scope,
            "globalThis.borrowedRows=[];(function(){borrowedRows.push('callback')})",
        );
        let child = v8::Context::new(scope, Default::default());
        child.set_security_token(main.get_security_token(scope));
        {
            let scope = &mut v8::ContextScope::new(scope, child);
            install_independent_html(scope, "<!doctype html><body></body>");
            let global = child.global(scope);
            set_prop(scope, global, "parentCallback", callback);
            eval(
                scope,
                r#"globalThis.rows=[];
              globalThis.Retired=class extends HTMLElement {
                static observedAttributes=['data-v'];
                constructor(){super();rows.push('constructor');}
                connectedCallback(){rows.push('connected');}
                disconnectedCallback(){rows.push('disconnected');}
              };
              Retired.prototype.attributeChangedCallback=parentCallback;
              customElements.define('x-retired-unit',Retired);
              globalThis.kept=document.createElement('x-retired-unit');document.body.append(kept);
              globalThis.keptRegistry=customElements;rows.length=0;"#,
            );
        }
        retire_context(child);
        let global = main.global(scope);
        let child_global = child.global(scope);
        set_prop(scope, global, "child", child_global.into());
        assert!(eval(scope, r#"(()=>{
          const doc=child.document,created=doc.createElement('x-retired-unit');
          doc.body.append(created);created.setAttribute('data-v','new');
          const parsed=doc.createElement('div');parsed.innerHTML='<x-retired-unit></x-retired-unit>';doc.body.append(parsed);
          const range=doc.createRange();range.selectNodeContents(doc.body);const fragment=range.createContextualFragment('<x-retired-unit></x-retired-unit>');
          child.kept.setAttribute('data-v','retired');child.kept.remove();doc.body.append(child.kept);
          document.body.append(child.kept);child.kept.setAttribute('data-v','adopted');
          child.customElements.upgrade(created);
          return child.customElements===child.keptRegistry&&child.customElements.get('x-retired-unit')===child.Retired&&
            !(created instanceof child.Retired)&&!(parsed.firstChild instanceof child.Retired)&&!(fragment.firstChild instanceof child.Retired)&&
            child.kept instanceof child.Retired&&child.kept.ownerDocument===document&&child.rows.length===0&&borrowedRows.length===0;
        })()"#).is_true());
        reset();
    }

    #[test]
    fn custom_element_idl_conversion_and_failure_atomicity_match_independent_chrome() {
        check_chrome_probe(
            include_str!("custom_elements_idl_probe.js"),
            include_str!("custom_elements_idl_chrome155.json"),
        );
    }
    #[test]
    fn custom_element_reactions_and_clones_match_independent_chrome() {
        check_chrome_probe(
            include_str!("custom_elements_reactions_probe.js"),
            include_str!("custom_elements_reactions_chrome155.json"),
        );
    }
    #[test]
    fn custom_element_extensions_failures_and_scopes_match_independent_chrome() {
        check_chrome_probe(
            include_str!("custom_elements_extended_probe.js"),
            include_str!("custom_elements_extended_chrome155.json"),
        );
    }

    fn eval<'s>(scope: &mut v8::PinScope<'s, '_>, source: &str) -> v8::Local<'s, v8::Value> {
        v8::tc_scope!(let scope,scope);
        let code = v8::String::new(scope, source).unwrap();
        v8::Script::compile(scope, code, None)
            .and_then(|script| script.run(scope))
            .unwrap_or_else(|| panic!("{}", scope.exception().unwrap().to_rust_string_lossy(scope)))
    }
    #[test]
    fn global_registry_associations_are_primitive_and_parsed_tracking_stays_private() {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope,isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(scope, "<!doctype html><body></body>");
        let node = eval(
            scope,
            "globalThis.keptRegistryNode=document.createElement('div');keptRegistryNode",
        )
        .to_object(scope)
        .unwrap();
        let name = v8::String::new(scope, "turbo.custom-elements.association").unwrap();
        let key = v8::Private::for_api(scope, Some(name));
        assert!(node.get_private(scope, key).unwrap().is_false());
        assert!(eval(scope,"(()=>{const other=document.implementation.createHTMLDocument();other.adoptNode(keptRegistryNode);return keptRegistryNode.customElementRegistry===null&&!('__captureCustomElementParsed'in globalThis)})()").is_true());
        assert!(node.get_private(scope, key).unwrap().is_false());
        assert!(!recording());
        reset();
    }

    #[test]
    fn pages_without_custom_definitions_do_not_record_or_budget_plain_dom_mutations() {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope,isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(scope, "<!doctype html><body>");
        assert!(eval(scope,"document.body.innerHTML='<div data-value=plain>text</div>'.repeat(5000);document.body.childNodes.length===5000").is_true());
        assert!(!recording());
        assert!(scope.get_slot::<DefinitionsActive>().is_none());
        assert!(eval(scope,"customElements.define('x-lazy-activation',class extends HTMLElement{});document.createElement('x-lazy-activation') instanceof customElements.get('x-lazy-activation')").is_true());
        assert!(recording());
        assert!(scope.get_slot::<DefinitionsActive>().is_some());
        reset();
    }
    #[test]
    fn custom_move_and_adoption_payloads_ignore_guest_object_prototype_setters() {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope,isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(scope, "<!doctype html><body>");
        assert!(eval(scope,r#"(()=>{
          let moved=0,adopted=0,hooks=0;class Element extends HTMLElement{connectedMoveCallback(){moved++;}adoptedCallback(){adopted++;}}
          customElements.define('x-private-payload',Element);const node=document.createElement('x-private-payload'),parent=document.createElement('div'),other=document.implementation.createHTMLDocument();document.body.append(node,parent);
          const define=Object.defineProperty;
          try{for(const name of ['target','type'])define(Object.prototype,name,{configurable:true,set(){hooks++;throw Error('guest payload setter');}});parent.moveBefore(node,null);other.adoptNode(node);}
          finally{delete Object.prototype.target;delete Object.prototype.type;}
          return hooks===0&&moved===1&&adopted===1&&node.ownerDocument===other;
        })()"#).is_true());
        reset();
    }
    #[test]
    fn interrupted_definition_and_reaction_scopes_recover_without_guest_finally() {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope,isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(scope, "<!doctype html><body>");
        for source in [
            "class Definition extends HTMLElement{};Object.defineProperty(Definition.prototype,'connectedCallback',{get(){while(true){}}});customElements.define('x-budget-definition',Definition)",
            "globalThis.runaway=true;class Reaction extends HTMLElement{connectedCallback(){if(runaway)while(true){}}};customElements.define('x-budget-reaction',Reaction);document.body.append(document.createElement('x-budget-reaction'))"
        ]{
            let handle=scope.thread_safe_handle();let watchdog=std::thread::spawn(move||{std::thread::sleep(std::time::Duration::from_millis(50));handle.terminate_execution();});
            let code=v8::String::new(scope,source).unwrap();assert!(v8::Script::compile(scope,code,None).unwrap().run(scope).is_none());watchdog.join().unwrap();
            assert!(scope.get_slot::<Rc<DefinitionFrames>>().is_none_or(|frames|frames.0.borrow().is_empty()));
            assert!(scope.get_slot::<Rc<ReactionFrames>>().is_none_or(|frames|frames.0.borrow().is_empty()));
            scope.cancel_terminate_execution();
        }
        assert!(eval(scope,"runaway=false;customElements.define('x-recovered',class extends HTMLElement{});document.body.replaceChildren();document.body.append(document.createElement('x-recovered'));document.body.firstChild.localName==='x-recovered'").is_true());
        reset();
    }
    #[test]
    fn installed_document_without_custom_definitions_releases_context() {
        super::super::tests::init_v8();
        let mut isolate = v8::Isolate::new(Default::default());
        let context = {
            v8::scope!(let scope,&mut isolate);
            let context = v8::Context::new(scope, Default::default());
            let scope = &mut v8::ContextScope::new(scope, context);
            install_with_html(scope, "<!doctype html><body>");
            v8::Weak::new(scope, context)
        };
        for _ in 0..3 {
            isolate.low_memory_notification();
        }
        assert!(context.is_empty());
        clear_interface_records(&mut isolate);
        reset();
    }
    #[test]
    fn custom_element_private_records_retain_live_nodes_and_collect_unused_cycles() {
        super::super::tests::init_v8();
        let mut isolate = v8::Isolate::new(Default::default());
        let (context, weak_context, weak_node, weak_constructor, weak_shadow) = {
            v8::scope!(let scope,&mut isolate);
            let context = v8::Context::new(scope, Default::default());
            let scope = &mut v8::ContextScope::new(scope, context);
            install_with_html(scope, "<!doctype html><body>");
            let node=eval(scope,"(()=>{class Kept extends HTMLElement{connectedCallback(){this.counter=(this.counter||0)+1;}}customElements.define('x-kept',Kept);const node=document.createElement('x-kept');document.body.append(node);return node;})()").to_object(scope).unwrap();
            let constructor = eval(scope, "customElements.get('x-kept')")
                .to_object(scope)
                .unwrap();
            let shadow=eval(scope,"(()=>{globalThis.scopedRegistry=new CustomElementRegistry();const host=document.createElement('div'),shadow=host.attachShadow({mode:'closed',customElementRegistry:scopedRegistry});scopedRegistry.define('x-scoped-cycle',class extends HTMLElement{});shadow.innerHTML='<x-scoped-cycle></x-scoped-cycle>';return shadow;})()").to_object(scope).unwrap();
            (
                v8::Global::new(scope, context),
                v8::Weak::new(scope, context),
                v8::Weak::new(scope, node),
                v8::Weak::new(scope, constructor),
                v8::Weak::new(scope, shadow),
            )
        };
        for _ in 0..3 {
            isolate.low_memory_notification();
        }
        assert!(!weak_node.is_empty());
        assert!(!weak_constructor.is_empty());
        assert!(weak_shadow.is_empty());
        {
            v8::scope!(let scope,&mut isolate);
            let context = v8::Local::new(scope, &context);
            let scope = &mut v8::ContextScope::new(scope, context);
            assert!(eval(scope, "document.body.firstChild.counter===1").is_true());
            eval(scope, "document.body.replaceChildren()");
        }
        for _ in 0..3 {
            isolate.low_memory_notification();
        }
        assert!(weak_node.is_empty());
        assert!(!weak_constructor.is_empty());
        drop(context);
        for _ in 0..3 {
            isolate.low_memory_notification();
        }
        assert!(weak_context.is_empty());
        assert!(weak_constructor.is_empty());
        clear_interface_records(&mut isolate);
        reset();
    }

    #[test]
    fn native_construction_preserves_existing_nodes_and_nested_stack_identity() {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        setup(scope);
        let code = v8::String::new(scope, r#"(() => {
          const Base=nativeTestConstructor((target,prototype,existing)=>{
            const node=existing===undefined?document.createElement('div'):existing;
            Object.setPrototypeOf(node,prototype);return node;
          },'HTMLTestElement');
          const first=document.createElement('div'),second=document.createElement('div');
          class Nested extends Base { constructor(){super();this.marker='nested';} }
          class Outer extends Base { constructor(){super();this.other=nativeTestUpgrade(Nested,second);this.marker='outer';} }
          if(nativeTestUpgrade(Outer,first)!==first||first.other!==second||first.marker!=='outer'||second.marker!=='nested')throw Error('identity');
          class Throws extends Base {constructor(){super();throw Error('expected');}}
          try{nativeTestUpgrade(Throws,document.createElement('div'));throw Error('accepted');}catch(error){if(error.message!=='expected')throw error;}
          class Twice extends Base {constructor(){super();super();}}
          try{nativeTestUpgrade(Twice,document.createElement('div'));throw Error('accepted');}catch(error){if(error.name!=='TypeError')throw error;}
          class Wrong extends Base {constructor(){super();return {};}}
          try{nativeTestUpgrade(Wrong,document.createElement('div'));throw Error('accepted');}catch(error){if(error.name!=='TypeError')throw error;}
          const fresh=new Nested();if(fresh===first||fresh===second||!(fresh instanceof Nested))throw Error('failed stack cleanup');
          return true;
        })()"#).unwrap();
        v8::tc_scope!(let scope, scope);
        let value = v8::Script::compile(scope, code, None)
            .unwrap()
            .run(scope)
            .unwrap_or_else(|| {
                panic!("{}", scope.exception().unwrap().to_rust_string_lossy(scope))
            });
        assert!(value.is_true());
        assert!(scope
            .get_slot::<Rc<ConstructionFrames>>()
            .unwrap()
            .0
            .borrow()
            .is_empty());
        reset();
    }

    #[test]
    fn interrupted_native_construction_unwinds_without_guest_finally() {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        let handle = isolate.thread_safe_handle();
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        setup(scope);
        let code = v8::String::new(scope, r#"
          globalThis.testFinally=false;
          const Base=nativeTestConstructor((target,prototype,existing)=>existing,'HTMLTestElement');
          class Runaway extends Base {constructor(){super();try{while(true){}}finally{testFinally=true;}}}
          nativeTestUpgrade(Runaway,document.createElement('div'));
        "#).unwrap();
        let watchdog = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(50));
            handle.terminate_execution();
        });
        assert!(v8::Script::compile(scope, code, None)
            .unwrap()
            .run(scope)
            .is_none());
        watchdog.join().unwrap();
        assert!(scope
            .get_slot::<Rc<ConstructionFrames>>()
            .unwrap()
            .0
            .borrow()
            .is_empty());
        scope.cancel_terminate_execution();
        let code = v8::String::new(
            scope,
            "testFinally===false && document.createElement('p').nodeType===1",
        )
        .unwrap();
        assert!(v8::Script::compile(scope, code, None)
            .unwrap()
            .run(scope)
            .unwrap()
            .is_true());
        reset();
    }
}
