//! The shared computed-style pipeline; native and WASM adapters supply facts only.
use super::candidate_set::collect_candidates;
use super::computed_values::{
    compute_values, ComputedValues, ValueContext, ValueError, ValueLimits,
};
use super::custom_cascade::CustomCascadeLimits;
use super::inline_style::{InlineLimits, InlineStyle};
use super::prepared::{CascadeOrigin, PreparedStyles};
use super::rule_matches::{
    matching_rules, matching_rules_with_context_answers, ContextAnswers, MatchError, MatchedRules,
    TargetFacts,
};
use super::selection::{select_style, SelectionError};
use super::{ComputedCssError, CssText, CustomProperties};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Eq, PartialEq)]
pub struct DeclaredTextDecoration {
    pub value: CssText,
    pub origin: CascadeOrigin,
    pub important: bool,
    pub inline: bool,
    pub order: (usize, usize),
}
pub struct ComputedStyle {
    pub custom: CustomProperties,
    pub values: ComputedValues,
    /// Explicit author/user keys (and inherited explicit keys), kept separate from
    /// the full internal computed map used by the canonical layout peer.
    pub public_keys: BTreeSet<String>,
    /// Winning admitted declared shorthand, not substituted/inherited used values.
    pub declared_text_decoration: Option<DeclaredTextDecoration>,
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

fn declared_marker(
    candidates: &super::candidate_set::CandidateSet<'_>,
    values: &ComputedValues,
    limits: ValueLimits,
) -> Result<Option<DeclaredTextDecoration>, ResolveError> {
    let Some(winner) = candidates
        .declared_text_decoration
        .iter()
        .max_by_key(|candidate| candidate.rank())
    else {
        return Ok(None);
    };
    let units = winner.value.as_units().len();
    let retained = values
        .values
        .values()
        .try_fold(units, |total, value| {
            total.checked_add(value.as_units().len())
        })
        .filter(|total| *total <= limits.retained_units);
    if units > limits.computation.max_output_units || retained.is_none() {
        return Err(ResolveError::Values(ValueError::Computation(
            ComputedCssError::ResourceLimit,
        )));
    }
    super::check_component_limits(winner.value, limits.computation)
        .map_err(|error| ResolveError::Values(ValueError::Computation(error)))?;
    Ok(Some(DeclaredTextDecoration {
        value: winner.value.clone(),
        origin: winner.origin,
        important: winner.important,
        inline: winner.inline,
        order: winner.order,
    }))
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
    finish_style(&matched, facts, inline, parent, context, limits)
}

/// Resolve using exact tree-context answer maps; legacy `TargetFacts` maps are ignored.
pub fn compute_style_with_context_answers(
    prepared: &PreparedStyles,
    facts: &TargetFacts<'_>,
    inline: Option<&CssText>,
    parent: Option<&ComputedStyle>,
    context: &TargetContext<'_>,
    limits: ResolveLimits,
    answers: ContextAnswers<'_>,
) -> Result<ComputedStyle, ResolveError> {
    let matched =
        matching_rules_with_context_answers(prepared, facts, answers, limits.matching_decisions)
            .map_err(ResolveError::Matching)?;
    finish_style(&matched, facts, inline, parent, context, limits)
}

fn finish_style(
    matched: &MatchedRules,
    facts: &TargetFacts<'_>,
    inline: Option<&CssText>,
    parent: Option<&ComputedStyle>,
    context: &TargetContext<'_>,
    limits: ResolveLimits,
) -> Result<ComputedStyle, ResolveError> {
    let prepared = matched.prepared();
    let inline = inline
        .filter(|_| facts.pseudo.is_none())
        .map(|source| InlineStyle::compile(source, limits.inline))
        .transpose()
        .map_err(ResolveError::Inline)?;
    let mut candidates = collect_candidates(matched);
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
    let declared_text_decoration = declared_marker(&candidates, &values, limits.values)?;
    Ok(ComputedStyle {
        declared_text_decoration,
        custom: selected.custom,
        values,
        public_keys,
    })
}
