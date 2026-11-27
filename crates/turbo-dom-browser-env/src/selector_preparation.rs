//! Match-only canonical binding selector state for native cascades.
use crate::browser_env::{DomContextId, DomScope};

/// Why no match predicate could be certified. `Exception` preserves the pending exception.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectorPreparationError {
    InvalidSelector,
    Unavailable,
    Exception,
}

/// Capture provenance, not a standalone cache-reuse permission.
/// Connected checkpoint reuse still requires the host's exact composition and other keys;
/// local/disconnected roots require full native-version validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SelectorStateStamp {
    document: u64,
    root: super::Handle,
    version: u64,
    styles: super::StyleRevisions,
}

/// A current-state match predicate. Retain authored specificity and scope metadata separately.
#[derive(Debug)]
pub struct PreparedSelector {
    predicate: String,
    stamp: SelectorStateStamp,
    volatile: bool,
}

impl PreparedSelector {
    #[must_use]
    pub fn match_selector(&self) -> &str {
        &self.predicate
    }

    #[must_use]
    pub fn stamp(&self) -> SelectorStateStamp {
        self.stamp
    }

    /// Interest/view-transition state has no invalidation epoch. Prepare it afresh on each read.
    /// Stamp/checkpoint equality cannot justify cached targets or a solved view; a fresh complete
    /// projection equality proof may be combined with the host's remaining cache guards.
    #[must_use]
    pub fn is_volatile(&self) -> bool {
        self.volatile
    }
}

/// Prepare a selector for synchronous native matching in a genuine native root's realm.
/// No layout or guest getters run. Consume without guest reentry; fresh captures and all
/// host cache guards remain necessary. This does not change authored specificity.
pub fn prepare_selector(
    scope: &mut v8::PinScope,
    root: v8::Local<v8::Object>,
    source: &str,
) -> Result<PreparedSelector, SelectorPreparationError> {
    with_root(scope, root, |scope, stamp| {
        if !crate::browser_env::cssom_parser::valid_selector(source) {
            return Err(SelectorPreparationError::InvalidSelector);
        }
        v8::tc_scope!(let scope, scope);
        let predicate = crate::browser_env::rewrite_pseudo_selector_impl(
            scope, stamp.root, source, false, true,
        );
        if scope.has_caught() || scope.has_terminated() {
            scope.rethrow();
            return Err(SelectorPreparationError::Exception);
        }
        let predicate = predicate.ok_or(SelectorPreparationError::Unavailable)?;
        if super::with_tree(|tree| tree.version) != Some(stamp.version)
            || super::style_revisions() != stamp.styles
        {
            return Err(SelectorPreparationError::Unavailable);
        }
        Ok(PreparedSelector {
            predicate,
            stamp,
            volatile: crate::browser_env::transform_mask(source) & ((1 << 2) | (1 << 4)) != 0,
        })
    })
}

/// Read fresh provenance for a native selector scope. This token alone grants no reuse.
pub fn selector_state_stamp(
    scope: &mut v8::PinScope,
    root: v8::Local<v8::Object>,
) -> Result<SelectorStateStamp, SelectorPreparationError> {
    with_root(scope, root, |_, stamp| Ok(stamp))
}

fn with_root<R>(
    scope: &mut v8::PinScope,
    root: v8::Local<v8::Object>,
    operation: impl FnOnce(&mut v8::PinScope, SelectorStateStamp) -> Result<R, SelectorPreparationError>,
) -> Result<R, SelectorPreparationError> {
    use SelectorPreparationError::Unavailable;
    if scope.get_microtasks_policy() != v8::MicrotasksPolicy::Explicit {
        return Err(Unavailable);
    }
    let context = root.get_creation_context(scope).ok_or(Unavailable)?;
    let owner = context
        .get_slot::<DomContextId>()
        .map_or(0, |owner| owner.0);
    let scope = &mut v8::ContextScope::new(scope, context);
    let _owner = DomScope::for_object(scope, root);
    let handle = super::handle_of(scope, root).ok_or(Unavailable)?;
    if super::current_document_context(scope) != Some(context)
        || owner != super::current_document_id()
        || !super::is_browsing_context()
    {
        return Err(Unavailable);
    }
    let version = super::with_tree(|tree| {
        matches!(tree.node_type_id(handle), 1 | 9 | 11).then_some(tree.version)
    })
    .flatten()
    .ok_or(Unavailable)?;
    operation(
        scope,
        SelectorStateStamp {
            document: owner,
            root: handle,
            version,
            styles: super::style_revisions(),
        },
    )
}
