//! Pointer capture for the mouse pointer (id 1) and a document's pointer lock.
//!
//! Native per-document state: the mouse pointer's active buttons state, its pending and
//! active pointer capture target overrides, and the lock element with its pending flag.
//! Only the embedder's trusted input driver ([`host_pointer`]) presses buttons, processes
//! pending capture and reads the event-retargeting overrides; guest `dispatchEvent` never
//! moves any of it. Pointer lock grants need embedder evidence ([`PointerLockPolicy`]): a
//! realm without it never has transient activation, so its requests are refused exactly
//! like Chrome refuses a gesture-less request.
use super::*;

#[derive(Default)]
pub(super) struct State {
    /// A trusted press of the mouse pointer is down (Chrome's active buttons state).
    buttons: bool,
    /// The pending pointer capture target override; `hasPointerCapture` reads it.
    pending: Option<Handle>,
    /// The pointer capture target override the next trusted pointer event retargets to.
    capture: Option<Handle>,
    /// The element holding (or waiting for) the lock, and whether the grant is pending.
    lock: Option<Handle>,
    lock_pending: bool,
    unadjusted: bool,
}

/// Embedder evidence for a realm's pointer lock requests: transient user activation and
/// the sandboxed pointer lock browsing context flag. Absent, a realm is never activated.
pub struct PointerLockPolicy {
    pub activation: fn(v8::Local<v8::Context>) -> bool,
    pub sandboxed: fn(v8::Local<v8::Context>) -> bool,
}

/// The JS task that queues a trusted `pointerlockchange`/`pointerlockerror` at a document.
struct LockEvents(v8::Weak<v8::Function>);

fn with_state<R>(f: impl FnOnce(&mut State) -> R) -> Option<R> {
    DOM.with(|dom| dom.borrow_mut().as_mut().map(|dom| f(&mut dom.pointer)))
}

fn read<R>(f: impl FnOnce(&State) -> R) -> Option<R> {
    DOM.with(|dom| dom.borrow().as_ref().map(|dom| f(&dom.pointer)))
}

fn connected(node: Handle) -> bool {
    with_tree(|tree| is_connected(tree, node)).unwrap_or(false)
}

fn within(tree: &Tree, node: Handle, root: Handle, include_root: bool) -> bool {
    let (mut previous, mut current) = (None, Some(node));
    while let Some(entry) = current {
        if entry == root {
            return include_root || previous.is_some_and(|child| tree.parent(child) == Some(root));
        }
        previous = Some(entry);
        current = tree.parent(entry).or_else(|| tree.shadow_host(entry));
    }
    false
}

/// Element removing steps: a removed pending capture target is dropped (the active one
/// is reported lost to the document at the next processing), and a removed lock element
/// exits the lock with a queued `pointerlockchange` at its document.
pub(super) fn disconnect(scope: &mut v8::PinScope, root: Handle, include_root: bool) {
    let unlocked = DOM.with(|dom| {
        let mut dom = dom.borrow_mut();
        let Some(dom) = dom.as_mut() else {
            return false;
        };
        let tree = &dom.tree;
        let state = &mut dom.pointer;
        if state
            .pending
            .is_some_and(|node| within(tree, node, root, include_root))
        {
            state.pending = None;
        }
        let removed = state
            .lock
            .is_some_and(|node| within(tree, node, root, include_root));
        if removed {
            state.lock = None;
            state.lock_pending = false;
        }
        removed
    });
    if unlocked {
        let document = with_tree(|tree| tree.root()).map(|root| wrap(scope, root));
        if let Some(document) = document {
            queue_event(scope, document, "pointerlockchange");
        }
    }
}

fn queue_event(scope: &mut v8::PinScope, target: v8::Local<v8::Object>, kind: &str) {
    let Some(context) = target.get_creation_context(scope) else {
        return;
    };
    let Some(events) = context.get_slot::<LockEvents>() else {
        return;
    };
    let function = dom_ownership::local(scope, &events.0);
    let kind = v8::String::new(scope, kind).unwrap();
    let receiver = v8::undefined(scope);
    function.call(scope, receiver.into(), &[kind.into(), target.into()]);
}

/// The lock element retargeted into `root`'s tree (a document or shadow root); null while
/// the grant is pending, as Chrome's DocumentOrShadowRoot.pointerLockElement reports.
pub(super) fn lock_element_in(
    scope: &mut v8::PinScope,
    root: v8::Local<v8::Object>,
) -> Option<Handle> {
    let _dom = DomScope::for_object(scope, root);
    let root = handle_of(scope, root)?;
    let lock = read(|state| state.lock.filter(|_| !state.lock_pending)).flatten()?;
    with_tree(|tree| {
        let mut node = lock;
        loop {
            let mut top = node;
            while let Some(parent) = tree.parent(top) {
                top = parent;
            }
            if top == root {
                return Some(node);
            }
            node = tree.shadow_host(top)?;
        }
    })
    .flatten()
}

/// `Document.exitPointerLock()` for a document of a browsing context.
pub(super) fn exit(scope: &mut v8::PinScope, document: v8::Local<v8::Object>) {
    let unlocked = {
        let _dom = DomScope::for_object(scope, document);
        with_state(|state| {
            let held = state.lock.is_some();
            state.lock = None;
            state.lock_pending = false;
            held
        })
        .unwrap_or(false)
    };
    if unlocked {
        queue_event(scope, document, "pointerlockchange");
    }
}

