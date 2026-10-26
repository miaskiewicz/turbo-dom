//! Native mutation-time fanout and one agent-wide V8 notification microtask.
use super::*;
use std::rc::Rc;
use turbo_dom_parser::rtdom::mutations::{MutationEvent, MutationRecord};

#[derive(Default)]
struct Registry {
    observers: RefCell<Vec<v8::Weak<v8::Object>>>,
    notification: RefCell<Option<v8::Weak<v8::Function>>>,
}

thread_local! {
    static ENABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static SUPPRESSED: RefCell<HashMap<u64, usize>> = RefCell::new(HashMap::new());
}

#[derive(Default)]
pub(super) struct IsolationState {
    enabled: bool,
    suppressed: HashMap<u64, usize>,
}
pub(super) fn isolation_available() -> bool {
    SUPPRESSED.try_with(|_| ()).is_ok()
}
pub(super) fn take_isolation_state() -> IsolationState {
    IsolationState {
        enabled: ENABLED.with(|value| value.replace(false)),
        suppressed: SUPPRESSED.with(|rows| std::mem::take(&mut *rows.borrow_mut())),
    }
}
pub(super) fn put_isolation_state(state: IsolationState) {
    ENABLED.with(|value| value.set(state.enabled));
    SUPPRESSED.with(|rows| *rows.borrow_mut() = state.suppressed);
}
pub(super) fn recording() -> bool {
    custom_elements::recording()
        || ENABLED.with(std::cell::Cell::get)
        || script_state::recording()
        || DOM.with(|state| {
            state
                .borrow()
                .as_ref()
                .is_some_and(|state| !state.shadow_modes.is_empty())
        })
}

pub(super) fn suppressed() -> bool {
    let id = DOM_ID.with(std::cell::Cell::get);
    SUPPRESSED.with(|depths| depths.borrow().get(&id).copied().unwrap_or(0) != 0)
}

struct Suppression(u64);
impl Drop for Suppression {
    fn drop(&mut self) {
        SUPPRESSED.with(|depths| {
            let mut depths = depths.borrow_mut();
            if let Some(depth) = depths.get_mut(&self.0) {
                *depth -= 1;
            }
            depths.retain(|_, depth| *depth != 0);
        });
    }
}

/// DOM compound algorithms queue their explicit semantic record after executing
/// the suppressed primitives. Earlier source removals retain their own records.
pub(super) fn quiet<R>(
    scope: &mut v8::PinScope,
    operation: impl FnOnce(&mut v8::PinScope) -> R,
) -> R {
    if !recording() {
        return operation(scope);
    }
    flush(scope);
    let id = DOM_ID.with(std::cell::Cell::get);
    SUPPRESSED.with(|depths| *depths.borrow_mut().entry(id).or_default() += 1);
    let _guard = Suppression(id);
    operation(scope)
}

pub(super) fn child_list(
    scope: &mut v8::PinScope,
    parent: Handle,
    added: Vec<Handle>,
    removed: Vec<Handle>,
    previous_sibling: Option<Handle>,
    next_sibling: Option<Handle>,
) {
    if !recording() || added.is_empty() && removed.is_empty() {
        return;
    }
    let ancestors = with_tree(|tree| {
        let mut ancestors = Vec::new();
        let mut node = Some(parent);
        while let Some(handle) = node {
            ancestors.push(handle);
            node = tree.parent(handle);
        }
        ancestors
    })
    .unwrap_or_default();
    let event = MutationEvent {
        record: MutationRecord::ChildList {
            target: parent,
            added,
            removed,
        },
        inclusive_ancestors: ancestors,
        previous_sibling,
        next_sibling,
        attribute_namespace: None,
    };
    let observers = scope
        .get_slot::<Rc<Registry>>()
        .cloned()
        .map(|registry| {
            registry
                .observers
                .borrow()
                .iter()
                .filter_map(|weak| weak.to_local(scope))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let native_event = event;
    shadow_slots::mutation(scope, &native_event);
    let event = payload(scope, &native_event);
    custom_elements::mutation(scope, event);
    for observer in observers {
        if let Some(queue) = function(scope, observer, "queue") {
            let receiver = v8::undefined(scope);
            queue.call(scope, receiver.into(), &[event.into()]);
        }
    }
    consume(scope, event);
    script_state::inactive_mutation(&native_event);
}

fn key<'s>(scope: &mut v8::PinScope<'s, '_>, name: &str) -> v8::Local<'s, v8::Private> {
    let name = v8::String::new(scope, &format!("turbo.mutations.{name}")).unwrap();
    v8::Private::for_api(scope, Some(name))
}

fn function<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    observer: v8::Local<v8::Object>,
    name: &str,
) -> Option<v8::Local<'s, v8::Function>> {
    let key = key(scope, name);
    observer
        .get_private(scope, key)
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
}

