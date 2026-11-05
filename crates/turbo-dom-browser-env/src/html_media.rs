//! HTMLMediaElement's playback state (turbo-dom#323): the native half of html_media.js.
//!
//! The JS module owns the media element state machine; this module gives it the document's
//! trusted task scheduler, trusted event dispatch, the embedder's sticky activation answer
//! (autoplay policy), the `src` attribute changed steps (the media element load algorithm,
//! a track's reload) and the insertion / removal steps of `<source>` and `<track>` children
//! (turbo-dom#339).
//!
//! Insertions and removals reach this module through the mutation pipeline: once a document
//! has handed script an audio, video, source or track element, it records its mutations, and
//! each childList record that adds a source under a media element, or adds or removes a track
//! child of one, becomes a pending step. The steps run (in record order) when the DOM call that
//! made them returns, so their synchronous effects are visible to the caller.
use super::*;
use std::cell::{Cell, RefCell};
use turbo_dom_parser::rtdom::mutations::{MutationEvent, MutationRecord};

/// html_media.js's private hook: `hook("src", element)` runs the src attribute changed steps,
/// `hook("source" | "addtrack" | "removetrack", media, node, batch)` a source's insertion, a
/// track child's insertion / removal (`batch`: the element's nodes from the same DOM call).
struct Hook(v8::Weak<v8::Function>);
fn_slots!([Hook]);

/// A child's insertion / removal step, in the document it happened in.
struct Step {
    document: u64,
    operation: &'static str,
    media: Handle,
    node: Handle,
}

thread_local! {
    /// Script has seen an audio, video, source or track element: mutations are recorded.
    static ACTIVE: Cell<bool> = const { Cell::new(false) };
    static PENDING: RefCell<Vec<Step>> = const { RefCell::new(Vec::new()) };
}

fn html(tree: &Tree, node: Handle, names: &[&str]) -> bool {
    tree.node_type_id(node) == 1
        && tree.namespace_id(node) == 0
        && tree
            .local_name(node)
            .is_some_and(|name| names.contains(&name))
}
const MEDIA: &[&str] = &["audio", "video"];

/// A node is about to reach script: an element the media steps apply to turns recording on.
pub(super) fn observe(node: Handle) {
    if ACTIVE.with(Cell::get) {
        return;
    }
    if with_tree(|tree| html(tree, node, &["audio", "video", "source", "track"])).unwrap_or(false) {
        ACTIVE.with(|active| active.set(true));
    }
}

pub(super) fn recording() -> bool {
    ACTIVE.with(Cell::get)
}

/// The insertion / removal steps one childList record calls for: a removed track child, an
/// added track child, then each source child of a media element in an added subtree (Blink's
/// HTMLSourceElement::InsertedInto runs for every inserted source, its parent a media element;
/// a track's only when it was inserted into its media parent itself).
pub(super) fn mutation(event: &MutationEvent) {
    if !ACTIVE.with(Cell::get) {
        return;
    }
    let MutationRecord::ChildList {
        target,
        added,
        removed,
    } = &event.record
    else {
        return;
    };
    let document = DOM_ID.with(Cell::get);
    let steps = with_tree(|tree| {
        let mut steps = Vec::new();
        let media = html(tree, *target, MEDIA);
        let mut step = |operation, media, node| {
            steps.push(Step {
                document,
                operation,
                media,
                node,
            });
        };
        for &node in removed {
            if media && html(tree, node, &["track"]) {
                step("removetrack", *target, node);
            }
        }
        for &node in added {
            if media && html(tree, node, &["track"]) {
                step("addtrack", *target, node);
            }
            let mut pending = vec![node];
            while let Some(current) = pending.pop() {
                if html(tree, current, &["source"]) {
                    if let Some(parent) = tree
                        .parent(current)
                        .filter(|parent| html(tree, *parent, MEDIA))
                    {
                        step("source", parent, current);
                    }
                }
                pending.extend(tree.children(current).into_iter().rev());
            }
        }
        steps
    })
    .unwrap_or_default();
    PENDING.with(|pending| pending.borrow_mut().extend(steps));
}

