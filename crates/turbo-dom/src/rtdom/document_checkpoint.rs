//! Conservative connected-document equivalence, independent of storage provenance.
use super::tree::{Handle, NodeType, Tree};

/// Opaque evidence that one native tree's connected document has not changed.
/// This does not validate stylesheets, embedder state or an old live capture.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DocumentCheckpoint {
    incarnation: u64,
    generation: u64,
}

pub(super) struct CheckpointState {
    checkpoint: DocumentCheckpoint,
    accounted_version: u64,
}

impl CheckpointState {
    pub(super) fn new(version: u64) -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        Self {
            checkpoint: DocumentCheckpoint {
                incarnation: NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                generation: 0,
            },
            accounted_version: version,
        }
    }
}

impl Tree {
    /// A connected-document checkpoint, or `None` for an unaccounted version or
    /// shadow-bearing tree. Full [`Self::version`] checks still guard live reads.
    #[must_use]
    pub fn document_checkpoint(&self) -> Option<DocumentCheckpoint> {
        (self.checkpoint.accounted_version == self.version
            && !self.has_shadow_roots()
            && self.parent(self.root()).is_none())
        .then_some(self.checkpoint.checkpoint)
    }

    /// Compare only connected native inputs. All embedder/cascade keys remain required.
    #[must_use]
    pub fn document_unchanged_since(&self, checkpoint: DocumentCheckpoint) -> bool {
        self.document_checkpoint() == Some(checkpoint)
    }

    pub(super) fn invalidate_document_checkpoint(&mut self) {
        self.checkpoint.checkpoint.generation += 1;
    }

    pub(super) fn advance_version(&mut self, connected: bool) {
        if connected || self.checkpoint.accounted_version != self.version {
            self.invalidate_document_checkpoint();
        }
        self.version += 1;
        self.checkpoint.accounted_version = self.version;
    }

    pub(super) fn proven_detached(&self, h: Handle) -> bool {
        if !self.is_live(h) || self.has_shadow_roots() || self.parent(self.root()).is_some() {
            return false;
        }
        let mut root = h;
        while let Some(parent) = self.parent(root) {
            root = parent;
        }
        root != self.root() && self.node_type(root) != NodeType::Document
    }

    pub(super) fn bump_for_node(&mut self, h: Handle) {
        self.advance_version(!self.proven_detached(h));
        self.selector_version += 1;
    }

    pub(super) fn bump_detached(&mut self) {
        self.advance_version(false);
        self.selector_version += 1;
    }
}
