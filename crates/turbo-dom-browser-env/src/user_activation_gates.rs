//! Activation-gated behavior over the embedder's user activation: `showPicker()`, the
//! fullscreen API (a genuine per-document fullscreen stack in the top layer), the details
//! toggle event summary activation relies on, and the close watcher manager behind Esc
//! close requests and `closedby=any`/popover light dismiss.
//!
//! Activation itself belongs to the embedder: only it delivers trusted input. Without a
//! host (turbo-test's own runner) no input ever activates a window, so every gate reports
//! Chrome's refusal for a page nobody has interacted with.
use super::*;

/// What the embedder answers for a node document's window.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ActivationQuery {
    /// The window has transient activation.
    Transient,
    /// Consume transient activation across the window's frame tree.
    ConsumeTransient,
    /// The window has history-action activation.
    HistoryAction,
    /// Consume history-action activation across the window's frame tree.
    ConsumeHistoryAction,
    /// The window's frame is cross-origin to its top-level frame.
    CrossOriginFrame,
    /// The document's permissions policy enables `fullscreen`.
    FullscreenAllowed,
}

/// Answers a query for the given Document object.
pub type ActivationHost = fn(&mut v8::PinScope, v8::Local<v8::Object>, ActivationQuery) -> bool;
struct HostSlot(ActivationHost);
struct Hook(v8::Weak<v8::Function>);

/// Register the embedder's activation authority for a realm, before guest code runs.
pub fn set_activation_host(context: v8::Local<v8::Context>, host: ActivationHost) {
    context.set_slot(std::rc::Rc::new(HostSlot(host)));
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Watcher {
    Dialog(Handle),
    Popover(Handle),
}
impl Watcher {
    fn node(self) -> Handle {
        match self {
            Watcher::Dialog(node) | Watcher::Popover(node) => node,
        }
    }
}

struct FullscreenEntry {
    node: Handle,
    prefixed: bool,
}

/// https://html.spec.whatwg.org/#close-watcher-manager
struct Manager {
    groups: Vec<Vec<Watcher>>,
    allowed: usize,
    next_allows_new_group: bool,
    canceling: std::collections::HashSet<Watcher>,
}
impl Default for Manager {
    fn default() -> Self {
        Self {
            groups: Vec::new(),
            allowed: 1,
            next_allows_new_group: true,
            canceling: Default::default(),
        }
    }
}

#[derive(Default)]
pub(super) struct State {
    fullscreen: Vec<FullscreenEntry>,
    /// Each details element's open state as of its last attribute change.
    details: HashMap<Handle, bool>,
    /// Pending details toggle tasks: the coalesced old state and the task id.
    details_tasks: HashMap<Handle, (bool, u64)>,
    next_task: u64,
    watchers: Manager,
    /// The document's open dialogs list, in opening order.
    open_dialogs: Vec<Handle>,
    dialog_pointerdown: Option<Handle>,
    popover_pointerdown: Option<Handle>,
}
impl State {
    pub(super) fn sweep_storage(&mut self, tree: &Tree) {
        self.fullscreen.retain(|entry| tree.is_live(entry.node));
        self.details.retain(|node, _| tree.is_live(*node));
        self.details_tasks.retain(|node, _| tree.is_live(*node));
        self.open_dialogs.retain(|node| tree.is_live(*node));
        for group in &mut self.watchers.groups {
            group.retain(|watcher| tree.is_live(watcher.node()));
        }
        self.watchers.groups.retain(|group| !group.is_empty());
        self.watchers
            .canceling
            .retain(|watcher| tree.is_live(watcher.node()));
        for target in [&mut self.dialog_pointerdown, &mut self.popover_pointerdown] {
            if target.is_some_and(|node| !tree.is_live(node)) {
                *target = None;
            }
        }
    }
}

fn state<R>(f: impl FnOnce(&mut State) -> R) -> Option<R> {
    DOM.with(|dom| dom.borrow_mut().as_mut().map(|dom| f(&mut dom.activation)))
}

fn html(tree: &Tree, node: Handle, name: &str) -> bool {
    tree.node_type_id(node) == 1
        && tree.namespace_id(node) == 0
        && tree.local_name(node) == Some(name)
}

fn document_object<'s>(scope: &mut v8::PinScope<'s, '_>) -> Option<v8::Local<'s, v8::Object>> {
    let root = with_tree(|tree| tree.root())?;
    Some(wrap(scope, root))
}

/// Ask the embedder about the current DOM's document. Absent a host nothing is active.
fn query(scope: &mut v8::PinScope, kind: ActivationQuery) -> bool {
    let Some(document) = document_object(scope) else {
        return false;
    };
    let host = document
        .get_creation_context(scope)
        .and_then(|context| context.get_slot::<HostSlot>().map(|slot| slot.0));
    match host {
        Some(host) => host(scope, document, kind),
        None => kind == ActivationQuery::FullscreenAllowed,
    }
}

fn fully_active() -> bool {
    DOM.with(|dom| {
        dom.borrow()
            .as_ref()
            .is_some_and(|dom| dom.browsing_context)
    })
}

/// Call the module's private JS hook in `node`'s realm.
fn hook<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    operation: &str,
    arguments: &[v8::Local<'s, v8::Value>],
) -> Option<v8::Local<'s, v8::Value>> {
    let context = scope.get_current_context();
    let document = document_object(scope)?;
    let context = document.get_creation_context(scope).unwrap_or(context);
    let function = context.get_slot::<Hook>()?;
    let function = function.0.to_local(scope)?;
    let mut values = vec![v8::String::new(scope, operation)?.into()];
    values.extend_from_slice(arguments);
    let receiver = v8::undefined(scope).into();
    v8::tc_scope!(let scope, scope);
    let result = function.call(scope, receiver, &values);
    if !scope.has_terminated() {
        if let Some(error) = scope.exception() {
            scope.reset();
            custom_elements::report_value(scope, error);
            return None;
        }
    }
    result.map(|value| v8::Local::new(scope, value))
}

