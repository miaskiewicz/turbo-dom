//! Borrow canonical admitted declarations from the exact prepared match owner.
use super::candidates::Candidate;
use super::rule_matches::MatchedRules;
use std::collections::BTreeMap;

pub struct CandidateSet<'a> {
    pub ordinary: BTreeMap<String, Vec<Candidate<'a>>>,
    pub custom: BTreeMap<String, Vec<Candidate<'a>>>,
    pub declared_text_decoration: Vec<Candidate<'a>>,
}
impl<'a> CandidateSet<'a> {
    pub(super) fn push_property(
        &mut self,
        candidate: Candidate<'a>,
        marker: Option<&'a super::CssText>,
    ) {
        if let Some(value) = marker {
            self.declared_text_decoration.push(Candidate {
                property: "text-decoration",
                value,
                pending_source: None,
                ..candidate
            });
        }
        let target = if candidate.property.starts_with("--") {
            &mut self.custom
        } else {
            &mut self.ordinary
        };
        target
            .entry(candidate.property.to_owned())
            .or_default()
            .push(candidate);
    }
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
        declared_text_decoration: Vec::new(),
    };
    for rule_match in matched.rules() {
        let binding = &data.rules[rule_match.prepared_rule()];
        let rule = &compiled.rules[binding.rule_id];
        for declaration in &rule.compiled_declarations {
            let source = &rule.declarations[declaration.source_index];
            for (index, property) in declaration.properties.iter().enumerate() {
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
                let marker = if index == 0 {
                    declaration.declared_text_decoration.as_ref()
                } else {
                    None
                };
                set.push_property(candidate, marker);
            }
        }
    }
    set
}