fn registry(scope: &mut v8::PinScope) -> Rc<Registry> {
    scope
        .get_slot::<Rc<Registry>>()
        .cloned()
        .unwrap_or_else(|| {
            let registry = Rc::new(Registry::default());
            scope.set_slot(registry.clone());
            registry
        })
}

/// Trusted embedder registration. The owning Document retains its callback;
/// there is no guest setter or isolate-wide strong realm root.
pub fn install_mutation_consumer(
    scope: &mut v8::PinScope,
    document: v8::Local<v8::Object>,
    callback: v8::Local<v8::Function>,
) -> bool {
    let _owner = DomScope::for_object(scope, document);
    let Some(handle) = handle_of(scope, document) else {
        return false;
    };
    if with_tree(|tree| tree.node_type_id(handle)) != Some(9) {
        return false;
    }
    if !DOM.with(|dom| {
        dom.borrow()
            .as_ref()
            .is_some_and(|state| state.browsing_context)
    }) {
        return false;
    }
    let Some(context) = document.get_creation_context(scope) else {
        return false;
    };
    if callback.get_creation_context(scope) != Some(context) {
        return false;
    }
    let private = key(scope, "consumer");
    let registered = document
        .set_private(scope, private, callback.into())
        .unwrap_or(false);
    if registered {
        ENABLED.with(|enabled| enabled.set(true));
        registry(scope);
        with_tree_mut(Tree::start_recording);
    }
    registered
}

pub fn remove_mutation_consumer(scope: &mut v8::PinScope, document: v8::Local<v8::Object>) {
    let private = key(scope, "consumer");
    document.delete_private(scope, private);
}

fn consume(scope: &mut v8::PinScope, event: v8::Local<v8::Object>) {
    let Some(root) = with_tree(Tree::root) else {
        return;
    };
    let graph = DOM.with(|dom| dom.borrow().as_ref()?.ownership.clone());
    let Some(graph) = graph else {
        return;
    };
    let document = {
        let graph = graph.borrow();
        if !graph.context_alive() || graph.cached() == 0 {
            return;
        }
        graph.lookup(scope, root.raw())
    };
    let Some(document) = document else {
        return;
    };
    let Some(consumer) = function(scope, document, "consumer") else {
        return;
    };
    let Some(context) = document.get_creation_context(scope) else {
        return;
    };
    let scope = &mut v8::ContextScope::new(scope, context);
    let receiver = v8::undefined(scope);
    consumer.call(scope, receiver.into(), &[event.into()]);
}