// ---- Fullscreen ----

fn fullscreen_top() -> Option<Handle> {
    state(|state| state.fullscreen.last().map(|entry| entry.node)).flatten()
}

fn push_fullscreen(node: Handle, prefixed: bool) -> bool {
    state(|state| {
        if state
            .fullscreen
            .last()
            .is_some_and(|entry| entry.node == node)
        {
            return false;
        }
        state.fullscreen.retain(|entry| entry.node != node);
        state.fullscreen.push(FullscreenEntry { node, prefixed });
        true
    })
    .unwrap_or(false)
}

/// Unfullscreen the document's fullscreen element; it leaves the top layer.
fn pop_fullscreen() -> Option<(Handle, bool)> {
    state(|state| {
        state
            .fullscreen
            .pop()
            .map(|entry| (entry.node, entry.prefixed))
    })
    .flatten()
}

/// The fullscreen element removing steps: a removed element leaves the top layer at once;
/// when it was the document's fullscreen element its change event targets the document.
fn disconnect_fullscreen(scope: &mut v8::PinScope, root: Handle, include_root: bool) {
    let removed = DOM.with(|dom| {
        let mut dom = dom.borrow_mut();
        let dom = dom.as_mut()?;
        let tree = &dom.tree;
        let inside = |node: Handle| {
            let (mut previous, mut current) = (None, Some(node));
            while let Some(node) = current {
                if node == root {
                    return include_root
                        || previous.is_some_and(|child| tree.parent(child) == Some(root));
                }
                previous = Some(node);
                current = tree.parent(node).or_else(|| tree.shadow_host(node));
            }
            false
        };
        let stack = &mut dom.activation.fullscreen;
        let top = stack
            .last()
            .filter(|entry| inside(entry.node))
            .map(|entry| entry.prefixed);
        stack.retain(|entry| !inside(entry.node));
        top
    });
    if let (Some(prefixed), Some(document)) = (removed, document_object(scope)) {
        let prefixed = v8::Boolean::new(scope, prefixed);
        hook(
            scope,
            "fullscreen-removed",
            &[document.into(), prefixed.into()],
        );
    }
}

// ---- Details toggle events ----

/// Wrapper creation observes a details element's open state before script can change it.
pub(super) fn observe(handle: Handle) {
    let open = with_tree(|tree| {
        html(tree, handle, "details").then(|| tree.get_attribute_ns(handle, None, "open").is_some())
    })
    .flatten();
    if let Some(open) = open {
        state(|state| {
            state.details.entry(handle).or_insert(open);
        });
    }
}

