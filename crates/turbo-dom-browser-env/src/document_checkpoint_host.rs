//! Native document-input equivalence for host-owned solved-layout caches.
use crate::browser_env::{DomContextId, DomScope};

/// Opaque identity of unchanged connected native document inputs.
/// CSSOM, host state, resources, viewport and composition remain separate cache keys.
/// This never authorizes using a stale live capture for a fresh layout.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DocumentCheckpoint {
    document: u64,
    native: turbo_dom_parser::rtdom::DocumentCheckpoint,
}

/// Read an exact native Document's checkpoint, without invoking guest code or layout.
/// Compare fresh `Some` tokens only; `None` requires normal full-version handling.
/// The isolate must use explicit microtask checkpoints. Unproven shadow graphs, inert, retired,
/// foreign-owner and unaccounted native-version states are conservatively refused.
#[must_use]
pub fn document_checkpoint(
    scope: &mut v8::PinScope,
    document: v8::Local<v8::Object>,
) -> Option<DocumentCheckpoint> {
    if scope.get_microtasks_policy() != v8::MicrotasksPolicy::Explicit {
        return None;
    }
    let context = document.get_creation_context(scope)?;
    let owner = context
        .get_slot::<DomContextId>()
        .map_or(0, |owner| owner.0);
    let scope = &mut v8::ContextScope::new(scope, context);
    let _owner = DomScope::for_object(scope, document);
    let handle = super::handle_of(scope, document)?;
    if super::current_document_context(scope)? != context {
        return None;
    }
    if owner != super::current_document_id() || !super::is_browsing_context() {
        return None;
    }
    super::with_tree(|tree| {
        if handle != tree.root() || tree.node_type_id(handle) != 9 {
            return None;
        }
        Some(DocumentCheckpoint {
            document: owner,
            native: tree.document_checkpoint()?,
        })
    })?
}
