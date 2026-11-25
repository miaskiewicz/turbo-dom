//! Rule eligibility and specificity stay in Rust; the adapter supplies selector answers.
use super::container::{
    container_matches_with_measurements, ContainerAncestor, ContainerError, ContainerMeasurements,
};
use super::prepared::{PreparedRule, PreparedStyles};
use super::selector_policy::{
    GeneratedPseudo, SelectorPolicy, SelectorPolicyInspection, SelectorTarget,
};
use super::shadow_routes::{selector_route, RouteError, RoutePath, ValidatedRouteFacts};
use super::stylesheet::GroupKind;
use super::target_scope::{scope_roots_with_owner_budget, BoundaryAnswers, ScopeError};
use super::ComputationLimits;
use std::cmp::Reverse;
use std::collections::BTreeMap;

pub use super::facts::MechanicalAnswer as SelectorAnswer;
pub type SelectorAnswers = BTreeMap<(usize, usize, Option<u32>), SelectorAnswer>;
/// Actual current content boxes and font metrics from the renderer, nearest-first.
/// No measurement is inferred from authored CSS or synthetic DOM geometry.
#[derive(Clone, Copy)]
pub struct ContainerMatchContext<'a> {
    pub ancestors: &'a [ContainerAncestor<'a>],
    pub measurements: Option<&'a ContainerMeasurements>,
    pub limits: ComputationLimits,
}
pub struct TargetFacts<'a> {
    /// Original target followed by its actual ancestors through the tree-scope root.
    pub path: &'a [u32],
    pub implicit_scope_root: u32,
    pub pseudo: Option<&'a str>,
    pub selectors: &'a SelectorAnswers,
    pub routes: Option<&'a ValidatedRouteFacts<'a>>,
    pub boundaries: &'a BoundaryAnswers,
    pub containers: Option<ContainerMatchContext<'a>>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MatchError {
    Scope(ScopeError),
    MissingSelectorAnswer,
    InvalidSelectorAnswer,
    UnsupportedSelector(usize),
    RequiresContainer(usize),
    Container(ContainerError),
    ResourceLimit,
    MissingRouteFacts(u32),
    InvalidRouteFacts,
    Route(RouteError),
    MissingImplicitRoot,
    UnsupportedScopedSlotted,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuleMatch {
    prepared_rule: usize,
    specificity: (u32, u32, u32),
    proximity: usize,
}
/// Match results retain their exact immutable CSS owner, preventing reuse against a
/// different prepared model even when numeric rule IDs happen to coincide.
pub struct MatchedRules {
    prepared: PreparedStyles,
    rules: Vec<RuleMatch>,
}
impl MatchedRules {
    #[must_use]
    pub fn prepared(&self) -> &PreparedStyles {
        &self.prepared
    }
    #[must_use]
    pub fn rules(&self) -> &[RuleMatch] {
        &self.rules
    }
}
impl RuleMatch {
    #[must_use]
    pub fn prepared_rule(&self) -> usize {
        self.prepared_rule
    }
    #[must_use]
    pub fn specificity(&self) -> (u32, u32, u32) {
        self.specificity
    }
    #[must_use]
    pub fn proximity(&self) -> usize {
        self.proximity
    }
}
fn selector_matches(
    answers: &SelectorAnswers,
    key: (usize, usize, Option<u32>),
    target: u32,
) -> Result<bool, MatchError> {
    let answer = answers.get(&key).ok_or(MatchError::MissingSelectorAnswer)?;
    // MechanicalAnswer validates construction; matches() can fail only when
    // the requested original target is outside its exhaustive answer domain.
    answer
        .matches(target)
        .map_err(|_| MatchError::MissingSelectorAnswer)
}

