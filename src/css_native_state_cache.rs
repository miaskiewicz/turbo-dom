use super::{available_result_bytes, CachedResult, WireCache, MAX_RESULT_ENTRIES};
use crate::css_resolve::native_state::NativeStyleState;
use std::collections::VecDeque;
use std::sync::Arc;
struct ProjectionEntry {
    wire: Box<str>,
    response: Box<str>,
}
impl ProjectionEntry {
    fn bytes(&self) -> usize {
        self.wire.len().saturating_add(self.response.len())
    }
}
struct StateEntry {
    wire: Box<str>,
    state: Arc<NativeStyleState>,
    bytes: usize,
    projection: Option<ProjectionEntry>,
}
#[derive(Default)]
pub(super) struct StateCache {
    entries: VecDeque<StateEntry>,
    bytes: usize,
}
impl StateCache {
    fn get(&mut self, wire: &str) -> Option<Arc<NativeStyleState>> {
        let index = self
            .entries
            .iter()
            .position(|entry| entry.wire.as_ref() == wire)?;
        let entry = self.entries.remove(index)?;
        let state = Arc::clone(&entry.state);
        self.entries.push_back(entry);
        Some(state)
    }
    fn projection(&self, state: &Arc<NativeStyleState>, wire: &str) -> Option<String> {
        let entry = self
            .entries
            .iter()
            .find(|entry| Arc::ptr_eq(&entry.state, state))?;
        let projection = entry.projection.as_ref()?;
        (projection.wire.as_ref() == wire).then(|| projection.response.to_string())
    }
    fn retain_projection(
        &mut self,
        state: &Arc<NativeStyleState>,
        wire: &str,
        response: &str,
        available: usize,
    ) {
        let Some(entry) = self
            .entries
            .iter_mut()
            .find(|entry| Arc::ptr_eq(&entry.state, state))
        else {
            return;
        };
        let old = entry.projection.as_ref().map_or(0, ProjectionEntry::bytes);
        let bytes = wire.len().saturating_add(response.len());
        let total = self.bytes.saturating_sub(old).saturating_add(bytes);
        // Projection reuse must not evict successful typed requests to make room.
        if total > available {
            return;
        }
        entry.projection = Some(ProjectionEntry {
            wire: wire.into(),
            response: response.into(),
        });
        entry.bytes = entry.bytes.saturating_sub(old).saturating_add(bytes);
        self.bytes = total;
    }
    fn remove(&mut self, index: usize) {
        if let Some(entry) = self.entries.remove(index) {
            self.bytes -= entry.bytes;
        }
    }
    fn retain(&mut self, entry: StateEntry, available: usize) {
        if entry.bytes > available {
            self.entries.clear();
            self.bytes = 0;
            return;
        }
        if let Some(index) = self.entries.iter().position(|old| old.wire == entry.wire) {
            self.remove(index);
        }
        while self.entries.len() >= MAX_RESULT_ENTRIES
            || self.bytes.saturating_add(entry.bytes) > available
        {
            self.remove(0);
        }
        self.bytes += entry.bytes;
        self.entries.push_back(entry);
    }
}
pub(super) enum ResultCache {
    Wire(WireCache),
    State(StateCache),
}
impl Default for ResultCache {
    fn default() -> Self {
        Self::Wire(WireCache::default())
    }
}
impl ResultCache {
    pub(super) fn bytes(&self) -> usize {
        match self {
            Self::Wire(cache) => cache.bytes(),
            Self::State(cache) => cache.bytes,
        }
    }
    pub(super) fn wire(&mut self, wire: &str) -> Option<String> {
        match self {
            Self::Wire(cache) => cache.cache.get(wire),
            Self::State(_) => None,
        }
    }
    pub(super) fn template_bytes(&self) -> usize {
        match self {
            Self::Wire(cache) => cache.template_bytes(),
            Self::State(_) => 0,
        }
    }
    pub(super) fn wire_cache_mut(&mut self) -> Option<&mut WireCache> {
        match self {
            Self::Wire(cache) => Some(cache),
            Self::State(_) => None,
        }
    }
    pub(super) fn ensure_wire(&mut self) -> &mut WireCache {
        if !matches!(self, Self::Wire(_)) {
            *self = Self::Wire(WireCache::default());
        }
        match self {
            Self::Wire(cache) => cache,
            Self::State(_) => unreachable!(),
        }
    }
    pub(super) fn state(&mut self, wire: &str) -> Option<Arc<NativeStyleState>> {
        match self {
            Self::State(cache) => cache.get(wire),
            Self::Wire(_) => None,
        }
    }
    pub(super) fn retain_wire(&mut self, entry: Option<CachedResult>, available: usize) {
        if !matches!(self, Self::Wire(_)) {
            *self = Self::Wire(WireCache::default());
        }
        if let Self::Wire(cache) = self {
            cache.cache.retain(entry, available);
        }
    }
    pub(super) fn projection(&self, state: &Arc<NativeStyleState>, wire: &str) -> Option<String> {
        match self {
            Self::State(cache) => cache.projection(state, wire),
            Self::Wire(_) => None,
        }
    }
    pub(super) fn retain_projection(
        &mut self,
        state: &Arc<NativeStyleState>,
        wire: &str,
        response: &str,
        matching_bytes: usize,
    ) {
        // A late projection cannot recreate retired state or switch cache modes.
        if let Self::State(cache) = self {
            cache.retain_projection(
                state,
                wire,
                response,
                available_result_bytes(matching_bytes),
            );
        }
    }
    pub(super) fn retain_state(
        &mut self,
        wire: &str,
        state: Arc<NativeStyleState>,
        matching_bytes: usize,
    ) {
        let bytes = wire.len().saturating_add(state.payload_bytes());
        let available = available_result_bytes(matching_bytes);
        if bytes > available {
            *self = Self::State(StateCache::default());
            return;
        }
        if !matches!(self, Self::State(_)) {
            *self = Self::State(StateCache::default());
        }
        if let Self::State(cache) = self {
            cache.retain(
                StateEntry {
                    wire: wire.into(),
                    state,
                    bytes,
                    projection: None,
                },
                available,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{NativeStyleState, StateCache, StateEntry};
    use std::{collections::BTreeMap, sync::Arc};
    fn entry(wire: &str, bytes: usize) -> StateEntry {
        StateEntry {
            wire: wire.into(),
            bytes,
            projection: None,
            state: Arc::new(NativeStyleState {
                geometry_only: false,
                order: Box::new([]),
                styles: BTreeMap::new(),
                extras: BTreeMap::new(),
            }),
        }
    }
    #[test]
    fn repeated_success_replaces_exact_entry_without_double_charging() {
        let mut cache = StateCache::default();
        let first = entry("same", 10);
        let old = Arc::clone(&first.state);
        cache.retain(first, 64);
        let next = entry("same", 12);
        let fresh = Arc::clone(&next.state);
        cache.retain(next, 64);
        assert_eq!(cache.entries.len(), 1);
        assert_eq!(cache.bytes, 12);
        let found = cache.get("same").unwrap();
        assert!(Arc::ptr_eq(&fresh, &found));
        assert!(!Arc::ptr_eq(&old, &found));
        assert_eq!(Arc::strong_count(&old), 1);
    }
    #[test]
    fn internal_oversize_success_releases_old_cache_payload() {
        let mut cache = StateCache::default();
        let first = entry("old", 10);
        let old = Arc::clone(&first.state);
        cache.retain(first, 64);
        cache.retain(entry("large", 65), 64);
        assert_eq!(cache.bytes, 0);
        assert!(cache.entries.is_empty());
        assert!(cache.get("old").is_none());
        assert_eq!(Arc::strong_count(&old), 1);
    }
    #[test]
    fn projection_retention_shares_budget_and_does_not_evict_states() {
        let mut cache = StateCache::default();
        let first = entry("a", 10);
        let state = Arc::clone(&first.state);
        cache.retain(first, 24);
        cache.retain(entry("b", 10), 24);
        cache.retain_projection(&state, "p", "123", 24);
        assert_eq!(cache.bytes, 24);
        assert_eq!(cache.projection(&state, "p").as_deref(), Some("123"));
        cache.retain_projection(&state, "q", "longer", 24);
        assert_eq!(cache.bytes, 24);
        assert_eq!(cache.entries.len(), 2);
        assert!(cache.projection(&state, "q").is_none());
        assert_eq!(cache.projection(&state, "p").as_deref(), Some("123"));
        cache.retain_projection(&state, "q", "1", 24);
        assert_eq!(cache.bytes, 22);
        assert!(cache.projection(&state, "p").is_none());
        assert_eq!(cache.projection(&state, "q").as_deref(), Some("1"));
        cache.retain(entry("a", 10), 24);
        assert_eq!(cache.bytes, 20);
        cache.retain_projection(&state, "old", "x", 24);
        assert_eq!(cache.bytes, 20);
        assert!(cache.projection(&state, "old").is_none());
    }
    #[test]
    fn late_projection_cannot_resurrect_state_after_wire_publication() {
        use super::super::{CachedResult, ResultCache};
        let state = entry("x", 10).state;
        let mut cache = ResultCache::default();
        cache.retain_state("x", Arc::clone(&state), 0);
        cache.retain_projection(&state, "p", "v", 0);
        assert_eq!(cache.projection(&state, "p").as_deref(), Some("v"));
        cache.retain_wire(
            Some(CachedResult {
                template: None,
                wire: "wire".into(),
                response: "value".into(),
            }),
            64,
        );
        let bytes = cache.bytes();
        cache.retain_projection(&state, "p", "v", 0);
        assert_eq!(cache.bytes(), bytes);
        assert!(cache.projection(&state, "p").is_none());
        assert_eq!(cache.wire("wire").as_deref(), Some("value"));
    }
}
