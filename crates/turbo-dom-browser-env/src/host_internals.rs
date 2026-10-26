//! Host-only access to the native documents behind DOM wrappers (feature
//! `host-internals`, doc-hidden).
//!
//! For embedders that implement native services over the live tree — turbo-surf's
//! native layout capture (`live_layout_dom.rs`) and compression-dictionary link
//! discovery (`compression_dictionary_dom.rs`). Nothing here is reachable from
//! page JavaScript; it only hands native handles and private records to Rust.
//!
//! The binding keeps one native document per V8 context and switches the
//! thread's *current* document when a callback enters through a node of another
//! context. The functions below follow the same model:
//!
//! 1. enter the document that owns a wrapper ([`enter_document_of`]) or a
//!    context ([`enter_context_document`]) — an RAII [`DocumentScope`];
//! 2. read the current document's [`Tree`] with [`with_tree`], translate
//!    wrappers to [`Handle`]s with [`handle_of`] and back with [`wrap`].
//!
//! Rules (all enforced by the binding's own callbacks the same way):
//! - `DocumentScope`s nest like a stack: drop them in reverse order of creation
//!   (ordinary lexical scoping does this).
//! - [`with_tree`]'s closure must not call back into the binding or V8 (the tree
//!   is borrowed for its duration); collect handles inside, act on them after.
//! - Handles are only meaningful for the document that was current when they
//!   were read; [`wrap`] and [`with_tree`] act on the current document.
use crate::browser_env::{
    collection_records, css_policy, dom_id_of, handle_of as binding_handle_of,
    is_connected as binding_is_connected, retire_custom_element_context,
    with_tree as binding_with_tree, wrap as binding_wrap, DomContextId, DomScope, DOM,
    DOM_CONTEXTS, DOM_ID,
};

pub use crate::browser_env::host_composed_children;
pub use crate::css_snapshot::{RuleSnapshot, SheetSnapshot};
pub use turbo_dom_parser::rtdom::cascade::StylePolicy;
pub use turbo_dom_parser::rtdom::css_values::zoom_value;
pub use turbo_dom_parser::rtdom::tree::{Handle, Tree};

/// While alive, the binding's current document is the one that was entered;
/// dropping it restores the caller's document.
#[must_use = "the entered document is left again when the guard drops"]
pub struct DocumentScope {
    _scope: DomScope,
}

/// Enter the native document that owns `object` (a node or document wrapper).
/// Objects that are not native DOM wrappers enter the main document.
pub fn enter_document_of(scope: &mut v8::PinScope, object: v8::Local<v8::Object>) -> DocumentScope {
    DocumentScope {
        _scope: DomScope::for_object(scope, object),
    }
}

/// Enter the native document installed in `context`, if one was installed with
/// an independent document (`install_independent_*`).
pub fn enter_context_document(context: v8::Local<v8::Context>) -> Option<DocumentScope> {
    let id = context.get_slot::<DomContextId>()?;
    Some(DocumentScope {
        _scope: DomScope::enter(id.0),
    })
}

/// Read the current document's tree. `None` if no document is installed.
pub fn with_tree<R>(f: impl FnOnce(&Tree) -> R) -> Option<R> {
    binding_with_tree(f)
}

/// The native handle behind a node wrapper, or `None` for any other object.
pub fn handle_of(scope: &mut v8::PinScope, object: v8::Local<v8::Object>) -> Option<Handle> {
    binding_handle_of(scope, object)
}

/// The (identity-preserving) wrapper of `handle` in the current document.
pub fn wrap<'s>(scope: &mut v8::PinScope<'s, '_>, handle: Handle) -> v8::Local<'s, v8::Object> {
    binding_wrap(scope, handle)
}

/// Identity of the native document that owns `object`.
pub fn document_id_of(scope: &mut v8::PinScope, object: v8::Local<v8::Object>) -> u64 {
    dom_id_of(scope, object)
}

/// Identity of the current native document.
pub fn current_document_id() -> u64 {
    DOM_ID.with(|id| id.get())
}

/// Whether the current document is displayed in a browsing context (a live
/// window), as opposed to an inert or retired document.
pub fn is_browsing_context() -> bool {
    DOM.with(|state| {
        state
            .borrow()
            .as_ref()
            .is_some_and(|state| state.browsing_context)
    })
}

/// The V8 context the current document was installed in, while it is alive.
pub fn current_document_context<'s>(
    scope: &mut v8::PinScope<'s, '_>,
) -> Option<v8::Local<'s, v8::Context>> {
    DOM_CONTEXTS.with(|contexts| {
        contexts
            .borrow()
            .get(&current_document_id())
            .and_then(|context| context.to_local(scope))
    })
}

/// Retire a frame's document when its context goes away: its custom-element
/// registry is retired and the document stops being a browsing context.
pub fn retire_context(context: v8::Local<v8::Context>) {
    retire_custom_element_context(context);
    let Some(_document) = enter_context_document(context) else {
        return;
    };
    DOM.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            state.browsing_context = false;
        }
    });
}

/// Whether `handle` is connected (shadow-including) in `tree`.
pub fn is_connected(tree: &Tree, handle: Handle) -> bool {
    binding_is_connected(tree, handle)
}

/// The private interface record of a DOM wrapper (e.g. a document's
/// `cssHost`, whose `sheet`/`readAdopted`/`exportSheet` methods produce
/// [`SheetSnapshot`] JSON, and its `imageHost`, whose `setState(img, state)`
/// publishes an image's load state to the `HTMLImageElement` accessors; see the
/// crate README "Image load state"). Page JS cannot reach it.
pub fn interface_record<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    object: v8::Local<'s, v8::Object>,
) -> Option<v8::Local<'s, v8::Value>> {
    collection_records::get(scope, object)
}

/// The host CSS policy applied to the current document (blocked sheets and
/// inline styles, CSSOM-applied inline overrides), cached per tree version.
pub fn cascade_policy(scope: &mut v8::PinScope) -> StylePolicy {
    css_policy::cascade_policy(scope)
}
