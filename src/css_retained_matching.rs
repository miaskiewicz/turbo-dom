//! One native answer set per prepared model. Only the owning adapter can prove
//! that current DOM, sheet and selector inputs still produce these answers.
use super::{
    apply_matching, fail, parse, Matching, PreparedStyles, ResolveInput, ValidatedMatching,
};
use std::collections::BTreeSet;
use std::mem::size_of;
use std::sync::Mutex;
use turbo_dom_core::rtdom::computed_css::shadow_routes::ContextRoot;

// This cap applies to retention only. Larger default-adapter requests use the
// ordinary complete transport with the caller's original limits.
const MAX_MATCHING_BYTES: usize = 256 * 1024;

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
            token,
        })
    }
    pub(crate) fn token(&self) -> &str {
        &self.token
    }
    pub(crate) fn payload_bytes(&self) -> usize {
        self.payload_bytes
    }
    pub(crate) fn resolve(&self, prepared: &PreparedStyles, wire: &str) -> Result<String, String> {
        let input: ResolveInput = parse(wire)?;
        input.validate()?;
        apply_matching(prepared, input, &self.facts)
    }
}
