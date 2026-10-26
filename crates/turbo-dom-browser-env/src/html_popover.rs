//! Private native popover state and trusted, task-queued lifecycle events.
use super::*;

#[derive(Clone)]
struct Entry {
    node: Handle,
    mode: String,
}
#[derive(Clone)]
struct Pending {
    target: v8::Weak<v8::Object>,
    record: v8::Weak<v8::Object>,
    old: bool,
    new: bool,
    source: Option<v8::Weak<v8::Object>>,
    task: u64,
}
#[derive(Default)]
pub(super) struct State {
    active: bool,
    open: Vec<Entry>,
    pending: HashMap<Handle, Pending>,
    busy: std::collections::HashSet<Handle>,
    constructor: Option<v8::Weak<v8::Function>>,
    scheduler: Option<v8::Weak<v8::Function>>,
    next_task: u64,
}
fn mode(tree: &Tree, node: Handle) -> Option<String> {
    let value = tree
        .get_attribute_ns(node, None, "popover")?
        .to_ascii_lowercase();
    Some(
        match value.as_str() {
            "" | "auto" => "auto",
            "hint" => "hint",
            _ => "manual",
        }
        .into(),
    )
}
pub(super) fn opened(node: Handle) -> bool {
    DOM.with(|dom| {
        dom.borrow()
            .as_ref()
            .is_some_and(|dom| dom.popover.open.iter().any(|entry| entry.node == node))
    })
}
fn descendant_light(tree: &Tree, mut node: Handle, ancestor: Handle) -> bool {
    loop {
        if node == ancestor {
            return true;
        }
        let Some(parent) = tree.parent(node) else {
            return false;
        };
        node = parent;
    }
}
fn descendant(tree: &Tree, node: Handle, ancestor: Handle) -> bool {
    let mut current = Some(node);
    while let Some(node) = current {
        if node == ancestor {
            return true;
        }
        current = tree.parent(node).or_else(|| tree.shadow_host(node));
    }
    false
}
fn event<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    node: Handle,
    kind: &str,
    old: bool,
    new: bool,
    source: Option<v8::Local<'s, v8::Object>>,
) -> Option<bool> {
    let target = wrap(scope, node);
    event_target(scope, target, kind, old, new, source)
}
fn event_target<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    target: v8::Local<'s, v8::Object>,
    kind: &str,
    old: bool,
    new: bool,
    source: Option<v8::Local<'s, v8::Object>>,
) -> Option<bool> {
    let constructor = DOM.with(|dom| dom.borrow().as_ref()?.popover.constructor.clone())?;
    let constructor = dom_ownership::local(scope, &constructor);
    let init = v8::Object::new(scope);
    let old_value = v8::String::new(scope, if old { "open" } else { "closed" }).unwrap();
    let new_value = v8::String::new(scope, if new { "open" } else { "closed" }).unwrap();
    let cancelable = v8::Boolean::new(scope, kind == "beforetoggle" && new);
    set_prop(scope, init, "oldState", old_value.into());
    set_prop(scope, init, "newState", new_value.into());
    set_prop(scope, init, "cancelable", cancelable.into());
    // Chrome composes toggle events exactly when they carry a source.
    let composed = v8::Boolean::new(scope, source.is_some());
    set_prop(scope, init, "composed", composed.into());
    if let Some(source) = source {
        set_prop(scope, init, "source", source.into());
    }
    let kind = v8::String::new(scope, kind).unwrap();
    let event = constructor.new_instance(scope, &[kind.into(), init.into()])?;
    dispatch_host_event(scope, event, target);
    event_record(scope, event).map(|record| !get_bool_prop(scope, record, "defaultPrevented"))
}
fn flush(scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue) {
    let Ok(data) = v8::Local::<v8::Array>::try_from(args.data()) else {
        return;
    };
    let owner = data
        .get_index(scope, 0)
        .and_then(|value| value.integer_value(scope))
        .unwrap_or(0) as u64;
    let node = Handle::from_raw(
        data.get_index(scope, 1)
            .and_then(|value| value.uint32_value(scope))
            .unwrap_or(0),
    );
    let task = data
        .get_index(scope, 2)
        .and_then(|value| value.integer_value(scope))
        .unwrap_or(0) as u64;
    let _dom = DomScope::enter(owner);
    let pending = DOM.with(|dom| {
        let mut dom = dom.borrow_mut();
        let state = &mut dom.as_mut()?.popover;
        if state.pending.get(&node)?.task != task {
            return None;
        }
        state.pending.remove(&node)
    });
    if let Some(pending) = pending {
        let source = pending
            .source
            .as_ref()
            .map(|source| dom_ownership::local(scope, source));
        let Some(target) = pending.target.to_local(scope) else {
            return;
        };
        event_target(scope, target, "toggle", pending.old, pending.new, source);
    }
}
fn queue<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    node: Handle,
    old: bool,
    new: bool,
    source: Option<v8::Local<'s, v8::Object>>,
) {
    let target = wrap(scope, node);
    let old_record = DOM.with(|dom| {
        dom.borrow()
            .as_ref()
            .and_then(|dom| dom.popover.pending.get(&node))
            .and_then(|pending| pending.record.to_local(scope))
    });
    let record = old_record.unwrap_or_else(|| {
        let record = v8::Object::new(scope);
        let null = v8::null(scope);
        record.set_prototype(scope, null.into());
        record
    });
    let key = v8::String::new(scope, "target").unwrap();
    record.create_data_property(scope, key.into(), target.into());
    let key = v8::String::new(scope, "source").unwrap();
    let source_value = source
        .map(|source| source.into())
        .unwrap_or_else(|| v8::undefined(scope).into());
    record.create_data_property(scope, key.into(), source_value);
    let source = source.map(|source| v8::Weak::new(scope, source));
    let target = v8::Weak::new(scope, target);
    let scheduled = DOM.with(|dom| {
        let mut dom = dom.borrow_mut();
        let state = &mut dom.as_mut()?.popover;
        if let Some(pending) = state.pending.get_mut(&node) {
            pending.new = new;
            pending.source = source;
            return None;
        }
        state.next_task += 1;
        let task = state.next_task;
        state.pending.insert(
            node,
            Pending {
                target,
                record: v8::Weak::new(scope, record),
                old,
                new,
                source,
                task,
            },
        );
        state.scheduler.clone().map(|scheduler| (scheduler, task))
    });
    let Some((scheduler, task)) = scheduled else {
        return;
    };
    let owner = v8::Number::new(scope, DOM_ID.with(|id| id.get()) as f64);
    let handle = v8::Integer::new_from_unsigned(scope, node.raw());
    let task = v8::Number::new(scope, task as f64);
    let data = v8::Array::new_with_elements(
        scope,
        &[owner.into(), handle.into(), task.into(), record.into()],
    );
    let Some(function) = v8::FunctionTemplate::builder(flush)
        .data(data.into())
        .constructor_behavior(v8::ConstructorBehavior::Throw)
        .build(scope)
        .get_function(scope)
    else {
        return;
    };
    let scheduler = dom_ownership::local(scope, &scheduler);
    let receiver = v8::undefined(scope);
    let delay = v8::Integer::new(scope, 0);
    scheduler.call(scope, receiver.into(), &[function.into(), delay.into()]);
}
fn check(scope: &mut v8::PinScope, node: Handle, showing: bool) -> Option<String> {
    let Some(mode) = with_tree(|tree| mode(tree, node)).flatten() else {
        throw_dom_error(
            scope,
            "NotSupportedError",
            "The element does not have a popover attribute",
        );
        return None;
    };
    let active = DOM.with(|dom| dom.borrow().as_ref().is_some_and(|dom| dom.popover.active));
    if showing && (!active || !with_tree(|tree| is_connected(tree, node)).unwrap_or(false)) {
        throw_dom_error(
            scope,
            "InvalidStateError",
            "The popover is not connected to a document",
        );
        return None;
    }
    Some(mode)
}
fn hide(scope: &mut v8::PinScope, node: Handle, events: bool) {
    hide_from(scope, node, events, None);
}
fn hide_from<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    node: Handle,
    events: bool,
    source: Option<v8::Local<'s, v8::Object>>,
) {
    if !opened(node) {
        return;
    }
    let busy = DOM.with(|dom| dom.borrow_mut().as_mut().unwrap().popover.busy.insert(node));
    if events && event(scope, node, "beforetoggle", true, false, source).is_none() {
        if busy {
            DOM.with(|dom| {
                dom.borrow_mut()
                    .as_mut()
                    .unwrap()
                    .popover
                    .busy
                    .remove(&node);
            });
        }
        return;
    }
    DOM.with(|dom| {
        dom.borrow_mut()
            .as_mut()
            .unwrap()
            .popover
            .open
            .retain(|entry| entry.node != node)
    });
    user_activation_gates::popover_hidden(node);
    if events {
        queue(scope, node, true, false, source);
    }
    if busy {
        DOM.with(|dom| {
            dom.borrow_mut()
                .as_mut()
                .unwrap()
                .popover
                .busy
                .remove(&node);
        });
    }
}
fn show<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    node: Handle,
    source: Option<v8::Local<'s, v8::Object>>,
) {
    if check(scope, node, true).is_none() {
        return;
    }
    if opened(node) {
        return;
    }
    if event(scope, node, "beforetoggle", false, true, source) != Some(true) {
        return;
    }
    let Some(mode) = check(scope, node, true) else {
        return;
    };
    if opened(node) {
        return;
    }
    if mode != "manual" {
        let source_handle = source.and_then(|source| {
            if dom_id_of(scope, source) == DOM_ID.with(|id| id.get()) {
                handle_of(scope, source)
            } else {
                None
            }
        });
        let closing = DOM.with(|dom| {
            let dom = dom.borrow();
            let dom = dom.as_ref().unwrap();
            dom.popover
                .open
                .iter()
                .rev()
                .filter(|entry| {
                    entry.mode != "manual"
                        && (mode == "auto" || entry.mode == "hint")
                        && !descendant(&dom.tree, node, entry.node)
                        && !source_handle
                            .is_some_and(|source| descendant(&dom.tree, source, entry.node))
                })
                .map(|entry| entry.node)
                .collect::<Vec<_>>()
        });
        for entry in closing {
            hide(scope, entry, true);
        }
    }
    if check(scope, node, true).is_none() {
        return;
    }
    DOM.with(|dom| {
        dom.borrow_mut().as_mut().unwrap().popover.open.push(Entry {
            node,
            mode: mode.clone(),
        })
    });
    if mode != "manual" {
        user_activation_gates::popover_shown(node);
    }
    queue(scope, node, false, true, source);
}
/// Invoker activation: the popover algorithms with an invoker source and exceptions
/// suppressed. `action` is the canonical toggle/show/hide keyword.
pub(super) fn invoke<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    node: Handle,
    action: &str,
    source: v8::Local<'s, v8::Object>,
) {
    if with_tree(|tree| mode(tree, node)).flatten().is_none() {
        return;
    }
    let open = opened(node);
    if (action == "show" && open) || (action == "hide" && !open) {
        return;
    }
    let busy = DOM.with(|dom| dom.borrow_mut().as_mut().unwrap().popover.busy.insert(node));
    if !busy {
        return;
    }
    {
        v8::tc_scope!(let scope, scope);
        if open {
            hide_from(scope, node, true, Some(source));
        } else {
            show(scope, node, Some(source));
        }
        if scope.has_caught() && !scope.has_terminated() {
            scope.reset();
        }
    }
    DOM.with(|dom| {
        if let Some(dom) = dom.borrow_mut().as_mut() {
            dom.popover.busy.remove(&node);
        }
    });
}
pub(super) fn disconnect(scope: &mut v8::PinScope, root: Handle, include_root: bool) {
    html_dialog::disconnect(root, include_root);
    pointer_events::disconnect(scope, root, include_root);
    interest_invokers::disconnect(scope, root, include_root);
    user_activation_gates::disconnect(scope, root, include_root);
    let nodes = DOM.with(|dom| {
        let dom = dom.borrow();
        let Some(dom) = dom.as_ref() else {
            return Vec::new();
        };
        dom.popover
            .open
            .iter()
            .filter(|entry| {
                (include_root || entry.node != root)
                    && if include_root {
                        descendant(&dom.tree, entry.node, root)
                    } else {
                        descendant_light(&dom.tree, entry.node, root)
                    }
            })
            .map(|entry| entry.node)
            .collect::<Vec<_>>()
    });
    for node in nodes {
        hide(scope, node, false);
    }
}
pub(super) fn attribute_node_change(scope: &mut v8::PinScope, node: Handle, value: Option<&str>) {
    let owner = with_tree(|tree| {
        tree.attribute_data(node)
            .filter(|data| data.namespace.is_none() && data.local_name == "popover")
            .and_then(|_| tree.attribute_owner(node))
    })
    .flatten();
    if let Some(owner) = owner {
        attribute_change(scope, owner, value);
    }
}
pub(super) fn attribute_change(scope: &mut v8::PinScope, node: Handle, value: Option<&str>) {
    let old = with_tree(|tree| mode(tree, node)).flatten();
    let new = value.map(|value| match value.to_ascii_lowercase().as_str() {
        "" | "auto" => "auto",
        "hint" => "hint",
        _ => "manual",
    });
    let busy = DOM.with(|dom| {
        dom.borrow()
            .as_ref()
            .is_some_and(|dom| dom.popover.busy.contains(&node))
    });
    if old.as_deref() != new && opened(node) && !busy {
        hide(scope, node, true);
    }
}
pub(super) fn dispatch<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    let Ok(receiver) = v8::Local::<v8::Object>::try_from(args.get(0)) else {
        return;
    };
    let _dom = DomScope::for_object(scope, receiver);
    let Some(node) = handle_of(scope, receiver) else {
        return;
    };
    if with_tree(|tree| tree.node_type_id(node) != 1 || tree.namespace_id(node) != 0)
        .unwrap_or(true)
    {
        return;
    }
    let operation = args.get(1).to_rust_string_lossy(scope);
    if operation == "read" {
        if let Some(mode) = with_tree(|tree| mode(tree, node)).flatten() {
            rv.set(v8::String::new(scope, &mode).unwrap().into());
        } else {
            rv.set(v8::null(scope).into());
        }
        return;
    }
    let busy = DOM.with(|dom| dom.borrow_mut().as_mut().unwrap().popover.busy.insert(node));
    if !busy {
        rv.set_bool(opened(node));
        return;
    }
    let showing = match operation.as_str() {
        "show" => true,
        "hide" => false,
        _ => {
            if args.get(2).is_undefined() {
                !opened(node)
            } else {
                args.get(2).boolean_value(scope)
            }
        }
    };
    if check(scope, node, showing && !opened(node)).is_some() {
        let source = v8::Local::<v8::Object>::try_from(args.get(3)).ok();
        if showing {
            show(scope, node, source);
        } else {
            hide(scope, node, true);
        }
    }
    DOM.with(|dom| {
        dom.borrow_mut()
            .as_mut()
            .unwrap()
            .popover
            .busy
            .remove(&node);
    });
    if operation == "toggle" {
        rv.set_bool(opened(node));
    }
}
pub(super) fn install(scope: &mut v8::PinScope) {
    let global = scope.get_current_context().global(scope);
    bind_method(scope, global, "__popoverNative", dispatch);
    run_js(scope, include_str!("html_popover.js"));
    let global = scope.get_current_context().global(scope);
    let key = v8::String::new(scope, "ToggleEvent").unwrap();
    if let Some(constructor) = global
        .get(scope, key.into())
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
    {
        let Some(constructor) = dom_ownership::helper_function(scope, constructor) else {
            return;
        };
        DOM.with(|dom| {
            let mut dom = dom.borrow_mut();
            let state = &mut dom.as_mut().unwrap().popover;
            state.constructor = Some(constructor);
            state.active = true;
        });
    }
    capture_scheduler(scope);
}
/// Dialog show steps: hide every open auto/hint popover that is not a flat-tree
/// ancestor of `node`, topmost first, firing events.
pub(super) fn hide_unrelated(scope: &mut v8::PinScope, node: Handle) {
    let closing = DOM.with(|dom| {
        let dom = dom.borrow();
        let dom = dom.as_ref().unwrap();
        dom.popover
            .open
            .iter()
            .rev()
            .filter(|entry| entry.mode != "manual" && !descendant(&dom.tree, node, entry.node))
            .map(|entry| entry.node)
            .collect::<Vec<_>>()
    });
    for entry in closing {
        hide(scope, entry, true);
    }
}
/// A close watcher's close action: hide the popover, firing events.
pub(super) fn hide_popover(scope: &mut v8::PinScope, node: Handle) {
    hide(scope, node, true);
}
/// Whether `node` is a showing auto or hint popover (light dismiss considers no others).
pub(super) fn auto_showing(node: Handle) -> bool {
    DOM.with(|dom| {
        dom.borrow().as_ref().is_some_and(|dom| {
            dom.popover
                .open
                .iter()
                .any(|entry| entry.node == node && entry.mode != "manual")
        })
    })
}
pub(super) fn any_auto_showing() -> bool {
    DOM.with(|dom| {
        dom.borrow()
            .as_ref()
            .is_some_and(|dom| dom.popover.open.iter().any(|entry| entry.mode != "manual"))
    })
}
/// The later-shown (higher in the popover stack) of two showing popovers.
pub(super) fn topmost_of(first: Handle, second: Handle) -> Handle {
    DOM.with(|dom| {
        let dom = dom.borrow();
        let position = |node: Handle| {
            dom.as_ref()
                .and_then(|dom| dom.popover.open.iter().position(|entry| entry.node == node))
        };
        if position(first) >= position(second) {
            first
        } else {
            second
        }
    })
}
/// https://html.spec.whatwg.org/#hide-all-popovers-until: every auto/hint popover above
/// `endpoint` (all of them for the document), topmost first, firing events.
pub(super) fn hide_until(scope: &mut v8::PinScope, endpoint: Option<Handle>) {
    let closing = DOM.with(|dom| {
        let dom = dom.borrow();
        let Some(dom) = dom.as_ref() else {
            return Vec::new();
        };
        let open = &dom.popover.open;
        let start = endpoint
            .and_then(|endpoint| open.iter().position(|entry| entry.node == endpoint))
            .map_or(0, |index| index + 1);
        open[start..]
            .iter()
            .rev()
            .filter(|entry| entry.mode != "manual")
            .map(|entry| entry.node)
            .collect::<Vec<_>>()
    });
    for node in closing {
        hide(scope, node, true);
    }
}
/// Queue `callback` as a task on this document's private, host-captured scheduler.
pub(super) fn schedule(scope: &mut v8::PinScope, callback: v8::Local<v8::Function>) {
    schedule_after(scope, callback, 0);
}
/// Queue `callback` as a task `delay` milliseconds out on the same trusted scheduler.
/// Returns false when the host never captured one.
pub(super) fn schedule_after(
    scope: &mut v8::PinScope,
    callback: v8::Local<v8::Function>,
    delay: u32,
) -> bool {
    let Some(scheduler) = DOM.with(|dom| dom.borrow().as_ref()?.popover.scheduler.clone()) else {
        return false;
    };
    let scheduler = dom_ownership::local(scope, &scheduler);
    let receiver = v8::undefined(scope);
    let delay = v8::Integer::new_from_unsigned(scope, delay);
    scheduler.call(scope, receiver.into(), &[callback.into(), delay.into()]);
    true
}
pub(super) fn capture_scheduler(scope: &mut v8::PinScope) {
    // Independent-context installation restores its caller's TLS document. The
    // host installs timers afterwards, so resolve ownership from the realm here.
    let owner = scope
        .get_current_context()
        .get_slot::<DomContextId>()
        .map_or(0, |id| id.0);
    let _dom = DomScope::enter(owner);
    let global = scope.get_current_context().global(scope);
    let key = v8::String::new(scope, "setTimeout").unwrap();
    if let Some(scheduler) = global
        .get(scope, key.into())
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
    {
        let Some(scheduler) = dom_ownership::helper_function(scope, scheduler) else {
            return;
        };
        DOM.with(|dom| dom.borrow_mut().as_mut().unwrap().popover.scheduler = Some(scheduler));
    }
}