/// https://html.spec.whatwg.org/#the-details-element attribute changed steps for `open`.
pub(super) fn attribute_changed(scope: &mut v8::PinScope, handle: Handle, name: &str) {
    if name != "open" {
        return;
    }
    let open = with_tree(|tree| {
        html(tree, handle, "details").then(|| tree.get_attribute_ns(handle, None, "open").is_some())
    })
    .flatten();
    let Some(open) = open else {
        return;
    };
    let queued = state(|state| {
        let old = state.details.insert(handle, open).unwrap_or(open);
        if old == open {
            return None;
        }
        let old = match state.details_tasks.get(&handle) {
            Some((pending, _)) => *pending,
            None => old,
        };
        state.next_task += 1;
        state.details_tasks.insert(handle, (old, state.next_task));
        Some((old, state.next_task))
    })
    .flatten();
    let Some((old, task)) = queued else {
        return;
    };
    let node = wrap(scope, handle);
    let old = v8::Boolean::new(scope, old);
    let task = v8::Number::new(scope, task as f64);
    hook(
        scope,
        "details-toggle",
        &[node.into(), old.into(), task.into()],
    );
}

/// Claim a queued details toggle task: whether it is still current.
fn take_details_task(handle: Handle, task: u64) -> bool {
    state(|state| {
        if state
            .details_tasks
            .get(&handle)
            .is_some_and(|(_, id)| *id == task)
        {
            state.details_tasks.remove(&handle);
            true
        } else {
            false
        }
    })
    .unwrap_or(false)
}

// ---- Close watchers ----

fn establish(watcher: Watcher) {
    if !fully_active() {
        return;
    }
    state(|state| {
        let manager = &mut state.watchers;
        if manager.groups.len() < manager.allowed {
            manager.groups.push(vec![watcher]);
        } else if let Some(group) = manager.groups.last_mut() {
            group.push(watcher);
        }
        manager.next_allows_new_group = true;
    });
}

fn destroy(watcher: Watcher) {
    state(|state| {
        let manager = &mut state.watchers;
        for group in &mut manager.groups {
            group.retain(|entry| *entry != watcher);
        }
        manager.groups.retain(|group| !group.is_empty());
    });
}

fn watching(watcher: Watcher) -> bool {
    state(|state| {
        state
            .watchers
            .groups
            .iter()
            .flatten()
            .any(|entry| *entry == watcher)
    })
    .unwrap_or(false)
}

/// A dialog's close watcher is enabled unless its computed closedby state is None.
fn enabled(watcher: Watcher) -> bool {
    let Watcher::Dialog(node) = watcher else {
        return true;
    };
    if !with_tree(|tree| tree.get_attribute_ns(node, None, "open").is_some()).unwrap_or(false) {
        return false;
    }
    let value = with_tree(|tree| {
        tree.get_attribute_ns(node, None, "closedby")
            .map(|value| value.to_string())
    })
    .flatten()
    .map(|value| value.to_ascii_lowercase());
    match value.as_deref() {
        Some("any" | "closerequest") => true,
        Some("none") => false,
        _ => html_dialog::is_modal(node),
    }
}

/// Dialogs establish their close watcher while opening and enter the open dialogs list.
pub(super) fn dialog_opened(node: Handle) {
    establish(Watcher::Dialog(node));
    state(|state| {
        state.open_dialogs.retain(|entry| *entry != node);
        state.open_dialogs.push(node);
    });
}

pub(super) fn dialog_closed(node: Handle) {
    destroy(Watcher::Dialog(node));
    state(|state| state.open_dialogs.retain(|entry| *entry != node));
}

pub(super) fn popover_shown(node: Handle) {
    establish(Watcher::Popover(node));
}

pub(super) fn popover_hidden(node: Handle) {
    destroy(Watcher::Popover(node));
}

/// The dialog removing steps destroy the close watcher and leave the open dialogs list.
fn disconnect_dialogs(root: Handle, include_root: bool) {
    let removed = DOM.with(|dom| {
        let dom = dom.borrow();
        let Some(dom) = dom.as_ref() else {
            return Vec::new();
        };
        let tree = &dom.tree;
        dom.activation
            .open_dialogs
            .iter()
            .copied()
            .filter(|&node| {
                let (mut previous, mut current) = (None, Some(node));
                while let Some(node) = current {
                    if node == root {
                        return include_root
                            || previous.is_some_and(|child| tree.parent(child) == Some(root));
                    }
                    previous = Some(node);
                    current = tree.parent(node).or_else(|| tree.shadow_host(node));
                }
                false
            })
            .collect::<Vec<_>>()
    });
    for node in removed {
        dialog_closed(node);
    }
}