fn deliver(scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue) {
    registry(scope).notification.borrow_mut().take();
    let Ok(pending) = v8::Local::<v8::Array>::try_from(args.data()) else {
        return;
    };
    let private = key(scope, "signal-slots");
    let slots = pending
        .get_private(scope, private)
        .and_then(|value| v8::Local::<v8::Array>::try_from(value).ok());
    let count = pending.length();
    let slot_count = slots.map_or(0, |slots| slots.length());
    for index in 0..count {
        let Some(observer) = pending
            .get_index(scope, index)
            .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
        else {
            continue;
        };
        let Some(notify) = function(scope, observer, "notify") else {
            continue;
        };
        v8::tc_scope!(let catch, scope);
        catch.set_verbose(true);
        let receiver = v8::undefined(catch);
        notify.call(catch, receiver.into(), &[]);
        if catch.is_execution_terminating() {
            return;
        }
    }
    if let Some(slots) = slots {
        for index in 0..slot_count {
            let Some(slot) = slots
                .get_index(scope, index)
                .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
            else {
                continue;
            };
            v8::tc_scope!(let catch, scope);
            catch.set_verbose(true);
            shadow_slots::fire(catch, slot);
            if catch.is_execution_terminating() {
                return;
            }
        }
    }
}
fn notification<'s>(scope: &mut v8::PinScope<'s, '_>) -> Option<v8::Local<'s, v8::Array>> {
    let registry = registry(scope);
    let private = key(scope, "pending");
    if let Some(current) = registry
        .notification
        .borrow()
        .as_ref()
        .and_then(|weak| weak.to_local(scope))
    {
        if let Some(pending) = current
            .get_private(scope, private)
            .and_then(|value| v8::Local::<v8::Array>::try_from(value).ok())
        {
            return Some(pending);
        }
    }
    let pending = v8::Array::new(scope, 0);
    let notification = v8::Function::builder(deliver)
        .data(pending.into())
        .build(scope)?;
    notification.set_private(scope, private, pending.into());
    *registry.notification.borrow_mut() = Some(v8::Weak::new(scope, notification));
    scope.enqueue_microtask(notification);
    Some(pending)
}
fn append_unique(
    scope: &mut v8::PinScope,
    pending: v8::Local<v8::Array>,
    object: v8::Local<v8::Object>,
) {
    for index in 0..pending.length() {
        if pending
            .get_index(scope, index)
            .is_some_and(|value| value.strict_equals(object.into()))
        {
            return;
        }
    }
    let index = v8::String::new(scope, &pending.length().to_string()).unwrap();
    pending.create_data_property(scope, index.into(), object.into());
}
fn schedule(scope: &mut v8::PinScope, observer: v8::Local<v8::Object>) {
    if let Some(pending) = notification(scope) {
        append_unique(scope, pending, observer);
    }
}
pub(super) fn signal_slot(scope: &mut v8::PinScope, slot: v8::Local<v8::Object>) {
    let Some(pending) = notification(scope) else {
        return;
    };
    let private = key(scope, "signal-slots");
    let slots = pending
        .get_private(scope, private)
        .and_then(|value| v8::Local::<v8::Array>::try_from(value).ok())
        .unwrap_or_else(|| {
            let slots = v8::Array::new(scope, 0);
            pending.set_private(scope, private, slots.into());
            slots
        });
    append_unique(scope, slots, slot);
}

fn anchor(
    scope: &mut v8::PinScope,
    observer: v8::Local<v8::Object>,
    target: v8::Local<v8::Object>,
    add: bool,
) {
    let private = key(scope, "registrations");
    let previous = target
        .get_private(scope, private)
        .and_then(|value| v8::Local::<v8::Array>::try_from(value).ok());
    let mut rows = Vec::new();
    let mut found = false;
    if let Some(previous) = previous {
        for index in 0..previous.length() {
            if let Some(value) = previous.get_index(scope, index) {
                if value.strict_equals(observer.into()) {
                    found = true;
                    if !add {
                        continue;
                    }
                }
                rows.push(value);
            }
        }
    }
    if add && !found {
        rows.push(observer.into());
    }
    let next = v8::Array::new_with_elements(scope, &rows);
    target.set_private(scope, private, next.into());
}

fn native<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    _rv: v8::ReturnValue,
) {
    let operation = args.get(0).to_rust_string_lossy(scope);
    if operation == "flush" {
        flush(scope);
        return;
    }
    let Ok(object) = v8::Local::<v8::Object>::try_from(args.get(1)) else {
        return;
    };
    match operation.as_str() {
        "register" => {
            for (name, index) in [("queue", 2), ("notify", 3)] {
                let private = key(scope, name);
                object.set_private(scope, private, args.get(index));
            }
            registry(scope)
                .observers
                .borrow_mut()
                .push(v8::Weak::new(scope, object));
        }
        "record" => {
            collection_records::set(scope, object, args.get(2));
        }
        "recordTree" => {
            ENABLED.with(|enabled| enabled.set(true));
            let _owner = DomScope::for_object(scope, object);
            with_tree_mut(Tree::start_recording);
        }
        "anchor" => {
            if let Ok(target) = v8::Local::<v8::Object>::try_from(args.get(2)) {
                anchor(scope, object, target, args.get(3).is_true());
            }
        }
        "schedule" => schedule(scope, object),
        _ => {}
    }
}

fn nodes<'s>(scope: &mut v8::PinScope<'s, '_>, handles: &[Handle]) -> v8::Local<'s, v8::Array> {
    let values = handles
        .iter()
        .map(|handle| wrap(scope, *handle).into())
        .collect::<Vec<_>>();
    v8::Array::new_with_elements(scope, &values)
}