pub(super) fn selector_root(tree: &Tree, mut node: Handle) -> Handle {
    while let Some(parent) = tree.parent(node) {
        node = parent;
    }
    node
}
pub(super) fn selector_path(tree: &Tree, mut node: Handle) -> String {
    let mut path = Vec::new();
    loop {
        let parent = tree.parent(node);
        let index = parent
            .map(|parent| {
                tree.children(parent)
                    .into_iter()
                    .filter(|&child| tree.node_type_id(child) == 1)
                    .position(|child| child == node)
                    .unwrap_or(0)
                    + 1
            })
            .unwrap_or(1);
        path.push(format!(":nth-child({index})"));
        let Some(parent) = parent.filter(|&parent| tree.node_type_id(parent) == 1) else {
            break;
        };
        node = parent;
    }
    path.reverse();
    format!(":root{}", path.join(" > "))
}
fn pseudo_ranges<'i, 't>(
    input: &mut cssparser::Parser<'i, 't>,
    ranges: &mut Vec<(usize, usize)>,
) -> Result<(), cssparser::ParseError<'i, ()>> {
    use cssparser::Token;
    let mut colon = None;
    let mut double_colon = false;
    while !input.is_exhausted() {
        let position = input.position().byte_index();
        let token = input.next_including_whitespace_and_comments()?.clone();
        match token {
            Token::Colon => {
                double_colon = colon.is_some();
                colon = if double_colon { None } else { Some(position) };
            }
            Token::Ident(name) => {
                if !double_colon && name.eq_ignore_ascii_case("popover-open") {
                    if let Some(start) = colon {
                        ranges.push((start, input.position().byte_index()));
                    }
                }
                colon = None;
                double_colon = false;
            }
            Token::Function(ref name)
                if name.eq_ignore_ascii_case("popover-open") && colon.is_some() =>
            {
                ranges.push((colon.unwrap(), input.position().byte_index()));
                return Err(input.new_custom_error(()));
            }
            Token::Function(_) | Token::ParenthesisBlock => {
                colon = None;
                double_colon = false;
                input.parse_nested_block(|nested| pseudo_ranges(nested, ranges))?;
            }
            Token::Comment(_) => (),
            _ => {
                colon = None;
                double_colon = false;
            }
        }
    }
    Ok(())
}
/// Represent private native state as structural selectors within this exact DOM
/// root. No attributes, styles or nodes are inserted into the guest's tree.
pub(super) fn selector(scope: &mut v8::PinScope, anchor: Handle, source: &str) -> Option<String> {
    let mut input = cssparser::ParserInput::new(source);
    let mut parser = cssparser::Parser::new(&mut input);
    let mut ranges = Vec::new();
    let parsed = pseudo_ranges(&mut parser, &mut ranges);
    if ranges.is_empty() {
        return Some(source.to_owned());
    }
    if parsed.is_err() || !cssom_parser::valid_selector(source) {
        throw_dom_error(scope, "SyntaxError", "Invalid selector");
        return None;
    }
    let replacement = DOM.with(|dom| {
        let dom = dom.borrow();
        let dom = dom.as_ref().unwrap();
        let root = selector_root(&dom.tree, anchor);
        let paths = dom
            .popover
            .open
            .iter()
            .filter(|entry| selector_root(&dom.tree, entry.node) == root)
            .map(|entry| selector_path(&dom.tree, entry.node))
            .collect::<Vec<_>>();
        if paths.is_empty() {
            ":not(*)".to_owned()
        } else {
            format!(":is({})", paths.join(","))
        }
    });
    let mut result = String::new();
    let mut start = 0;
    for (from, to) in ranges {
        result.push_str(&source[start..from]);
        result.push_str(&replacement);
        start = to;
    }
    result.push_str(&source[start..]);
    Some(result)
}