pub(super) fn disconnect(scope: &mut v8::PinScope, root: Handle, include_root: bool) {
    disconnect_dialogs(root, include_root);
    disconnect_fullscreen(scope, root, include_root);
}

/// https://html.spec.whatwg.org/#close-watcher-request-close
fn request_close(scope: &mut v8::PinScope, watcher: Watcher, require_history: bool) -> bool {
    if !watching(watcher) || !enabled(watcher) || !fully_active() {
        return true;
    }
    if state(|state| state.watchers.canceling.contains(&watcher)).unwrap_or(true) {
        return true;
    }
    let can_prevent = !require_history
        || (state(|state| state.watchers.groups.len() < state.watchers.allowed).unwrap_or(false)
            && query(scope, ActivationQuery::HistoryAction));
    state(|state| state.watchers.canceling.insert(watcher));
    let proceed = match watcher {
        Watcher::Dialog(node) => {
            let node = wrap(scope, node);
            let can_prevent = v8::Boolean::new(scope, can_prevent);
            hook(scope, "dialog-cancel", &[node.into(), can_prevent.into()])
                .is_none_or(|value| value.is_true())
        }
        Watcher::Popover(_) => true,
    };
    state(|state| state.watchers.canceling.remove(&watcher));
    if !proceed {
        query(scope, ActivationQuery::ConsumeHistoryAction);
        return false;
    }
    if !watching(watcher) || !fully_active() {
        return true;
    }
    destroy(watcher);
    match watcher {
        Watcher::Dialog(node) => {
            let node = wrap(scope, node);
            hook(scope, "dialog-close", &[node.into()]);
        }
        Watcher::Popover(node) => html_popover::hide_popover(scope, node),
    }
    true
}

/// https://html.spec.whatwg.org/#process-close-watchers for a close request (Esc) in
/// `document`'s window. Returns whether a close watcher was processed.
pub fn process_close_watchers(scope: &mut v8::PinScope, document: v8::Local<v8::Object>) -> bool {
    let _dom = DomScope::for_object(scope, document);
    let group = state(|state| state.watchers.groups.last().cloned())
        .flatten()
        .unwrap_or_default();
    let mut processed = false;
    for watcher in group.into_iter().rev() {
        if enabled(watcher) && watching(watcher) {
            processed = true;
        }
        if !request_close(scope, watcher, true) {
            break;
        }
    }
    state(|state| {
        if state.watchers.allowed > 1 {
            state.watchers.allowed -= 1;
        }
    });
    processed
}

/// https://html.spec.whatwg.org/#notify-the-close-watcher-manager-about-user-activation
pub fn notify_user_activation(scope: &mut v8::PinScope, document: v8::Local<v8::Object>) {
    let _dom = DomScope::for_object(scope, document);
    state(|state| {
        let manager = &mut state.watchers;
        if manager.next_allows_new_group {
            manager.allowed += 1;
        }
        manager.next_allows_new_group = false;
    });
}

/// The topmost modal dialog when `node` is outside it: hit testing skips the inert page
/// and lands on that dialog's `::backdrop`, so pointer input targets the dialog.
pub fn modal_backdrop<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    node: v8::Local<v8::Object>,
) -> Option<v8::Local<'s, v8::Object>> {
    let _dom = DomScope::for_object(scope, node);
    let handle = handle_of(scope, node)?;
    let dialog = html_dialog::blocking_dialog(handle)?;
    Some(wrap(scope, dialog))
}

/// Shadow-including inclusive ancestors (slot assignment is not consulted).
fn flat_ancestors(tree: &Tree, node: Handle) -> Vec<Handle> {
    let mut out = Vec::new();
    let mut current = Some(node);
    while let Some(node) = current {
        out.push(node);
        current = tree.parent(node).or_else(|| tree.shadow_host(node));
    }
    out
}

