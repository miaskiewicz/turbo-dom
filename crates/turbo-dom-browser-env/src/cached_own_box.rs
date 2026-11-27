//! Cache-only admission for an element's own visual CSS box.
use crate::browser_env::{crate_surface::engine_namespace, DomContextId, DomScope};
use turbo_dom_parser::rtdom::tree::Namespace;

/// A synchronous proof from an already-valid computed-style cache entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CachedOwnBox {
    /// Retain the embedder's normal geometry path and its errors.
    Unknown,
    /// Own `display:none`: bounding/client rects are empty.
    /// This does not grant permission to replace client/scroll/viewport metrics.
    DisplayNone,
}

/// Inspect only a trusted cached own-box proof. Never computes style or layout.
/// The isolate and any attached context queue must use explicit microtask checkpoints.
/// Consume synchronously without guest reentry; this is not a reusable permission.
#[must_use]
pub fn cached_own_box(scope: &mut v8::PinScope, element: v8::Local<v8::Object>) -> CachedOwnBox {
    if scope.get_microtasks_policy() != v8::MicrotasksPolicy::Explicit {
        return CachedOwnBox::Unknown;
    }
    read_cached(scope, element).unwrap_or(CachedOwnBox::Unknown)
}

fn read_cached(scope: &mut v8::PinScope, element: v8::Local<v8::Object>) -> Option<CachedOwnBox> {
    let context = element.get_creation_context(scope)?;
    let scope = &mut v8::ContextScope::new(scope, context);
    let _owner = DomScope::for_object(scope, element);
    let handle = super::handle_of(scope, element)?;
    let owner = context
        .get_slot::<DomContextId>()
        .map_or(0, |owner| owner.0);
    if owner != super::current_document_id() || !super::is_browsing_context() {
        return None;
    }
    if !eligible_handle(handle)? {
        return None;
    }
    probe_cached(scope, element)
}

fn probe_cached(scope: &mut v8::PinScope, element: v8::Local<v8::Object>) -> Option<CachedOwnBox> {
    v8::tc_scope!(let scope, scope);
    let namespace = engine_namespace(scope)?;
    let key = v8::String::new(scope, "peekCachedOwnBox")?;
    let function = namespace.get(scope, key.into())?;
    let function = v8::Local::<v8::Function>::try_from(function).ok()?;
    let result = function.call(scope, namespace.into(), &[element.into()])?;
    Some(if result.is_true() {
        CachedOwnBox::DisplayNone
    } else {
        CachedOwnBox::Unknown
    })
}

fn eligible_handle(handle: super::Handle) -> Option<bool> {
    super::with_tree(|tree| {
        tree.node_type_id(handle) == 1
            && tree.namespace(handle) == Namespace::Html
            && !tree.has_shadow_roots()
            && super::is_connected(tree, handle)
    })
}