fn payload<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    event: &MutationEvent,
) -> v8::Local<'s, v8::Object> {
    let object = v8::Object::new(scope);
    let null = v8::null(scope);
    object.set_prototype(scope, null.into());
    let kind = match event.record {
        MutationRecord::ChildList { .. } => "childList",
        MutationRecord::Attributes { .. } => "attributes",
        MutationRecord::CharacterData { .. } => "characterData",
    };
    let kind = v8::String::new(scope, kind).unwrap();
    set_prop(scope, object, "type", kind.into());
    let target = wrap(scope, event.record.target());
    set_prop(scope, object, "target", target.into());
    for (name, handles) in [
        ("ancestors", event.inclusive_ancestors.as_slice()),
        ("added", event.record.added()),
        ("removed", event.record.removed()),
    ] {
        let values = nodes(scope, handles);
        set_prop(scope, object, name, values.into());
        if name == "added" {
            set_prop(scope, object, "addedNodes", values.into());
        }
        if name == "removed" {
            set_prop(scope, object, "removedNodes", values.into());
        }
    }
    let candidates = script_state::candidates(event);
    let candidates = nodes(scope, &candidates);
    set_prop(scope, object, "scriptCandidates", candidates.into());
    for (name, handle) in [
        ("previousSibling", event.previous_sibling),
        ("nextSibling", event.next_sibling),
    ] {
        let value = handle.map_or_else(|| null.into(), |handle| wrap(scope, handle).into());
        set_prop(scope, object, name, value);
    }
    for (name, value) in [
        ("attributeName", event.record.attribute_name()),
        ("attributeNamespace", event.attribute_namespace.as_deref()),
        ("oldValue", event.record.old_value()),
    ] {
        let value = value.map_or_else(
            || null.into(),
            |value| v8::String::new(scope, value).unwrap().into(),
        );
        set_prop(scope, object, name, value);
    }
    object
}

pub(super) fn flush(scope: &mut v8::PinScope) {
    form_checked_state::owner_changes();
    if !recording() || scope.is_execution_terminating() {
        return;
    }
    let observers = scope
        .get_slot::<Rc<Registry>>()
        .cloned()
        .map(|registry| {
            let mut observers = registry.observers.borrow_mut();
            observers.retain(|observer| !observer.is_empty());
            observers
                .iter()
                .filter_map(|weak| weak.to_local(scope))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let current = DOM_ID.with(std::cell::Cell::get);
    let mut ids = OTHER_DOMS.with(|doms| doms.borrow().keys().copied().collect::<Vec<_>>());
    ids.push(current);
    // Claim every document's pending batch before invoking any consumer. A
    // consumer can query another document, which reenters this fanout. Leaving
    // its old events in Tree would move them into the inner CE reaction scope
    // and run adoption callbacks before ownerDocument has been updated.
    let batches = ids
        .into_iter()
        .map(|id| {
            let _owner = DomScope::enter(id);
            let events = DOM
                .with(|dom| {
                    dom.borrow_mut()
                        .as_mut()
                        .map(|state| state.tree.take_mutation_events_utf16())
                })
                .unwrap_or_default();
            (id, events)
        })
        .collect::<Vec<_>>();
    for (id, events) in batches {
        let _owner = DomScope::enter(id);
        for native_event in events {
            shadow_slots::mutation(scope, &native_event.event);
            let event = payload(scope, &native_event.event);
            for (name, units) in [
                ("oldValue", &native_event.old_value),
                ("attributeName", &native_event.attribute_name),
                ("attributeNamespace", &native_event.attribute_namespace),
            ] {
                let value = match units {
                    Some(units) => unit_string(scope, units).into(),
                    None => v8::null(scope).into(),
                };
                set_prop(scope, event, name, value);
            }
            custom_elements::mutation(scope, event);
            for observer in &observers {
                if let Some(queue) = function(scope, *observer, "queue") {
                    let receiver = v8::undefined(scope);
                    queue.call(scope, receiver.into(), &[event.into()]);
                }
            }
            consume(scope, event);
            script_state::inactive_mutation(&native_event.event);
        }
    }
}

pub(super) fn install(scope: &mut v8::PinScope) {
    let global = scope.get_current_context().global(scope);
    bind_method(scope, global, "__mutationObserverNative", native);
    run_js(scope, include_str!("mutation_observers.js"));
}

pub(super) fn clear(isolate: &mut v8::Isolate) {
    isolate.remove_slot::<Rc<Registry>>();
    let _ = ENABLED.try_with(|enabled| enabled.set(false));
    let _ = SUPPRESSED.try_with(|depths| depths.borrow_mut().clear());
}