/// https://html.spec.whatwg.org/#topmost-clicked-popover
fn topmost_clicked_popover(scope: &mut v8::PinScope, node: Handle) -> Option<Handle> {
    let ancestors = with_tree(|tree| flat_ancestors(tree, node)).unwrap_or_default();
    let clicked = ancestors
        .iter()
        .copied()
        .find(|&node| html_popover::auto_showing(node));
    let mut invoked = None;
    for &ancestor in &ancestors {
        let value = wrap(scope, ancestor);
        let target = hook(scope, "invoker-target", &[value.into()])
            .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
            .and_then(|target| handle_of(scope, target));
        if let Some(target) = target.filter(|&target| html_popover::auto_showing(target)) {
            invoked = Some(target);
            break;
        }
    }
    match (clicked, invoked) {
        (Some(clicked), Some(invoked)) => Some(html_popover::topmost_of(clicked, invoked)),
        (clicked, invoked) => clicked.or(invoked),
    }
}

/// https://html.spec.whatwg.org/#nearest-clicked-dialog; `backdrop` is the embedder's
/// hit test landing outside the topmost modal dialog's border box.
fn nearest_clicked_dialog(node: Handle, backdrop: bool) -> Option<Handle> {
    with_tree(|tree| {
        if backdrop && html(tree, node, "dialog") {
            return None;
        }
        flat_ancestors(tree, node).into_iter().find(|&ancestor| {
            html(tree, ancestor, "dialog")
                && tree.get_attribute_ns(ancestor, None, "open").is_some()
        })
    })
    .flatten()
}

/// Light dismiss for a trusted `pointerdown`/`pointerup` the embedder dispatched:
/// https://html.spec.whatwg.org/#popover-light-dismiss and #light-dismiss-open-dialogs.
pub fn light_dismiss(scope: &mut v8::PinScope, event: v8::Local<v8::Object>, backdrop: bool) {
    let Some(record) = event_record(scope, event) else {
        return;
    };
    if !get_bool_prop(scope, record, "trusted") {
        return;
    }
    let kind = get_str_prop(scope, record, "type").unwrap_or_default();
    let down = match kind.as_str() {
        "pointerdown" => true,
        "pointerup" => false,
        _ => return,
    };
    let Some(target) = get_prop(scope, record, "target")
        .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
    else {
        return;
    };
    let _dom = DomScope::for_object(scope, target);
    let Some(node) = handle_of(scope, target) else {
        return;
    };
    if html_popover::any_auto_showing() {
        let ancestor = topmost_clicked_popover(scope, node);
        if down {
            state(|state| state.popover_pointerdown = ancestor);
        } else {
            let same = state(|state| state.popover_pointerdown.take() == ancestor).unwrap_or(false);
            if same {
                html_popover::hide_until(scope, ancestor);
            }
        }
    }
    if state(|state| state.open_dialogs.is_empty()).unwrap_or(true) {
        return;
    }
    let ancestor = nearest_clicked_dialog(node, backdrop);
    if down {
        state(|state| state.dialog_pointerdown = ancestor);
        return;
    }
    let same = state(|state| state.dialog_pointerdown.take() == ancestor).unwrap_or(false);
    let Some(topmost) = state(|state| state.open_dialogs.last().copied()).flatten() else {
        return;
    };
    if !same || ancestor == Some(topmost) {
        return;
    }
    let any = with_tree(|tree| {
        tree.get_attribute_ns(topmost, None, "closedby")
            .is_some_and(|value| value.eq_ignore_ascii_case("any"))
    })
    .unwrap_or(false);
    if any {
        request_close(scope, Watcher::Dialog(topmost), false);
    }
}

// ---- Rendering ----

/// Whether `node` has a box: connected and no flat-tree inclusive ancestor is
/// `display: none` (the native cascade plus the UA `[hidden]` rule).
fn rendered(scope: &mut v8::PinScope, node: Handle) -> bool {
    use turbo_dom_parser::rtdom::cascade::{computed_style_with_policy, get_property_value};
    let policy = css_policy::cascade_policy(scope);
    with_tree(|tree| {
        is_connected(tree, node)
            && flat_ancestors(tree, node)
                .into_iter()
                .filter(|&ancestor| tree.node_type_id(ancestor) == 1)
                .all(|ancestor| {
                    !(tree.namespace_id(ancestor) == 0 && tree.has_attribute(ancestor, "hidden"))
                        && !get_property_value(
                            &computed_style_with_policy(tree, ancestor, &policy),
                            "display",
                        )
                        .eq_ignore_ascii_case("none")
                })
    })
    .unwrap_or(false)
}

