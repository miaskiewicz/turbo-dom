//! One native answer set per prepared model. Only the owning adapter can prove
//! that current DOM, sheet and selector inputs still produce these answers.
use super::{
    apply_matching, apply_native_state, fail, parse, Matching, PreparedStyles, ResolveInput,
    ValidatedMatching,
};
use std::collections::{BTreeSet, VecDeque};
use std::mem::size_of;
use std::sync::{Arc, Mutex};
#[path = "css_native_state_cache.rs"]
mod state_cache;
use state_cache::ResultCache;
use turbo_dom_core::rtdom::computed_css::shadow_routes::ContextRoot;

// This cap applies to retention only. Larger default-adapter requests use the
// ordinary complete transport with the caller's original limits.
const MAX_MATCHING_BYTES: usize = 256 * 1024;
const MAX_RESULT_BYTES: usize = 64 * 1024;
const MAX_RESULT_ENTRIES: usize = 8;

fn available_result_bytes(payload_bytes: usize) -> usize {
    MAX_RESULT_BYTES.min(MAX_MATCHING_BYTES.saturating_sub(payload_bytes))
}

struct CachedResult {
    wire: Box<str>,
    response: Box<str>,
}
impl CachedResult {
    fn bytes(&self) -> usize {
        self.wire.len() + self.response.len()
    }
}

#[derive(Default)]
struct SuccessCache {
    entries: VecDeque<CachedResult>,
    bytes: usize,
}
impl SuccessCache {
    fn get(&mut self, wire: &str) -> Option<String> {
        let index = self
            .entries
            .iter()
            .position(|entry| entry.wire.as_ref() == wire)?;
        let entry = self.entries.remove(index)?;
        let response = entry.response.to_string();
        self.entries.push_back(entry);
        Some(response)
    }
    fn remove(&mut self, index: usize) {
        if let Some(entry) = self.entries.remove(index) {
            self.bytes -= entry.bytes();
        }
    }
    fn retain(&mut self, entry: Option<CachedResult>, available: usize) {
        let Some(entry) = entry else {
            self.entries.clear();
            self.bytes = 0;
            return;
        };
        if let Some(index) = self.entries.iter().position(|old| old.wire == entry.wire) {
            self.remove(index);
        }
        let bytes = entry.bytes();
        while self.entries.len() >= MAX_RESULT_ENTRIES || self.bytes + bytes > available {
            self.remove(0);
        }
        self.bytes += bytes;
        self.entries.push_back(entry);
    }
}

