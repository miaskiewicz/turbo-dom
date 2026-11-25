//! Borrow canonical admitted declarations from the exact prepared match owner.
use super::candidates::Candidate;
use super::rule_matches::MatchedRules;
use std::collections::BTreeMap;

pub struct CandidateSet<'a> {
    pub ordinary: BTreeMap<String, Vec<Candidate<'a>>>,
    pub custom: BTreeMap<String, Vec<Candidate<'a>>>,
}
/// No declaration text is reparsed here, and each candidate borrows the model's
/// original payload or its once-expanded static shorthand value.
#[must_use]
pub fn collect_candidates(matched: &MatchedRules) -> CandidateSet<'_> {
    let data = matched.prepared().metadata();
    let compiled = data.compiled.metadata();
    let mut set = CandidateSet {
        ordinary: BTreeMap::new(),
        custom: BTreeMap::new(),
    };
    for rule_match in matched.rules() {
        let binding = &data.rules[rule_match.prepared_rule()];
        let rule = &compiled.rules[binding.rule_id];
        for declaration in &rule.compiled_declarations {
            let source = &rule.declarations[declaration.source_index];
            for property in &declaration.properties {
                let candidate = Candidate {
                    property: &property.name,
                    value: property.value.as_ref().unwrap_or(&source.value),
                    pending_source: property.pending_shorthand.then_some(source.name.as_str()),
                    important: source.important,
                    origin: binding.origin,
                    scope_depth: binding.scope_depth,
                    inline: false,
                    layer_context: binding.layer_context,
                    layer_rank: &binding.layer_rank,
                    specificity: rule_match.specificity(),
                    proximity: rule_match.proximity(),
                    order: (rule_match.prepared_rule(), declaration.source_index),
                };
                let target = if property.name.starts_with("--") {
                    &mut set.custom
                } else {
                    &mut set.ordinary
                };
                target
                    .entry(property.name.clone())
                    .or_default()
                    .push(candidate);
            }
        }
    }
    set
}