fn dispatch(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let Ok(target) = v8::Local::<v8::Object>::try_from(args.get(1)) else {
        return;
    };
    let _dom = DomScope::for_object(scope, target);
    let Some(node) = handle_of(scope, target) else {
        return;
    };
    let query_kind = |index: i32| match index {
        0 => Some(ActivationQuery::Transient),
        1 => Some(ActivationQuery::ConsumeTransient),
        2 => Some(ActivationQuery::HistoryAction),
        3 => Some(ActivationQuery::ConsumeHistoryAction),
        4 => Some(ActivationQuery::CrossOriginFrame),
        5 => Some(ActivationQuery::FullscreenAllowed),
        _ => None,
    };
    match args.get(0).to_rust_string_lossy(scope).as_str() {
        "query" => {
            let index = args.get(2).int32_value(scope).unwrap_or(-1);
            if let Some(kind) = query_kind(index) {
                let answer = query(scope, kind);
                rv.set_bool(answer);
            }
        }
        "disabled" => rv.set_bool(html_invokers::disabled_control(node)),
        "rendered" => {
            let rendered = rendered(scope, node);
            rv.set_bool(rendered);
        }
        "active" => rv.set_bool(fully_active()),
        "document" => {
            if let Some(document) = document_object(scope) {
                rv.set(document.into());
            }
        }
        "dispatch" => {
            let Ok(event) = v8::Local::<v8::Object>::try_from(args.get(2)) else {
                return;
            };
            dispatch_host_event(scope, event, target);
        }
        "schedule" => {
            if let Ok(callback) = v8::Local::<v8::Function>::try_from(args.get(2)) {
                html_popover::schedule(scope, callback);
            }
        }
        "fullscreen-top" => match fullscreen_top() {
            Some(top) => {
                let top = wrap(scope, top);
                rv.set(top.into());
            }
            None => rv.set(v8::null(scope).into()),
        },
        "fullscreen-push" => {
            let pushed = push_fullscreen(node, args.get(2).is_true());
            rv.set_bool(pushed);
        }
        "fullscreen-pop" => match pop_fullscreen() {
            Some((element, prefixed)) => {
                let element = wrap(scope, element);
                let prefixed = v8::Boolean::new(scope, prefixed);
                let pair = v8::Array::new_with_elements(scope, &[element.into(), prefixed.into()]);
                rv.set(pair.into());
            }
            None => rv.set(v8::null(scope).into()),
        },
        "details-task" => {
            let task = args.get(2).number_value(scope).unwrap_or(-1.0) as u64;
            rv.set_bool(take_details_task(node, task));
        }
        "dialog-opened" => dialog_opened(node),
        "dialog-closed" => dialog_closed(node),
        _ => {}
    }
}

/// `:fullscreen` matches every element in the document's fullscreen stack.
pub(super) fn fullscreen_elements() -> Vec<Handle> {
    state(|state| state.fullscreen.iter().map(|entry| entry.node).collect()).unwrap_or_default()
}

