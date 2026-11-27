//! Rule eligibility and specificity stay in Rust; the adapter supplies selector answers.
use super::container::{
    container_matches_with_measurements, ContainerAncestor, ContainerError, ContainerMeasurements,
};
use super::prepared::{PreparedRule, PreparedStyles};
use super::selector_policy::{
    GeneratedPseudo, SelectorPolicy, SelectorPolicyInspection, SelectorTarget,
};
use super::shadow_routes::{
    selector_route_with_scope_anchor, RouteError, RoutePath, ValidatedRouteFacts,
};
use super::stylesheet::GroupKind;
use super::target_scope::{
    scope_roots_with_answer_budget, BoundaryAnswerSource, BoundaryAnswers, ContextBoundaryAnswers,
    ScopeError,
};
use super::ComputationLimits;
use std::borrow::Cow;
use std::cmp::Reverse;
use std::collections::BTreeMap;

pub use super::facts::MechanicalAnswer as SelectorAnswer;
pub type SelectorAnswers = BTreeMap<(usize, usize, Option<u32>), SelectorAnswer>;
/// Exact stylesheet tree context, selector, item and optional explicit scope root.
pub type ContextSelectorAnswers = BTreeMap<(u32, usize, usize, Option<u32>), SelectorAnswer>;
#[derive(Clone, Copy)]
pub struct ContextAnswers<'a> {
    pub selectors: &'a ContextSelectorAnswers,
    pub boundaries: &'a ContextBoundaryAnswers,
}
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
    context: Option<ContextAnswers<'a>>,
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
struct ScopeContext<'a> {
    path: Cow<'a, [u32]>,
    owner: Option<u32>,
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
        scope_anchor: bool,
    ) -> Result<Option<RouteRead<'facts>>, MatchError> {
        let Some(validated) = self.facts.routes else {
            // rule() already accepted binding_relevant(), which refuses every
            // nonzero scope without provenance before this private path runs.
            return Ok(Some(RouteRead {
                path: RoutePath::Physical(self.facts.path),
            }));
        };
        let route =
            selector_route_with_scope_anchor(binding.tree_scope, policy, validated, scope_anchor)
                .map_err(MatchError::Route)?;
        Ok(route.map(|route| RouteRead { path: route.path }))
    }
    fn scoped_route(
        &self,
        binding: &PreparedRule,
        policy: &SelectorPolicy,
        selector: &crate::rtdom::query::CascadeSelector,
        scoped: bool,
    ) -> Result<Option<RouteRead<'facts>>, MatchError> {
        let anchor = self.context.is_some() && scoped && selector.references_scope();
        let Some(route) = self.route(binding, policy, anchor)? else {
            return Ok(None);
        };
        if scoped && matches!(policy.target, SelectorTarget::Slotted { .. }) {
            return Err(MatchError::UnsupportedScopedSlotted);
        }
        Ok(Some(route))
    }
    fn scope_context<'path>(
        &mut self,
        binding: &PreparedRule,
        route: &'path RoutePath<'_>,
        scoped: bool,
    ) -> Result<ScopeContext<'path>, MatchError> {
        let mut scope = ScopeContext {
            path: Cow::Borrowed(route.ids()),
            owner: self.implicit_owner(binding),
        };
        if self.context.is_none() || !scoped {
            return Ok(scope);
        }
        let Some(routes) = self.facts.routes else {
            return Ok(scope);
        };
        let root = routes
            .facts()
            .context_roots
            .iter()
            .find(|root| root.tree_scope == binding.tree_scope)
            .ok_or(MatchError::Route(RouteError::MissingContext(
                binding.tree_scope,
            )))?;
        let Some(host) = root.host else {
            return Ok(scope);
        };
        if scope.path.last() != Some(&root.root) {
            return Err(MatchError::InvalidRouteFacts);
        }
        if scope.owner == Some(root.root) {
            scope.owner = Some(host);
        }
        if matches!(route, RoutePath::Physical(_)) {
            scope.path = Cow::Owned(self.scope_host_path(scope.path.as_ref(), host)?);
        }
        Ok(scope)
    }
    fn scope_host_path(&mut self, path: &[u32], host: u32) -> Result<Vec<u32>, MatchError> {
        if path.contains(&host) {
            return Err(MatchError::InvalidRouteFacts);
        }
        let length = path.len().checked_add(1).ok_or(MatchError::ResourceLimit)?;
        self.remaining = self
            .remaining
            .checked_sub(length)
            .ok_or(MatchError::ResourceLimit)?;
        let mut scope = Vec::with_capacity(length);
        scope.extend_from_slice(&path[..path.len() - 1]);
        scope.push(host);
        scope.push(path[path.len() - 1]);
        Ok(scope)
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
        // Unchanged policy proves absence of composed selector families. Its
        // native terminal pseudo cannot style a different requested subject.
        let ordinary =
            compiled.selectors[rule.selector_id].policy == SelectorPolicyInspection::Unchanged;
        if (self.facts.routes.is_none() || ordinary)
            && selector.pseudo_element() != self.facts.pseudo
        {
            return Ok(None);
        }
        let policy = item_policy(&compiled.selectors[rule.selector_id], item, selector)?;
        let groups = scope_groups(compiled, &rule.groups);
        let scoped = !groups.is_empty();
        let Some(route) = self.scoped_route(binding, &policy, selector, scoped)? else {
            return Ok(None);
        };
        let scope = self.scope_context(binding, &route.path, scoped)?;
        let path = scope.path.as_ref();
        let roots = scope_roots_with_answer_budget(
            &compiled.groups,
            &groups,
            path,
            scope.owner,
            self.context.map_or(
                BoundaryAnswerSource::Legacy(self.facts.boundaries),
                |answers| BoundaryAnswerSource::Context(binding.tree_scope, answers.boundaries),
            ),
            &mut self.remaining,
        )
        .map_err(|error| match error {
            ScopeError::MissingImplicitRoot => MatchError::MissingImplicitRoot,
            error => MatchError::Scope(error),
        })?;
        let specificity = policy.specificity.unwrap_or_else(|| selector.specificity());
        self.roots(
            (binding.tree_scope, rule.selector_id, item),
            path,
            &roots,
            !groups.is_empty(),
            specificity,
        )
    }
    fn roots(
        &mut self,
        query: (u32, usize, usize),
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
            let key = (query.1, query.2, scoped.then_some(path[*root]));
            let matched = match self.context {
                Some(context) => context
                    .selectors
                    .get(&(query.0, key.0, key.1, key.2))
                    .ok_or(MatchError::MissingSelectorAnswer)?
                    .matches(self.target)
                    .map_err(|_| MatchError::MissingSelectorAnswer)?,
                None => selector_matches(self.facts.selectors, key, self.target)?,
            };
            if matched {
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
    match_with_answers(prepared, facts, maximum_decisions, implicit_policy, None)
}

/// Contextual facts must match the exact prepared binding; legacy maps are ignored.
pub fn matching_rules_with_context_answers(
    prepared: &PreparedStyles,
    facts: &TargetFacts<'_>,
    answers: ContextAnswers<'_>,
    maximum_decisions: usize,
) -> Result<MatchedRules, MatchError> {
    match_with_answers(
        prepared,
        facts,
        maximum_decisions,
        ImplicitRootPolicy::PerStylesheetUse,
        Some(answers),
    )
}

fn match_with_answers(
    prepared: &PreparedStyles,
    facts: &TargetFacts<'_>,
    maximum_decisions: usize,
    implicit_policy: ImplicitRootPolicy,
    context: Option<ContextAnswers<'_>>,
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
        context,
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

#[cfg(test)]
mod scope_context_contracts {
    use super::super::prepared::{CascadeOrigin, LayerRank};
    use super::super::shadow_routes::{ContextRoot, RouteBudget, RouteFacts};
    use super::*;
    #[test]
    fn private_scope_context_rejects_a_nonterminal_root_before_spending_or_allocating() {
        // Defensive helper contract, not an authored-DOM reproduction: the public
        // canonical route already validates its terminal root before this seam.
        let roots = [
            ContextRoot {
                tree_scope: 1,
                root: 10,
                host: None,
                parent_tree_scope: None,
            },
            ContextRoot {
                tree_scope: 2,
                root: 20,
                host: Some(11),
                parent_tree_scope: Some(1),
            },
        ];
        let input = RouteFacts {
            target: 21,
            own_tree_scope: 2,
            own_path: &[21, 20],
            host_of: None,
            assigned_slot: None,
            context_roots: &roots,
            requested_pseudo: None,
        };
        let routes = ValidatedRouteFacts::new(
            &input,
            &mut RouteBudget {
                contexts: 2,
                path_units: 2,
            },
        )
        .unwrap();
        let selectors = BTreeMap::new();
        let boundaries = BTreeMap::new();
        let facts = TargetFacts {
            path: input.own_path,
            implicit_scope_root: 20,
            pseudo: None,
            selectors: &selectors,
            routes: Some(&routes),
            boundaries: &boundaries,
            containers: None,
        };
        let contextual_selectors = BTreeMap::new();
        let contextual_boundaries = BTreeMap::new();
        let mut read = MatchRead {
            facts: &facts,
            target: 21,
            remaining: 64,
            implicit_policy: ImplicitRootPolicy::PerStylesheetUse,
            context: Some(ContextAnswers {
                selectors: &contextual_selectors,
                boundaries: &contextual_boundaries,
            }),
        };
        let binding = PreparedRule {
            rule_id: 0,
            use_id: 0,
            origin: CascadeOrigin::Author,
            tree_scope: 2,
            scope_depth: 1,
            implicit_scope_root: Some(20),
            layer_context: 0,
            layer: None,
            layer_rank: LayerRank {
                normal: std::sync::Arc::default(),
                important: std::sync::Arc::default(),
            },
        };
        assert!(matches!(
            read.scope_context(&binding, &RoutePath::Physical(&[21, 99]), true),
            Err(MatchError::InvalidRouteFacts)
        ));
        assert_eq!(read.remaining, 64);
        assert_eq!(binding.implicit_scope_root, Some(20));
    }
}
