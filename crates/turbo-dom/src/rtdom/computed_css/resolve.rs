//! The shared computed-style pipeline; native and WASM adapters supply facts only.
use super::candidate_set::collect_candidates;
use super::computed_values::{
    compute_values, ComputedValues, ValueContext, ValueError, ValueLimits,
};
use super::custom_cascade::CustomCascadeLimits;
use super::inline_style::{InlineLimits, InlineStyle};
use super::prepared::{CascadeOrigin, PreparedStyles};
use super::rule_matches::{matching_rules, MatchError, TargetFacts};
use super::selection::{select_style, SelectionError};
use super::{ComputedCssError, CssText, CustomProperties};
use std::collections::{BTreeMap, BTreeSet};

pub struct ComputedStyle {
    pub custom: CustomProperties,
    pub values: ComputedValues,
    /// Explicit author/user keys (and inherited explicit keys), kept separate from
    /// the full internal computed map used by the canonical layout peer.
    pub public_keys: BTreeSet<String>,
}
pub struct TargetContext<'a> {
    pub tree_scope: u32,
    pub scope_depth: u32,
    pub values: ValueContext<'a>,
}
#[derive(Clone, Copy)]
pub struct ResolveLimits {
    pub matching_decisions: usize,
    pub inline: InlineLimits,
    pub custom: CustomCascadeLimits,
    pub values: ValueLimits,
}
#[derive(Debug)]
pub enum ResolveError {
    Matching(MatchError),
    Inline(ComputedCssError),
    Selection(SelectionError),
    Values(ValueError),
}

fn explicit_keys(
    candidates: &super::candidate_set::CandidateSet<'_>,
    parent: Option<&ComputedStyle>,
) -> BTreeSet<String> {
    let mut keys: BTreeSet<_> = candidates
        .ordinary
        .iter()
        .filter(|(_, values)| {
            values
                .iter()
                .any(|value| value.origin != CascadeOrigin::UserAgent)
        })
        .map(|(name, _)| name.clone())
        .collect();
    if let Some(parent) = parent {
        keys.extend(
            parent
                .public_keys
                .iter()
                .filter(|name| super::properties::inherited(name))
                .cloned(),
        );
    }
    keys
}

/// `parent` is an already-computed style, never a CSSOM used-value string map.
/// A pseudo's parent is its originating element; inline declarations do not apply
/// directly to pseudo-element subjects.
pub fn compute_style(
    prepared: &PreparedStyles,
    facts: &TargetFacts<'_>,
    inline: Option<&CssText>,
    parent: Option<&ComputedStyle>,
    context: &TargetContext<'_>,
    limits: ResolveLimits,
) -> Result<ComputedStyle, ResolveError> {
    let matched = matching_rules(prepared, facts, limits.matching_decisions)
        .map_err(ResolveError::Matching)?;
    let inline = inline
        .filter(|_| facts.pseudo.is_none())
        .map(|source| InlineStyle::compile(source, limits.inline))
        .transpose()
        .map_err(ResolveError::Inline)?;
    let mut candidates = collect_candidates(&matched);
    if let Some(inline) = &inline {
        let layer_context = prepared
            .metadata()
            .rules
            .iter()
            .find(|rule| {
                rule.origin == CascadeOrigin::Author && rule.tree_scope == context.tree_scope
            })
            .map_or(usize::MAX, |rule| rule.layer_context);
        inline.append(&mut candidates, context.scope_depth, layer_context);
    }
    let public_keys = explicit_keys(&candidates, parent);
    let empty = BTreeMap::new();
    let parent_custom = parent.map_or(&empty, |parent| parent.custom.as_map());
    let selected = select_style(
        parent_custom,
        &candidates.custom,
        &candidates.ordinary,
        limits.custom,
    )
    .map_err(ResolveError::Selection)?;
    let values = compute_values(
        parent.map(|parent| &parent.values),
        &selected.ordinary,
        &context.values,
        limits.values,
    )
    .map_err(ResolveError::Values)?;
    Ok(ComputedStyle {
        custom: selected.custom,
        values,
        public_keys,
    })
}