type MatchRank = ((u32, u32, u32), Reverse<usize>);
struct MatchRead<'a, 'facts> {
    facts: &'a TargetFacts<'facts>,
    target: u32,
    remaining: usize,
    implicit_policy: ImplicitRootPolicy,
}
fn generated(name: Option<&str>) -> Result<Option<GeneratedPseudo>, MatchError> {
    match name {
        None => Ok(None),
        Some("before") => Ok(Some(GeneratedPseudo::Before)),
        Some("after") => Ok(Some(GeneratedPseudo::After)),
        _ => Err(MatchError::InvalidRouteFacts),
    }
}
fn item_policy(
    metadata: &super::model::MatchSelector,
    item: usize,
    selector: &crate::rtdom::query::CascadeSelector,
) -> Result<SelectorPolicy, MatchError> {
    if let SelectorPolicyInspection::Items(items) = &metadata.policy {
        return items
            .iter()
            .find(|policy| policy.index == item)
            .cloned()
            .ok_or(MatchError::InvalidSelectorAnswer);
    }
    let target = match generated(selector.pseudo_element()) {
        Ok(None) => SelectorTarget::Element,
        Ok(Some(pseudo)) => SelectorTarget::Generated { pseudo },
        Err(_) => SelectorTarget::UnsupportedComposed,
    };
    Ok(SelectorPolicy {
        index: item,
        host_reference: false,
        target,
        specificity: None,
    })
}
struct RouteRead<'a> {
    path: RoutePath<'a>,
}
impl<'facts> MatchRead<'_, 'facts> {
    fn implicit_owner(&self, binding: &PreparedRule) -> Option<u32> {
        binding.implicit_scope_root.or(match self.implicit_policy {
            ImplicitRootPolicy::PerStylesheetUse => None,
            ImplicitRootPolicy::LegacyTarget => Some(self.facts.implicit_scope_root),
        })
    }
    fn route(
        &self,
        binding: &PreparedRule,
        policy: &SelectorPolicy,
    ) -> Result<Option<RouteRead<'facts>>, MatchError> {
        let Some(validated) = self.facts.routes else {
            // rule() already accepted binding_relevant(), which refuses every
            // nonzero scope without provenance before this private path runs.
            return Ok(Some(RouteRead {
                path: RoutePath::Physical(self.facts.path),
            }));
        };
        let route =
            selector_route(binding.tree_scope, policy, validated).map_err(MatchError::Route)?;
        Ok(route.map(|route| RouteRead { path: route.path }))
    }
    fn best(
        &mut self,
        compiled: &super::model::CompiledData,
        rule: &super::model::CompiledRule,
        binding: &PreparedRule,
        list: &crate::rtdom::query::SelectorList,
    ) -> Result<Option<MatchRank>, MatchError> {
        let mut best = None;
        for (item, selector) in list.selectors().iter().enumerate() {
            if let Some(rank) = self.item(compiled, rule, binding, item, selector)? {
                best = Some(best.map_or(rank, |old: MatchRank| old.max(rank)));
            }
        }
        Ok(best)
    }
    fn item(
        &mut self,
        compiled: &super::model::CompiledData,
        rule: &super::model::CompiledRule,
        binding: &PreparedRule,
        item: usize,
        selector: &crate::rtdom::query::CascadeSelector,
    ) -> Result<Option<MatchRank>, MatchError> {
        // Scope-zero callers retain the existing additional pseudo-name contract.
        if self.facts.routes.is_none() && selector.pseudo_element() != self.facts.pseudo {
            return Ok(None);
        }
        let policy = item_policy(&compiled.selectors[rule.selector_id], item, selector)?;
        let Some(route) = self.route(binding, &policy)? else {
            return Ok(None);
        };
        let groups = scope_groups(compiled, &rule.groups);
        if !groups.is_empty() && matches!(policy.target, SelectorTarget::Slotted { .. }) {
            return Err(MatchError::UnsupportedScopedSlotted);
        }
        let path = route.path.ids();
        let roots = scope_roots_with_owner_budget(
            &compiled.groups,
            &groups,
            path,
            self.implicit_owner(binding),
            self.facts.boundaries,
            &mut self.remaining,
        )
        .map_err(|error| match error {
            ScopeError::MissingImplicitRoot => MatchError::MissingImplicitRoot,
            error => MatchError::Scope(error),
        })?;
        let specificity = policy.specificity.unwrap_or_else(|| selector.specificity());
        self.roots(
            rule.selector_id,
            item,
            path,
            &roots,
            !groups.is_empty(),
            specificity,
        )
    }
    fn roots(
        &mut self,
        id: usize,
        item: usize,
        path: &[u32],
        roots: &[usize],
        scoped: bool,
        specificity: (u32, u32, u32),
    ) -> Result<Option<MatchRank>, MatchError> {
        let mut best = None;
        for root in roots {
            self.remaining = self
                .remaining
                .checked_sub(1)
                .ok_or(MatchError::ResourceLimit)?;
            let key = (id, item, scoped.then_some(path[*root]));
            if selector_matches(self.facts.selectors, key, self.target)? {
                let proximity = if scoped { *root } else { usize::MAX };
                let rank = (specificity, Reverse(proximity));
                best = Some(best.map_or(rank, |old: MatchRank| old.max(rank)));
            }
        }
        Ok(best)
    }
    fn binding_relevant(&self, binding: &PreparedRule) -> Result<bool, MatchError> {
        let Some(routes) = self.facts.routes else {
            if binding.tree_scope != 0 {
                return Err(MatchError::MissingRouteFacts(binding.tree_scope));
            }
            return Ok(true);
        };
        let facts = routes.facts();
        Ok(binding.tree_scope == facts.own_tree_scope
            || facts.host_of == Some(binding.tree_scope)
            || facts
                .assigned_slot
                .is_some_and(|slot| slot.tree_scope == binding.tree_scope))
    }
    fn rule(
        &mut self,
        compiled: &super::model::CompiledData,
        rule: &super::model::CompiledRule,
        binding: &PreparedRule,
    ) -> Result<Option<MatchRank>, MatchError> {
        if !self.binding_relevant(binding)? {
            return Ok(None);
        }
        let selector = &compiled.selectors[rule.selector_id];
        if selector.policy == SelectorPolicyInspection::Invalid {
            return Ok(None);
        }
        let list = selector
            .compiled
            .as_ref()
            .map_err(|_| MatchError::UnsupportedSelector(rule.selector_id))?;
        let best = self.best(compiled, rule, binding, list)?;
        if best.is_some() && !self.containers(compiled, &rule.groups)? {
            return Ok(None);
        }
        Ok(best)
    }
    fn containers(
        &mut self,
        compiled: &super::model::CompiledData,
        groups: &[usize],
    ) -> Result<bool, MatchError> {
        for id in groups {
            let group = &compiled.groups[*id];
            if group.kind != GroupKind::Container {
                continue;
            }
            let context = self
                .facts
                .containers
                .ok_or(MatchError::RequiresContainer(*id))?;
            self.remaining = self
                .remaining
                .checked_sub(1)
                .ok_or(MatchError::ResourceLimit)?;
            match container_matches_with_measurements(
                &group.prelude,
                context.ancestors,
                context.measurements,
                context.limits,
            ) {
                Ok(value) if value.matches => {}
                Ok(_) | Err(ContainerError::InvalidSyntax) => return Ok(false),
                Err(error) => return Err(MatchError::Container(error)),
            }
        }
        Ok(true)
    }
}
fn validate_route_facts(facts: &TargetFacts<'_>) -> Result<(), MatchError> {
    if let Some(routes) = facts.routes {
        let route = routes.facts();
        if route.own_path != facts.path || route.requested_pseudo != generated(facts.pseudo)? {
            return Err(MatchError::InvalidRouteFacts);
        }
    }
    Ok(())
}
fn scope_groups(compiled: &super::model::CompiledData, groups: &[usize]) -> Vec<usize> {
    groups
        .iter()
        .copied()
        .filter(|id| compiled.groups[*id].kind == GroupKind::Scope)
        .collect()
}
/// Match one target against an immutable prepared model. Container conditions are
/// evaluated only after a relevant selector matches, using actual renderer facts.
pub fn matching_rules(
    prepared: &PreparedStyles,
    facts: &TargetFacts<'_>,
    maximum_decisions: usize,
) -> Result<MatchedRules, MatchError> {
    matching_rules_with_implicit_policy(
        prepared,
        facts,
        maximum_decisions,
        ImplicitRootPolicy::PerStylesheetUse,
    )
}