pub(super) fn install(scope: &mut v8::PinScope) {
    let global = scope.get_current_context().global(scope);
    bind_method(scope, global, "__activationNative", dispatch);
    run_js(scope, include_str!("user_activation_gates.js"));
    let key = v8::String::new(scope, "__captureActivationHook").unwrap();
    if let Some(function) = global
        .get(scope, key.into())
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
    {
        if let Some(function) = dom_ownership::helper_function(scope, function) {
            scope
                .get_current_context()
                .set_slot(std::rc::Rc::new(Hook(function)));
        }
    }
    global.delete(scope, key.into());
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Run the committed async probe, draining a FIFO task queue standing in for timers.
    fn run_probe(probe: &str) -> serde_json::Value {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(
            scope,
            "<!doctype html><html><head></head><body></body></html>",
        );
        run_js(
            scope,
            "globalThis.__tasks=[];globalThis.setTimeout=callback=>{__tasks.push(callback);return __tasks.length};",
        );
        capture_popover_scheduler(scope);
        let source = format!(
            "Promise.resolve({}).then(JSON.stringify)",
            probe.trim().trim_end_matches(';')
        );
        let source = v8::String::new(scope, &source).unwrap();
        v8::tc_scope!(let scope, scope);
        let promise = v8::Script::compile(scope, source, None)
            .and_then(|script| script.run(scope))
            .and_then(|value| v8::Local::<v8::Promise>::try_from(value).ok())
            .unwrap_or_else(|| {
                panic!(
                    "probe threw: {:?}",
                    scope
                        .exception()
                        .map(|error| error.to_rust_string_lossy(scope))
                )
            });
        let step = v8::String::new(scope, "__tasks.length&&__tasks.shift()()").unwrap();
        let step = v8::Script::compile(scope, step, None).unwrap();
        for _ in 0..100_000 {
            scope.perform_microtask_checkpoint();
            if promise.state() != v8::PromiseState::Pending {
                break;
            }
            step.run(scope);
        }
        let value = promise.result(scope).to_rust_string_lossy(scope);
        assert_eq!(promise.state(), v8::PromiseState::Fulfilled, "{value}");
        reset();
        serde_json::from_str(&value).unwrap()
    }

    /// Measured Chrome behavior this binding does not reproduce, as `path = binding value`:
    /// a/area activation (following the hyperlink) needs navigation, which this engine
    /// does not perform from activation, so fragment links leave location.hash unchanged
    /// (and never focus the indicated part).
    const KNOWN_GAPS: &[&str] = &[
        r#"/synthetic/mouse:wa/sync/hash = """#,
        r#"/synthetic/mouse:wa/sync/active = "BODY""#,
        r#"/synthetic/mouse:wa/hashLater = """#,
        r#"/synthetic/mouse:anchor/sync/hash = """#,
        r#"/synthetic/mouse:anchor/hashLater = """#,
        r#"/synthetic/mouse:aspan/sync/hash = """#,
        r#"/synthetic/mouse:aspan/hashLater = """#,
        r#"/synthetic/mouseNoBubble:wa/sync/hash = """#,
        r#"/synthetic/mouseNoBubble:wa/sync/active = "BODY""#,
        r#"/synthetic/mouseNoBubble:wa/hashLater = """#,
        r#"/synthetic/mouseNoBubble:anchor/sync/hash = """#,
        r#"/synthetic/mouseNoBubble:anchor/hashLater = """#,
        r#"/synthetic/pointer:wa/sync/hash = """#,
        r#"/synthetic/pointer:wa/sync/active = "BODY""#,
        r#"/synthetic/pointer:wa/hashLater = """#,
        r#"/synthetic/pointer:anchor/sync/hash = """#,
        r#"/synthetic/pointer:anchor/hashLater = """#,
        r#"/synthetic/pointer:aspan/sync/hash = """#,
        r#"/synthetic/pointer:aspan/hashLater = """#,
        r#"/synthetic/method:wa/sync/hash = """#,
        r#"/synthetic/method:wa/sync/active = "BODY""#,
        r#"/synthetic/method:wa/hashLater = """#,
        r#"/synthetic/method:anchor/sync/hash = """#,
        r#"/synthetic/method:anchor/hashLater = """#,
        r#"/synthetic/method:aspan/sync/hash = """#,
        r#"/synthetic/method:aspan/hashLater = """#,
    ];

    fn differences(
        path: &str,
        actual: &serde_json::Value,
        expected: &serde_json::Value,
        out: &mut Vec<String>,
    ) {
        match (actual, expected) {
            (serde_json::Value::Object(a), serde_json::Value::Object(e)) => {
                for (key, value) in e {
                    differences(
                        &format!("{path}/{key}"),
                        a.get(key).unwrap_or(&serde_json::Value::Null),
                        value,
                        out,
                    );
                }
                for key in a.keys().filter(|key| !e.contains_key(*key)) {
                    out.push(format!("{path}/{key} (extra)"));
                }
            }
            _ if actual == expected => {}
            _ => out.push(format!("{path} = {actual}")),
        }
    }

    #[test]
    fn activation_gates_match_chrome_without_user_activation() {
        let control: serde_json::Value =
            serde_json::from_str(include_str!("user_activation_gates_chrome156.json")).unwrap();
        assert_eq!(control["browser"], "Chrome/156.0.8078.47");
        let actual = run_probe(include_str!("user_activation_gates_probe.js"));
        let mut found = Vec::new();
        differences("", &actual, &control["result"], &mut found);
        let unexpected = found
            .iter()
            .filter(|line| !KNOWN_GAPS.contains(&line.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        let missing = KNOWN_GAPS
            .iter()
            .filter(|gap| !found.iter().any(|line| line == *gap))
            .collect::<Vec<_>>();
        assert!(
            unexpected.is_empty() && missing.is_empty(),
            "unexpected:\n{}\nresolved gaps:\n{missing:?}",
            unexpected.join("\n")
        );
    }
}