fn policy(scope: &mut v8::PinScope, node: v8::Local<v8::Object>, sandbox: bool) -> bool {
    let Some(context) = node.get_creation_context(scope) else {
        return sandbox;
    };
    match context.get_slot::<PointerLockPolicy>() {
        Some(policy) if sandbox => (policy.sandboxed)(context),
        Some(policy) => (policy.activation)(context),
        None => false,
    }
}

fn element_arg(scope: &mut v8::PinScope, value: v8::Local<v8::Value>) -> Option<Handle> {
    let object = v8::Local::<v8::Object>::try_from(value).ok()?;
    handle_of(scope, object)
}

fn native<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    let Ok(target) = v8::Local::<v8::Object>::try_from(args.get(1)) else {
        return;
    };
    let operation = args.get(0).to_rust_string_lossy(scope);
    match operation.as_str() {
        "activation" => return rv.set_bool(policy(scope, target, false)),
        "sandboxed" => return rv.set_bool(policy(scope, target, true)),
        "dispatch" => {
            if let Ok(event) = v8::Local::<v8::Object>::try_from(args.get(2)) {
                dispatch_host_event(scope, event, target);
            }
            return;
        }
        _ => {}
    }
    let _dom = DomScope::for_object(scope, target);
    match operation.as_str() {
        "frame" => rv.set_bool(DOM.with(|dom| {
            dom.borrow()
                .as_ref()
                .is_some_and(|dom| dom.browsing_context)
        })),
        "buttons" => rv.set_bool(read(|state| state.buttons).unwrap_or(false)),
        "pending" => {
            let pending = read(|state| state.pending).flatten();
            let value = wrap_opt(scope, pending);
            rv.set(value);
        }
        "set-pending" => {
            let node = element_arg(scope, args.get(2));
            with_state(|state| state.pending = node);
        }
        "lock" => {
            let lock = read(|state| state.lock).flatten();
            let value = wrap_opt(scope, lock);
            rv.set(value);
        }
        "lock-pending" => rv.set_bool(read(|state| state.lock_pending).unwrap_or(false)),
        "unadjusted" => rv.set_bool(read(|state| state.unadjusted).unwrap_or(false)),
        "set-lock" => {
            let node = element_arg(scope, args.get(2));
            let (pending, unadjusted) = (args.get(3).is_true(), args.get(4).is_true());
            with_state(|state| {
                state.lock = node;
                state.lock_pending = node.is_some() && pending;
                state.unadjusted = unadjusted;
            });
        }
        // The grant completed: the lock (whatever element now holds it) is no longer
        // pending, and the mouse pointer's pending capture is released.
        "grant" => {
            let unadjusted = args.get(2).is_true();
            with_state(|state| {
                state.lock_pending = false;
                state.unadjusted = unadjusted;
                state.pending = None;
            });
        }
        "schedule" => {
            if let Ok(callback) = v8::Local::<v8::Function>::try_from(args.get(2)) {
                html_popover::schedule(scope, callback);
            }
        }
        _ => {}
    }
}

/// The embedder's trusted mouse input steps for `document`'s browsing context:
/// - `press` / `release`: enter or leave the active buttons state before the trusted
///   `pointerdown` / `pointerup` dispatch;
/// - `process`: process pending pointer capture, returning `[lost, got]` targets (`lost`
///   is the document when the old override is no longer connected) for the driver to fire;
/// - `capture`: the active override the next pointer event retargets to, or null;
/// - `implicit`: the implicit release after the trusted `pointerup`;
/// - `locked`: the granted lock element mouse events retarget to, or null.
pub fn host_pointer<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    document: v8::Local<'s, v8::Object>,
    command: &str,
) -> v8::Local<'s, v8::Value> {
    let _dom = DomScope::for_object(scope, document);
    let null = v8::null(scope).into();
    match command {
        "press" | "release" => {
            with_state(|state| state.buttons = command == "press");
            null
        }
        "implicit" => {
            with_state(|state| state.pending = None);
            null
        }
        "capture" => {
            let capture = read(|state| state.capture).flatten();
            wrap_opt(scope, capture)
        }
        "locked" => {
            let lock = read(|state| state.lock.filter(|_| !state.lock_pending)).flatten();
            wrap_opt(scope, lock)
        }
        "process" => {
            let Some((capture, pending)) = with_state(|state| {
                let previous = (state.capture, state.pending);
                state.capture = state.pending;
                previous
            }) else {
                return null;
            };
            let (lost, got) = if capture == pending {
                (None, None)
            } else {
                let lost = capture.map(|node| {
                    if connected(node) {
                        node
                    } else {
                        with_tree(|tree| tree.root()).unwrap_or(node)
                    }
                });
                (lost, pending)
            };
            let lost = wrap_opt(scope, lost);
            let got = wrap_opt(scope, got);
            v8::Array::new_with_elements(scope, &[lost, got]).into()
        }
        _ => null,
    }
}

pub(super) fn install(scope: &mut v8::PinScope) {
    let global = scope.get_current_context().global(scope);
    bind_method(scope, global, "__pointerNative", native);
    run_js(scope, include_str!("pointer_events.js"));
    let key = v8::String::new(scope, "__pointerLockEvents").unwrap();
    if let Some(function) = global
        .get(scope, key.into())
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
    {
        if let Some(function) = dom_ownership::helper_function(scope, function) {
            scope
                .get_current_context()
                .set_slot(std::rc::Rc::new(LockEvents(function)));
        }
    }
    global.delete(scope, key.into());
}