#[derive(Clone, Copy)]
pub enum ImplicitRootPolicy {
    PerStylesheetUse,
    LegacyTarget,
}
/// Legacy target ownership requires explicit caller opt-in; the default runtime
/// path never substitutes a document/target root for absent stylesheet provenance.
pub fn matching_rules_with_implicit_policy(
    prepared: &PreparedStyles,
    facts: &TargetFacts<'_>,
    maximum_decisions: usize,
    implicit_policy: ImplicitRootPolicy,
) -> Result<MatchedRules, MatchError> {
    validate_route_facts(facts)?;
    let data = prepared.metadata();
    let compiled = data.compiled.metadata();
    let target = *facts
        .path
        .first()
        .ok_or(MatchError::Scope(ScopeError::InvalidPath))?;
    let mut read = MatchRead {
        facts,
        target,
        remaining: maximum_decisions,
        implicit_policy,
    };
    let mut matches = Vec::new();
    for (id, binding) in data.rules.iter().enumerate() {
        if !prepared.includes_rule(id) {
            continue;
        }
        if let Some((specificity, Reverse(proximity))) =
            read.rule(compiled, &compiled.rules[binding.rule_id], binding)?
        {
            matches.push(RuleMatch {
                prepared_rule: id,
                specificity,
                proximity,
            });
        }
    }
    Ok(MatchedRules {
        prepared: prepared.clone(),
        rules: matches,
    })
}