impl State {
    pub(super) fn sweep_storage(&mut self, tree: &Tree) {
        self.open.retain(|entry| tree.is_live(entry.node));
        self.pending.retain(|node, _| tree.is_live(*node));
        self.busy.retain(|node| tree.is_live(*node));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn post_install_schedulers_follow_their_native_context_document() {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let parent = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, parent);
        install_with_html(scope, "<!doctype html><body><div id=popup popover=manual>");
        let timers = "globalThis.tasks=[];globalThis.events=[];globalThis.setTimeout=callback=>{tasks.push(callback);return tasks.length};document.body.replaceChildren();const popup=document.createElement('div');popup.id='popup';popup.popover='manual';document.body.append(popup);popup.ontoggle=event=>events.push([event.oldState,event.newState,event.isTrusted]);";
        run_js(scope, timers);
        capture_popover_scheduler(scope);
        let child = v8::Context::new(scope, Default::default());
        child.set_security_token(parent.get_security_token(scope));
        {
            let scope = &mut v8::ContextScope::new(scope, child);
            install_independent_html(scope, "<!doctype html><body><div id=popup popover=manual>");
            run_js(scope, timers);
            capture_popover_scheduler(scope);
            run_js(scope, "globalThis.setTimeout=()=>{throw new Error('guest timer replacement')};document.getElementById('popup').showPopover();");
        }
        let global = parent.global(scope);
        let scope = &mut v8::ContextScope::new(scope, parent);
        let key = v8::String::new(scope, "child").unwrap();
        let child_global = child.global(scope);
        global.set(scope, key.into(), child_global.into());
        let parsed = parse_html_document(scope, "<!doctype html><body>", false);
        let key = v8::String::new(scope, "inert").unwrap();
        global.set(scope, key.into(), parsed.into());
        let source = v8::String::new(scope, r#"(()=>{
          setTimeout=()=>{throw new Error('guest timer replacement')};document.getElementById('popup').showPopover();
          if(tasks.length!==1||child.tasks.length!==1)throw new Error('scheduler crossed document ownership');
          tasks.shift()();child.tasks.shift()();
          const inactive=inert.adoptNode(document.createElement('div'));inactive.popover='manual';inert.body.append(inactive);
          let error;try{inactive.showPopover()}catch(caught){error=caught.name}
          return JSON.stringify([events,child.events,error,document.querySelector(':popover-open').id,child.document.querySelector(':popover-open').id]);
        })()"#).unwrap();
        let result = v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap();
        assert_eq!(
            result.to_rust_string_lossy(scope),
            r#"[[["closed","open",true]],[["closed","open",true]],"InvalidStateError","popup","popup"]"#
        );
        reset();
    }
    fn fixture(source: &str) {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(
            scope,
            "<!doctype html><html><head></head><body></body></html>",
        );
        run_js(scope,"globalThis.tasks=[];globalThis.setTimeout=callback=>{tasks.push(callback);return tasks.length};globalThis.drain=()=>{let limit=100;while(tasks.length&&limit--)tasks.shift()();if(tasks.length)throw new Error('non-quiescent lifecycle queue')};");
        capture_popover_scheduler(scope);
        let target = parse_html_document(
            scope,
            "<!doctype html><html><head></head><body></body></html>",
            false,
        );
        let global = scope.get_current_context().global(scope);
        let key = v8::String::new(scope, "targetDocument").unwrap();
        global.set(scope, key.into(), target.into());

        let scope = std::pin::pin!(v8::TryCatch::new(scope));
        let scope = &mut scope.init();
        let source = v8::String::new(scope, source).unwrap();
        let result = v8::Script::compile(scope, source, None).and_then(|script| script.run(scope));
        if result.is_none() {
            panic!(
                "popover fixture: {:?}",
                scope
                    .exception()
                    .map(|error| error.to_rust_string_lossy(scope))
            );
        }
        assert!(result.unwrap().is_true());
        reset();
    }
    #[test]
    fn native_popover_transitions_are_trusted_cancelable_and_task_coalesced() {
        fixture(
            r#"(()=>{
          const element=document.createElement('div');element.popover='manual';document.body.append(element);
          const rows=[];for(const type of ['beforetoggle','toggle'])element.addEventListener(type,event=>rows.push([type,event.oldState,event.newState,event.cancelable,event.isTrusted,event instanceof ToggleEvent,element.matches(':popover-open')]));
          if(element.showPopover()!==undefined||!element.matches(':popover-open')||element.togglePopover(true)!==true)throw new Error('show state');
          element.hidePopover();element.togglePopover();element.togglePopover();
          if(rows.length!==4||tasks.length!==1)throw new Error('queued event coalescing');drain();
          if(JSON.stringify(rows)!=='[["beforetoggle","closed","open",true,true,true,false],["beforetoggle","open","closed",false,true,true,true],["beforetoggle","closed","open",true,true,true,false],["beforetoggle","open","closed",false,true,true,true],["toggle","closed","closed",false,true,true,false]]')throw new Error('trusted lifecycle '+JSON.stringify(rows));
          const canceled=document.createElement('div');canceled.popover='manual';document.body.append(canceled);canceled.onbeforetoggle=event=>event.preventDefault();
          if(canceled.togglePopover()!==false||canceled.matches(':popover-open'))throw new Error('canceled opening');
          return true;
        })()"#,
        );
    }
    #[test]
    fn auto_and_hint_stacks_keep_native_ancestors_and_silent_removal() {
        fixture(
            r#"(()=>{
          const make=mode=>{const element=document.createElement('div');element.popover=mode;document.body.append(element);return element;};
          const a=make('auto'),b=make('auto'),nested=make('auto'),hint=make('hint'),hint2=make('hint'),manual=make('manual');a.append(nested);
          a.showPopover();nested.showPopover();if(!a.matches(':popover-open')||!nested.matches(':popover-open'))throw new Error('nested auto');
          hint.showPopover();hint2.showPopover();manual.showPopover();
          if(hint.matches(':popover-open')||!a.matches(':popover-open')||!nested.matches(':popover-open')||!hint2.matches(':popover-open'))throw new Error('hint stack');
          b.showPopover();if(a.matches(':popover-open')||nested.matches(':popover-open')||hint2.matches(':popover-open')||!manual.matches(':popover-open')||!b.matches(':popover-open'))throw new Error('auto stack');
          drain();const rows=[];manual.onbeforetoggle=event=>rows.push(event.type);manual.ontoggle=event=>rows.push(event.type);manual.remove();drain();
          if(manual.matches(':popover-open')||rows.length)throw new Error('silent removal');
          return true;
        })()"#,
        );
    }
    #[test]
    fn popover_selector_state_uses_private_live_native_structure_without_dom_mutation() {
        fixture(
            r#"(()=>{
          const element=document.createElement('div');element.id='probe-open';element.popover='manual';document.body.append(element);const before=element.outerHTML;element.showPopover();
          if(!element.matches('div:popover-open')||document.querySelector('#probe-open:popover-open')!==element||document.querySelectorAll(':not(:popover-open)#probe-open').length||!document.body.matches(':has(> #probe-open:popover-open)'))throw new Error('selector composition');
          if(element.outerHTML!==before||element.matches('[title=":popover-open"]'))throw new Error('observable selector state');
          let error;try{element.matches(':popover-open(')}catch(caught){error=caught.name;}if(error!=='SyntaxError')throw new Error('invalid nested selector '+error);
          element.hidePopover();if(element.matches(':popover-open'))throw new Error('closed selector state');
          return true;
        })()"#,
        );
    }
    #[test]
    fn attribute_mutation_and_beforetoggle_reentrancy_recheck_native_validity() {
        fixture(
            r#"(()=>{
          const element=document.createElement('div');element.popover='manual';document.body.append(element);element.showPopover();drain();
          const rows=[];element.onbeforetoggle=event=>rows.push([event.oldState,event.newState,element.popover,element.matches(':popover-open')]);element.ontoggle=event=>rows.push([event.oldState,event.newState]);element.removeAttribute('popover');drain();
          if(JSON.stringify(rows)!=='[["open","closed","manual",true],["open","closed"]]'||element.matches(':popover-open'))throw new Error('attribute-close ordering '+JSON.stringify(rows));
          let error;try{element.hidePopover()}catch(caught){error=caught.name;}if(error!=='NotSupportedError')throw new Error('missing attribute');
          element.popover='manual';element.onbeforetoggle=()=>element.removeAttribute('popover');try{element.showPopover()}catch(caught){error=caught.name;}if(error!=='NotSupportedError'||element.matches(':popover-open'))throw new Error('reentrant attribute validity');
          element.popover='manual';element.onbeforetoggle=()=>element.remove();try{element.showPopover()}catch(caught){error=caught.name;}if(error!=='InvalidStateError'||element.matches(':popover-open'))throw new Error('reentrant connectivity');
          return true;
        })()"#,
        );
    }
    #[test]
    fn private_scheduler_sources_and_move_before_preserve_measured_contracts() {
        fixture(
            r#"(()=>{
          const source=document.createElement('button');source.id='source';document.body.append(source);
          const element=document.createElement('div');element.popover='manual';document.body.append(element);
          let reads=0;const rows=[];for(const name of ['beforetoggle','toggle'])element.addEventListener(name,event=>rows.push([name,event.oldState,event.newState,event.source===source,event.isTrusted]));
          const original=ToggleEvent;globalThis.ToggleEvent=function(){throw new Error('guest constructor')};globalThis.setTimeout=()=>{throw new Error('guest scheduler')};
          element.showPopover({get source(){reads++;return source;}});
          if(reads!==1||!element.matches(':popover-open'))throw new Error('source conversion');
          element.hidePopover({source});drain();globalThis.ToggleEvent=original;
          if(JSON.stringify(rows)!=='[["beforetoggle","closed","open",true,true],["beforetoggle","open","closed",false,true],["toggle","closed","closed",false,true]]')throw new Error('private event/scheduler '+JSON.stringify(rows));
          for(const value of [42,'x',true]){let error;try{element.showPopover(value)}catch(caught){error=caught.name;}if(error!=='TypeError')throw new Error('dictionary conversion');}
          element.showPopover(null);document.body.moveBefore(element,null);if(!element.matches(':popover-open'))throw new Error('moveBefore closed popover');
          document.body.insertBefore(element,element);if(element.matches(':popover-open'))throw new Error('ordinary self insertion preserved popover');
          element.showPopover();element.remove();drain();
          if(rows[rows.length-1][0]!=='toggle'||rows[rows.length-1][2]!=='open')throw new Error('queued toggle lost on removal');
          return true;
        })()"#,
        );
    }
    #[test]
    fn compound_selectors_track_shadow_roots_moves_and_escaped_tokens_without_markers() {
        fixture(
            r#"(()=>{
          const host=document.createElement('section'),shadow=host.attachShadow({mode:'open'}),popup=document.createElement('div'),light=document.createElement('div');popup.popover='manual';light.popover='manual';shadow.append(popup);host.append(light);document.body.append(host);popup.showPopover();
          if(!popup.matches(':popover-open')||shadow.querySelector(':popover-open')!==popup||document.querySelector(':popover-open')!==null||light.matches(':popover-open'))throw new Error('shadow state escaped DOM root');
          host.innerHTML='<div popover="manual"></div>';if(!popup.matches(':popover-open'))throw new Error('light replacement closed shadow popup');
          document.body.prepend(document.createElement('aside'));if(!popup.matches('div:is(:popover-open)')||!popup.matches('div:popover-\\6f pen')||popup.matches('[data-test=":popover-open"]'))throw new Error('moving/escaped selector state');
          const fragment=document.createDocumentFragment(),detached=document.createElement('div');detached.popover='manual';fragment.append(detached);if(detached.matches(':popover-open')||fragment.querySelector(':popover-open')!==null)throw new Error('detached fragment state');
          host.remove();if(popup.matches(':popover-open'))throw new Error('shadow disconnect');
          return true;
        })()"#,
        );
    }
    #[test]
    fn native_attr_aliases_namespaces_and_adoption_close_popovers_before_attribute_changes() {
        fixture(
            r#"(()=>{
          const make=()=>{const e=document.createElement('div');e.popover='auto';document.body.append(e);e.showPopover();drain();return e;};
          const seams=[e=>{e.getAttributeNode('popover').value='manual'},e=>{e.getAttributeNode('popover').nodeValue='manual'},e=>{e.getAttributeNode('popover').textContent='manual'},e=>e.setAttributeNS(null,'popover','manual'),e=>e.removeAttributeNS(null,'popover'),e=>e.toggleAttribute('popover',false),e=>{const a=document.createAttribute('popover');a.value='manual';e.setAttributeNode(a)},e=>e.removeAttributeNode(e.getAttributeNode('popover')),e=>document.adoptNode(e.getAttributeNode('popover'))];
          for(let index=0;index<seams.length;index++){const e=make();let count=0,oldValid=true;e.onbeforetoggle=event=>{if(event.oldState==='open'){count++;oldValid=e.popover==='auto'&&e.matches(':popover-open')}};seams[index](e);if(e.matches(':popover-open')||count!==1||!oldValid)throw new Error('attribute seam '+index);e.remove();drain();}
          const namespaced=document.createElement('div');namespaced.setAttributeNS('urn:foreign','popover','auto');document.body.append(namespaced);let error;try{namespaced.showPopover()}catch(caught){error=caught.name;}if(namespaced.popover!==null||error!=='NotSupportedError')throw new Error('foreign namespace enabled popover');namespaced.popover=null;if(namespaced.getAttributeNS('urn:foreign','popover')!=='auto')throw new Error('reflection removed foreign attribute');namespaced.popover='manual';if(namespaced.popover!=='manual'||namespaced.getAttributeNS('urn:foreign','popover')!=='auto')throw new Error('reflection changed namespace');namespaced.remove();
          const element=make();document.adoptNode(element);if(element.matches(':popover-open')||element.parentNode!==null)throw new Error('same-document adoption did not disconnect');
          return true;
        })()"#,
        );
    }
    #[test]
    fn inert_documents_and_adopted_pending_events_keep_native_ownership_and_identity() {
        fixture(
            r#"(()=>{
          const inactive=targetDocument.adoptNode(document.createElement('div'));inactive.popover='manual';targetDocument.body.append(inactive);let error;try{inactive.showPopover()}catch(caught){error=caught.name;}if(error!=='InvalidStateError'||inactive.matches(':popover-open'))throw new Error('inert document opened popover');
          const element=document.createElement('div');element.popover='auto';document.body.append(element);element.showPopover();drain();
          const attribute=element.getAttributeNode('popover');element.onbeforetoggle=()=>{attribute.value='manual'};
          targetDocument.adoptNode(attribute);if(attribute.ownerDocument!==targetDocument||attribute.value!=='manual'||attribute.ownerElement!==null||element.matches(':popover-open'))throw new Error('attribute reentrant adoption snapshot');
          const moved=document.createElement('div');moved.popover='manual';document.body.append(moved);let target; moved.ontoggle=event=>{target=event.target};moved.showPopover();targetDocument.adoptNode(moved);drain();
          if(target!==moved||moved.ownerDocument!==targetDocument||moved.matches(':popover-open'))throw new Error('queued event lost adopted target identity');
          return true;
        })()"#,
        );
    }
}