struct PayloadCounter {
    domains: BTreeSet<u32>,
    match_ids: usize,
    answers: usize,
}
impl PayloadCounter {
    fn row(&mut self, domain: u32, matches: &[u32]) {
        self.domains.insert(domain);
        self.match_ids += matches.len();
        self.answers += 1;
    }
}
fn payload_bytes(facts: &Matching) -> usize {
    let mut count = PayloadCounter {
        domains: BTreeSet::new(),
        match_ids: 0,
        answers: 0,
    };
    for row in &facts.selector_answers {
        count.row(row.domain_id, &row.matches);
    }
    for row in &facts.scope_answers {
        count.row(row.domain_id, &row.matches);
    }
    if let Some(context) = &facts.context_answers {
        for row in &context.selector_answers {
            count.row(row.domain_id, &row.matches);
        }
        for row in &context.scope_answers {
            count.row(row.domain_id, &row.matches);
        }
    }
    // Answer domains are shared by all answers that reference them. The
    // temporary domain map drops unreferenced domains after validation.
    let retained_domain_ids: usize = facts
        .domains
        .iter()
        .filter(|domain| count.domains.contains(&domain.id))
        .map(|domain| domain.node_ids.len())
        .sum();
    // Charge one scalar key per retained answer. B-tree and allocator overhead
    // are outside the existing payload-only metric.
    (count.match_ids + retained_domain_ids + count.answers) * size_of::<u32>()
        + facts.context_roots.len() * size_of::<ContextRoot>()
}
fn next_token() -> Result<String, String> {
    static LAST: Mutex<u64> = Mutex::new(0);
    let mut last = LAST
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *last = last.checked_add(1).ok_or_else(|| fail("TokenOverflow"))?;
    Ok(last.to_string())
}
pub(crate) struct MatchingSlot {
    facts: ValidatedMatching,
    token: String,
    payload_bytes: usize,
    result: Mutex<ResultCache>,
    #[cfg(test)]
    cascade_calls: std::sync::atomic::AtomicUsize,
    #[cfg(test)]
    parse_calls: std::sync::atomic::AtomicUsize,
}
impl MatchingSlot {
    pub(crate) fn new(wire: &str, limits: &str) -> Result<Self, String> {
        if wire.len() > MAX_MATCHING_BYTES {
            return Err(fail("ResourceLimit"));
        }
        let facts: Matching = parse(wire)?;
        let limits = parse(limits)?;
        let (facts, bytes) = facts.validate(limits, payload_bytes)?;
        let token = next_token()?;
        Ok(Self {
            facts,
            payload_bytes: bytes + token.len(),
            result: Mutex::new(ResultCache::default()),
            #[cfg(test)]
            cascade_calls: std::sync::atomic::AtomicUsize::new(0),
            #[cfg(test)]
            parse_calls: std::sync::atomic::AtomicUsize::new(0),
            token,
        })
    }
    #[cfg(test)]
    pub(crate) fn cascade_calls(&self) -> usize {
        self.cascade_calls
            .load(std::sync::atomic::Ordering::Relaxed)
    }
    #[cfg(test)]
    pub(crate) fn parse_calls(&self) -> usize {
        self.parse_calls.load(std::sync::atomic::Ordering::Relaxed)
    }
    pub(crate) fn token(&self) -> &str {
        &self.token
    }
    pub(crate) fn payload_bytes(&self) -> usize {
        self.payload_bytes
            + self
                .result
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .bytes()
    }
    pub(crate) fn resolve(&self, prepared: &PreparedStyles, wire: &str) -> Result<String, String> {
        // The live owner and current token were checked before this slot.
        // An exact wire retained only after successful validation and cascade
        // certifies these immutable prepared/matching facts and all budgets.
        if let Some(response) = self
            .result
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .wire(wire)
        {
            return Ok(response);
        }
        #[cfg(test)]
        self.parse_calls
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let input: ResolveInput = parse(wire)?;
        input.validate()?;
        #[cfg(test)]
        self.cascade_calls
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let response = apply_matching(prepared, input, &self.facts)?;
        let available = available_result_bytes(self.payload_bytes);
        // Boxed strings retain no caller spare capacity. A success too large
        // for the independent result cap does not expand the matching cap.
        let retained =
            (wire.len().saturating_add(response.len()) <= available).then(|| CachedResult {
                wire: wire.into(),
                response: response.clone().into_boxed_str(),
            });
        self.result
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .retain_wire(retained, available);
        Ok(response)
    }
    pub(crate) fn resolve_state(
        &self,
        prepared: &PreparedStyles,
        wire: &str,
    ) -> Result<Arc<super::native_state::NativeStyleState>, String> {
        if let Some(state) = self
            .result
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .state(wire)
        {
            return Ok(state);
        }
        #[cfg(test)]
        self.parse_calls
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let input: ResolveInput = parse(wire)?;
        input.validate()?;
        #[cfg(test)]
        self.cascade_calls
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let state = Arc::new(apply_native_state(prepared, input, &self.facts)?);
        self.result
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .retain_state(wire, Arc::clone(&state), self.payload_bytes);
        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn matching_slot_remains_send_and_sync() {
        fn check<T: Send + Sync>() {}
        check::<super::MatchingSlot>();
    }
    fn entry(wire: &str, response: &str) -> super::CachedResult {
        super::CachedResult {
            wire: wire.into(),
            response: response.into(),
        }
    }
    #[test]
    fn small_remaining_budget_and_duplicate_accounting_stay_bounded() {
        use super::*;
        assert_eq!(available_result_bytes(0), MAX_RESULT_BYTES);
        assert_eq!(available_result_bytes(MAX_MATCHING_BYTES - 8), 8);
        assert_eq!(available_result_bytes(MAX_MATCHING_BYTES), 0);
        assert_eq!(available_result_bytes(usize::MAX), 0);
        let mut cache = SuccessCache::default();
        cache.retain(Some(entry("a", "aaa")), 8);
        cache.retain(Some(entry("b", "bbb")), 8);
        assert_eq!(cache.bytes, 8);
        assert_eq!(cache.get("a").as_deref(), Some("aaa"));
        cache.retain(Some(entry("c", "cccc")), 8);
        assert_eq!(cache.bytes, 5);
        assert_eq!(cache.entries.len(), 1);
        assert_eq!(cache.get("b"), None);
        cache.retain(Some(entry("c", "ccccc")), 8);
        assert_eq!(cache.bytes, 6);
        assert_eq!(cache.entries.len(), 1);
        cache.retain(None, 0);
        assert_eq!(cache.bytes, 0);
        assert!(cache.entries.is_empty());
    }
    #[test]
    fn entry_storage_capacity_does_not_expand_beyond_fixed_bound() {
        use super::*;
        let mut cache = SuccessCache::default();
        for index in 0..64 {
            let wire = index.to_string();
            cache.retain(Some(entry(&wire, "ok")), MAX_RESULT_BYTES);
            assert!(cache.entries.len() <= MAX_RESULT_ENTRIES);
            assert!(cache.entries.capacity() <= MAX_RESULT_ENTRIES);
            assert_eq!(cache.get(&wire).as_deref(), Some("ok"));
            let sum: usize = cache.entries.iter().map(CachedResult::bytes).sum();
            assert_eq!(cache.bytes, sum);
        }
    }
    #[test]
    fn retained_payload_counts_shared_scope_and_context_domains_once() {
        use super::*;
        let facts: Matching = serde_json::from_value(serde_json::json!({
            "domains": [{"id":0,"nodeIds":[1,2,3]}, {"id":1,"nodeIds":[4,5]}, {"id":2,"nodeIds":[99,100]}],
            "selectorAnswers": [{"selectorId":0,"itemId":0,"rootId":null,"domainId":0,"matches":[2]}],
            "scopeAnswers": [{"groupId":0,"boundary":"start","rootId":1,"domainId":0,"matches":[2,3]}],
            "contextAnswers": {
                "selectorAnswers": [{"treeScope":1,"selectorId":1,"itemId":0,"rootId":null,"domainId":1,"matches":[4]}],
                "scopeAnswers": [{"treeScope":1,"groupId":1,"boundary":"end","rootId":4,"domainId":1,"matches":[5]}]
            },
            "contextRoots": [{"treeScope":0,"root":1,"host":null,"parentTreeScope":null}, {"treeScope":1,"root":4,"host":2,"parentTreeScope":0}]
        })).unwrap();
        assert_eq!(
            payload_bytes(&facts),
            14 * size_of::<u32>() + 2 * size_of::<ContextRoot>()
        );
    }
}