/// Run the pending insertion / removal steps (the DOM call that queued them has returned).
pub(super) fn flush(scope: &mut v8::PinScope) {
    loop {
        let steps = PENDING.with(|pending| std::mem::take(&mut *pending.borrow_mut()));
        if steps.is_empty() || scope.is_execution_terminating() {
            return;
        }
        for step in &steps {
            let _dom = DomScope::enter(step.document);
            let media = wrap(scope, step.media);
            let node = wrap(scope, step.node).into();
            // Every node of the element's steps from this DOM call: inserted since, so a
            // first look at a parsed element (its catch-up) leaves them out.
            let batch = steps
                .iter()
                .filter(|other| other.document == step.document && other.media == step.media)
                .map(|other| wrap(scope, other.node).into())
                .collect::<Vec<_>>();
            let batch = v8::Array::new_with_elements(scope, &batch).into();
            call(scope, step.operation, media, &[node, batch]);
        }
    }
}

pub(super) fn clear() {
    let _ = ACTIVE.try_with(|active| active.set(false));
    let _ = PENDING.try_with(|pending| pending.borrow_mut().clear());
}

/// Call html_media.js's hook in the element's realm, reporting what it throws.
fn call(
    scope: &mut v8::PinScope,
    operation: &str,
    element: v8::Local<v8::Object>,
    rest: &[v8::Local<v8::Value>],
) {
    let context = element
        .get_creation_context(scope)
        .unwrap_or_else(|| scope.get_current_context());
    let Some(function) = context
        .get_slot::<Hook>()
        .and_then(|hook| hook.0.to_local(scope))
    else {
        return;
    };
    let operation = v8::String::new(scope, operation).unwrap();
    let receiver = v8::undefined(scope).into();
    v8::tc_scope!(let scope, scope);
    let mut arguments = vec![operation.into(), element.into()];
    arguments.extend_from_slice(rest);
    function.call(scope, receiver, &arguments);
    if !scope.has_terminated() {
        if let Some(error) = scope.exception() {
            scope.reset();
            custom_elements::report_value(scope, error);
        }
    }
}

fn dispatch(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    // html_media.js (the only caller; the native is deleted from the global before guest code
    // runs) passes a media element it brand-checked, or another node of its document.
    let target =
        v8::Local::<v8::Object>::try_from(args.get(1)).expect("the media hook passes a node");
    let _dom = DomScope::for_object(scope, target);
    match args.get(0).to_rust_string_lossy(scope).as_str() {
        "dispatch" => {
            let event = v8::Local::<v8::Object>::try_from(args.get(2))
                .expect("the media hook dispatches an Event it constructed");
            dispatch_host_event(scope, event, target);
        }
        "sticky" => rv.set_bool(user_activation_gates::sticky_activation(scope)),
        // A media element's own events go to the element; its TextTrackList's to the list.
        "dispatchAt" => {
            let event = v8::Local::<v8::Object>::try_from(args.get(2))
                .expect("the media hook dispatches an Event it constructed");
            let list = v8::Local::<v8::Object>::try_from(args.get(3))
                .expect("the media hook dispatches at an object it made");
            dispatch_host_event(scope, event, list);
        }
        // "schedule", the last operation html_media.js sends.
        operation => {
            debug_assert_eq!(operation, "schedule");
            let callback = v8::Local::<v8::Function>::try_from(args.get(2))
                .expect("the media hook schedules a function");
            html_popover::schedule(scope, callback);
        }
    }
}

/// The `src` content attribute of an HTML media or track element was set, changed or removed.
pub(super) fn attribute_changed(scope: &mut v8::PinScope, element: v8::Local<v8::Object>) {
    let operation = handle_of(scope, element).and_then(|handle| {
        with_tree(|tree| {
            if html(tree, handle, MEDIA) {
                Some("src")
            } else if html(tree, handle, &["track"]) {
                Some("tracksrc")
            } else {
                None
            }
        })
        .flatten()
    });
    if let Some(operation) = operation {
        call(scope, operation, element, &[]);
    }
}

pub(super) fn install(scope: &mut v8::PinScope) {
    let global = scope.get_current_context().global(scope);
    bind_method(scope, global, "__mediaNative", dispatch);
    run_js(scope, include_str!("html_media.js"));
    let key = v8::String::new(scope, "__captureMediaHook").unwrap();
    if let Some(function) = global
        .get(scope, key.into())
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
    {
        context_rehydrate::capture::<Hook>(scope, function);
    }
    global.delete(scope, key.into());
}
