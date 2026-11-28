use super::{available_result_bytes, CachedResult, WireCache, MAX_RESULT_ENTRIES};
use crate::css_resolve::native_state::NativeStyleState;
use std::collections::VecDeque;
use std::sync::Arc;
struct StateEntry {
    wire: Box<str>,
    state: Arc<NativeStyleState>,
    bytes: usize,
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
}
