//! Native shadow distribution, private manual-assignment edges and slot signals.
use super::*;
use std::collections::HashSet;
use turbo_dom_parser::rtdom::mutations::{MutationEvent, MutationRecord};

const KEY_NAMES: [&str; 5] = ["manual", "assigned", "manual-slot", "distribution", "Event"];
struct Keys(Vec<v8::Global<v8::Private>>);
fn key<'s>(scope: &mut v8::PinScope<'s, '_>, name: &str) -> v8::Local<'s, v8::Private> {
    // Private symbols have no realm edges. Capture them before guest execution so
    // mutation paths need no string allocation after the watchdog terminates V8.
    let keys = scope
        .get_slot::<std::rc::Rc<Keys>>()
        .cloned()
        .expect("installed shadow keys");
    let index = KEY_NAMES
        .iter()
        .position(|key| *key == name)
        .expect("constant shadow key");
    v8::Local::new(scope, &keys.0[index])
}
fn slot(tree: &Tree, node: Handle) -> bool {
    tree.node_type_id(node) == 1
        && tree.namespace_id(node) == 0
        && tree.local_name(node) == Some("slot")
}
fn slottable(tree: &Tree, node: Handle) -> bool {
    matches!(tree.node_type_id(node), 1 | 3 | 4)
}
fn root(tree: &Tree, node: Handle) -> Option<Handle> {
    let mut cursor = node;
    while let Some(parent) = tree.parent(cursor) {
        cursor = parent;
    }
    tree.is_shadow_root(cursor).then_some(cursor)
}
fn slots(tree: &Tree, root: Handle) -> Vec<Handle> {
    let mut pending = tree.children(root);
    pending.reverse();
    let mut result = Vec::new();
    while let Some(node) = pending.pop() {
        if slot(tree, node) {
            result.push(node);
        }
        let mut children = tree.children(node);
        children.reverse();
        pending.extend(children);
    }
    result
}
fn attribute(tree: &Tree, node: Handle, name: &str) -> Vec<u16> {
    tree.get_attribute(node, name)
        .unwrap_or_default()
        .encode_utf16()
        .collect()
}
pub(super) fn manual(scope: &mut v8::PinScope, root: Handle) -> bool {
    let object = wrap(scope, root);
    let private = key(scope, "manual");
    object
        .get_private(scope, private)
        .is_some_and(|value| value.is_true())
}
fn assigned<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    handle: Handle,
) -> Vec<v8::Local<'s, v8::Object>> {
    let Some((root, host)) = with_tree(|tree| {
        let root = root(tree, handle)?;
        Some((root, tree.shadow_host(root)?))
    })
    .flatten() else {
        return Vec::new();
    };
    if manual(scope, root) {
        let object = wrap(scope, handle);
        let private = key(scope, "assigned");
        let Some(nodes) = object
            .get_private(scope, private)
            .and_then(|value| v8::Local::<v8::Array>::try_from(value).ok())
        else {
            return Vec::new();
        };
        let owner = DOM_ID.with(std::cell::Cell::get);
        let reverse = key(scope, "manual-slot");
        let mut result = Vec::new();
        for index in 0..nodes.length() {
            let Some(node) = nodes
                .get_index(scope, index)
                .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
            else {
                continue;
            };
            if dom_id_of(scope, node) != owner
                || !node
                    .get_private(scope, reverse)
                    .is_some_and(|value| value.strict_equals(object.into()))
            {
                continue;
            }
            let Some(handle) = handle_of(scope, node) else {
                continue;
            };
            if with_tree(|tree| slottable(tree, handle) && tree.parent(handle) == Some(host))
                .unwrap_or(false)
            {
                result.push(node);
            }
        }
        return result;
    }
    let handles = with_tree(|tree| {
        let name = attribute(tree, handle, "name");
        if slots(tree, root)
            .into_iter()
            .find(|node| attribute(tree, *node, "name") == name)
            != Some(handle)
        {
            return Vec::new();
        }
        tree.children(host)
            .into_iter()
            .filter(|node| slottable(tree, *node) && attribute(tree, *node, "slot") == name)
            .collect::<Vec<_>>()
    })
    .unwrap_or_default();
    handles.into_iter().map(|node| wrap(scope, node)).collect()
}
fn flattened<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    handle: Handle,
) -> Vec<v8::Local<'s, v8::Object>> {
    if with_tree(|tree| root(tree, handle).is_none()).unwrap_or(true) {
        return Vec::new();
    }
    let mut active = HashSet::new();
    let mut pending = vec![(wrap(scope, handle), false)];
    let mut result = Vec::new();
    while let Some((node, leaving)) = pending.pop() {
        if scope.is_execution_terminating() {
            return Vec::new();
        }
        let Some(handle) = handle_of(scope, node) else {
            continue;
        };
        if leaving {
            active.remove(&handle);
            continue;
        }
        if !with_tree(|tree| slot(tree, handle) && root(tree, handle).is_some()).unwrap_or(false) {
            result.push(node);
            continue;
        }
        if !active.insert(handle) {
            continue;
        }
        pending.push((node, true));
        let mut nodes = assigned(scope, handle);
        if nodes.is_empty() {
            nodes = with_tree(|tree| {
                tree.children(handle)
                    .into_iter()
                    .filter(|node| slottable(tree, *node))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
            .into_iter()
            .map(|node| wrap(scope, node))
            .collect();
        }
        pending.extend(nodes.into_iter().rev().map(|node| (node, false)));
    }
    result
}
fn assigned_slot<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    node: v8::Local<'s, v8::Object>,
) -> Option<v8::Local<'s, v8::Object>> {
    let handle = handle_of(scope, node)?;
    let root = with_tree(|tree| tree.shadow_root(tree.parent(handle)?)).flatten()?;
    let handles = with_tree(|tree| slots(tree, root))?;
    for slot in handles {
        if scope.is_execution_terminating() {
            return None;
        }
        if assigned(scope, slot)
            .iter()
            .any(|assigned| assigned.strict_equals(node.into()))
        {
            return Some(wrap(scope, slot));
        }
    }
    None
}
/// Embedder-only flat-tree projection. Closed roots remain visible to native
/// layout, and manual edges are read from V8 private records, never guest APIs.
pub(super) fn composed_children<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    node: v8::Local<'s, v8::Object>,
) -> Option<Vec<v8::Local<'s, v8::Object>>> {
    if scope.is_execution_terminating() {
        return None;
    }
    let _owner = DomScope::for_object(scope, node);
    let handle = handle_of(scope, node)?;
    let context = node.get_creation_context(scope)?;
    let scope = &mut v8::ContextScope::new(scope, context);
    let is_slot = with_tree(|tree| slot(tree, handle) && root(tree, handle).is_some())?;
    if is_slot {
        let assigned = assigned(scope, handle);
        if !assigned.is_empty() {
            return Some(assigned);
        }
    }
    let children = with_tree(|tree| tree.children(tree.shadow_root(handle).unwrap_or(handle)))?;
    Some(
        children
            .into_iter()
            .map(|child| wrap(scope, child))
            .collect(),
    )
}
pub(super) fn composed_parent<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    node: v8::Local<'s, v8::Object>,
) -> Option<v8::Local<'s, v8::Object>> {
    if scope.is_execution_terminating() {
        return None;
    }
    let _owner = DomScope::for_object(scope, node);
    let handle = handle_of(scope, node)?;
    let context = node.get_creation_context(scope)?;
    let scope = &mut v8::ContextScope::new(scope, context);
    if let Some(host) = with_tree(|tree| {
        tree.is_shadow_root(handle)
            .then(|| tree.shadow_host(handle))
    })
    .flatten()
    .flatten()
    {
        return Some(wrap(scope, host));
    }
    let parent = with_tree(|tree| tree.parent(handle)).flatten()?;
    if with_tree(|tree| tree.shadow_root(parent).is_some())? {
        return assigned_slot(scope, node);
    }
    if with_tree(|tree| slot(tree, parent) && root(tree, parent).is_some())?
        && !assigned(scope, parent).is_empty()
    {
        return None;
    }
    let parent = with_tree(|tree| tree.shadow_host(parent).unwrap_or(parent))?;
    Some(wrap(scope, parent))
}
pub(super) fn configure(scope: &mut v8::PinScope, root: v8::Local<v8::Object>, manual: bool) {
    let private = key(scope, "manual");
    let value = v8::Boolean::new(scope, manual);
    root.set_private(scope, private, value.into());
    with_tree_mut(Tree::start_recording);
}
fn array<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    nodes: &[v8::Local<'s, v8::Object>],
) -> v8::Local<'s, v8::Array> {
    let values = nodes.iter().map(|node| (*node).into()).collect::<Vec<_>>();
    v8::Array::new_with_elements(scope, &values)
}
fn previous<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    root: v8::Local<v8::Object>,
) -> Vec<(v8::Local<'s, v8::Object>, v8::Local<'s, v8::Array>)> {
    let private = key(scope, "distribution");
    let Some(rows) = root
        .get_private(scope, private)
        .and_then(|value| v8::Local::<v8::Array>::try_from(value).ok())
    else {
        return Vec::new();
    };
    let mut result = Vec::new();
    for index in 0..rows.length() {
        let Some(row) = rows
            .get_index(scope, index)
            .and_then(|value| v8::Local::<v8::Array>::try_from(value).ok())
        else {
            continue;
        };
        let slot = row
            .get_index(scope, 0)
            .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok());
        let nodes = row
            .get_index(scope, 1)
            .and_then(|value| v8::Local::<v8::Array>::try_from(value).ok());
        if let (Some(slot), Some(nodes)) = (slot, nodes) {
            result.push((slot, nodes));
        }
    }
    result
}
pub(super) fn refresh(scope: &mut v8::PinScope) {
    if scope.is_execution_terminating() {
        return;
    }
    let mut roots = DOM.with(|state| {
        state
            .borrow()
            .as_ref()
            .map(|state| state.shadow_modes.keys().copied().collect::<Vec<_>>())
            .unwrap_or_default()
    });
    roots.sort_unstable_by_key(|node| node.raw());
    for root in roots {
        if scope.is_execution_terminating() {
            return;
        }
        if !with_tree(|tree| tree.is_live(root)).unwrap_or(false) {
            continue;
        }
        let object = wrap(scope, root);
        let Some(context) = object.get_creation_context(scope) else {
            continue;
        };
        let scope = &mut v8::ContextScope::new(scope, context);
        let old = previous(scope, object);
        let handles = with_tree(|tree| slots(tree, root)).unwrap_or_default();
        let mut rows = Vec::new();
        let mut current = Vec::new();
        for handle in handles {
            if scope.is_execution_terminating() {
                return;
            }
            let object = wrap(scope, handle);
            let nodes = assigned(scope, handle);
            let before = old
                .iter()
                .find(|(slot, _)| slot.strict_equals(object.into()))
                .map(|(_, nodes)| *nodes);
            let equal = before.is_some_and(|before| {
                before.length() as usize == nodes.len()
                    && nodes.iter().enumerate().all(|(index, node)| {
                        before
                            .get_index(scope, index as u32)
                            .is_some_and(|old| old.strict_equals((*node).into()))
                    })
            });
            if !equal && (!nodes.is_empty() || before.is_some_and(|nodes| nodes.length() != 0)) {
                mutation_observers::signal_slot(scope, object);
            }
            let nodes = array(scope, &nodes);
            let row = v8::Array::new_with_elements(scope, &[object.into(), nodes.into()]);
            rows.push(row.into());
            current.push(object);
        }
        for (slot, nodes) in old {
            if nodes.length() != 0
                && !current
                    .iter()
                    .any(|current| current.strict_equals(slot.into()))
            {
                mutation_observers::signal_slot(scope, slot);
            }
        }
        let rows = v8::Array::new_with_elements(scope, &rows);
        let private = key(scope, "distribution");
        object.set_private(scope, private, rows.into());
    }
}
pub(super) fn mutation(scope: &mut v8::PinScope, event: &MutationEvent) {
    refresh(scope);
    if let MutationRecord::ChildList {
        target,
        added,
        removed,
    } = &event.record
    {
        if with_tree(|tree| {
            slot(tree, *target)
                && (added
                    .iter()
                    .chain(removed)
                    .any(|node| slottable(tree, *node)))
        })
        .unwrap_or(false)
            && assigned(scope, *target).is_empty()
            && with_tree(|tree| root(tree, *target).is_some()).unwrap_or(false)
        {
            let object = wrap(scope, *target);
            mutation_observers::signal_slot(scope, object);
        }
    }
}
fn assign(scope: &mut v8::PinScope, slot: v8::Local<v8::Object>, arguments: v8::Local<v8::Object>) {
    let Some(length) = v8::String::new(scope, "length") else {
        return;
    };
    let Some(length) = arguments
        .get(scope, length.into())
        .and_then(|value| value.uint32_value(scope))
    else {
        return;
    };
    let mut nodes = Vec::new();
    for index in 0..length {
        let Some(node) = arguments
            .get_index(scope, index)
            .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
        else {
            illegal(scope);
            return;
        };
        let _owner = DomScope::for_object(scope, node);
        let Some(handle) = handle_of(scope, node) else {
            illegal(scope);
            return;
        };
        if !with_tree(|tree| slottable(tree, handle)).unwrap_or(false) {
            illegal(scope);
            return;
        }
        if !nodes
            .iter()
            .any(|old: &v8::Local<v8::Object>| old.strict_equals(node.into()))
        {
            nodes.push(node);
        }
    }
    let reverse = key(scope, "manual-slot");
    let private = key(scope, "assigned");
    let previous = slot
        .get_private(scope, private)
        .and_then(|value| v8::Local::<v8::Array>::try_from(value).ok());
    if let Some(previous) = previous {
        for index in 0..previous.length() {
            let Some(node) = previous
                .get_index(scope, index)
                .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
            else {
                continue;
            };
            if !nodes
                .iter()
                .any(|retained| retained.strict_equals(node.into()))
                && node
                    .get_private(scope, reverse)
                    .is_some_and(|owner| owner.strict_equals(slot.into()))
            {
                node.delete_private(scope, reverse);
            }
        }
    }
    let mut affected = vec![dom_id_of(scope, slot)];
    for node in &nodes {
        if let Some(old) = node
            .get_private(scope, reverse)
            .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
        {
            if !old.strict_equals(slot.into()) {
                let owner = dom_id_of(scope, old);
                if !affected.contains(&owner) {
                    affected.push(owner);
                }
                if let Some(previous) = old
                    .get_private(scope, private)
                    .and_then(|value| v8::Local::<v8::Array>::try_from(value).ok())
                {
                    let mut retained = Vec::new();
                    for index in 0..previous.length() {
                        if let Some(value) = previous.get_index(scope, index) {
                            if !value.strict_equals((*node).into()) {
                                retained.push(value);
                            }
                        }
                    }
                    let Some(context) = old.get_creation_context(scope) else {
                        return;
                    };
                    let scope = &mut v8::ContextScope::new(scope, context);
                    let next = v8::Array::new_with_elements(scope, &retained);
                    old.set_private(scope, private, next.into());
                }
            }
        }
        node.set_private(scope, reverse, slot.into());
    }
    let Some(context) = slot.get_creation_context(scope) else {
        return;
    };
    {
        let scope = &mut v8::ContextScope::new(scope, context);
        let next = array(scope, &nodes);
        slot.set_private(scope, private, next.into());
    }
    for owner in affected {
        let _owner = DomScope::enter(owner);
        refresh(scope);
    }
}
fn illegal(scope: &mut v8::PinScope) {
    let Some(text) = v8::String::new(scope, "Illegal invocation") else {
        return;
    };
    let error = v8::Exception::type_error(scope, text);
    scope.throw_exception(error);
}
fn native(scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue) {
    if scope.is_execution_terminating() {
        return;
    }
    let operation = args.get(0).to_rust_string_lossy(scope);
    let Ok(object) = v8::Local::<v8::Object>::try_from(args.get(1)) else {
        illegal(scope);
        return;
    };
    let _owner = DomScope::for_object(scope, object);
    let Some(handle) = handle_of(scope, object) else {
        illegal(scope);
        return;
    };
    match operation.as_str() {
        "mode" => {
            let mode = if manual(scope, handle) {
                "manual"
            } else {
                "named"
            };
            if let Some(mode) = v8::String::new(scope, mode) {
                rv.set(mode.into());
            }
        }
        "assignedSlot" => {
            let found = with_tree(|tree| {
                let host = tree.parent(handle)?;
                Some((tree.shadow_root(host)?, host))
            })
            .flatten();
            let Some((root, _)) = found else {
                rv.set(v8::null(scope).into());
                return;
            };
            if DOM.with(|state| {
                state
                    .borrow()
                    .as_ref()
                    .and_then(|state| state.shadow_modes.get(&root))
                    .is_some_and(|mode| mode == "closed")
            }) {
                rv.set(v8::null(scope).into());
                return;
            }
            let handles = with_tree(|tree| slots(tree, root)).unwrap_or_default();
            for handle in handles {
                if assigned(scope, handle)
                    .iter()
                    .any(|node| node.strict_equals(object.into()))
                {
                    rv.set(wrap(scope, handle).into());
                    return;
                }
            }
            rv.set(v8::null(scope).into());
        }
        "assignedNodes" | "assignedElements" => {
            let mut nodes = if args.get(2).is_true() {
                flattened(scope, handle)
            } else {
                assigned(scope, handle)
            };
            if operation == "assignedElements" {
                nodes.retain(|node| {
                    handle_of(scope, *node)
                        .and_then(|handle| with_tree(|tree| tree.node_type_id(handle)))
                        == Some(1)
                });
            }
            let result = array(scope, &nodes);
            rv.set(result.into());
        }
        "assign" => {
            if let Ok(arguments) = v8::Local::<v8::Object>::try_from(args.get(2)) {
                assign(scope, object, arguments);
            }
        }
        _ => {}
    }
}
pub(super) fn fire(scope: &mut v8::PinScope, slot: v8::Local<v8::Object>) {
    if scope.is_execution_terminating() {
        return;
    }
    let Some(context) = slot.get_creation_context(scope) else {
        return;
    };
    let scope = &mut v8::ContextScope::new(scope, context);
    let _owner = DomScope::for_object(scope, slot);
    let global = context.global(scope);
    let private = key(scope, "Event");
    let Some(constructor) = global
        .get_private(scope, private)
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
    else {
        return;
    };
    let options = v8::Object::new(scope);
    let null = v8::null(scope);
    options.set_prototype(scope, null.into());
    let Some(name) = v8::String::new(scope, "bubbles") else {
        return;
    };
    let yes = v8::Boolean::new(scope, true);
    options.create_data_property(scope, name.into(), yes.into());
    let Some(name) = v8::String::new(scope, "slotchange") else {
        return;
    };
    if let Some(event) = constructor.new_instance(scope, &[name.into(), options.into()]) {
        dispatch_host_event(scope, event, slot);
    }
}
/// Capture the slot keys (idempotent); parsing may attach roots before install.
pub(super) fn ensure_keys(scope: &mut v8::PinScope) {
    if scope.get_slot::<std::rc::Rc<Keys>>().is_none() {
        let mut keys = Vec::new();
        for name in KEY_NAMES {
            let Some(name) = v8::String::new(scope, &format!("turbo.shadow.{name}")) else {
                return;
            };
            let private = v8::Private::for_api(scope, Some(name));
            keys.push(v8::Global::new(scope, private));
        }
        scope.set_slot(std::rc::Rc::new(Keys(keys)));
    }
}
pub(super) fn install(scope: &mut v8::PinScope) {
    ensure_keys(scope);
    let global = scope.get_current_context().global(scope);
    let Some(name) = v8::String::new(scope, "Event") else {
        return;
    };
    if let Some(event) = global.get(scope, name.into()) {
        let private = key(scope, "Event");
        global.set_private(scope, private, event);
    }
    bind_method(scope, global, "__shadowSlotNative", native);
    run_js(scope, include_str!("shadow_slots.js"));
}
