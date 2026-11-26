//! Request-local tree-scope admission. These are numeric facts, never a DOM mirror.
//! Physical ancestry and entering a host's shadow context are distinct route kinds.
use super::selector_policy::{GeneratedPseudo, SelectorPolicy, SelectorTarget};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContextRoot {
    pub tree_scope: u32,
    pub root: u32,
    pub host: Option<u32>,
    pub parent_tree_scope: Option<u32>,
}
#[derive(Clone, Copy, Debug)]
pub struct AssignedSlot<'a> {
    pub slot: u32,
    pub tree_scope: u32,
    /// Actual slot followed by physical ancestors through its `ShadowRoot`.
    pub path: &'a [u32],
}
#[derive(Clone, Copy)]
pub struct RouteFacts<'a> {
    pub target: u32,
    pub own_tree_scope: u32,
    /// Actual target followed by physical ancestors through its own context root.
    pub own_path: &'a [u32],
    pub host_of: Option<u32>,
    pub assigned_slot: Option<AssignedSlot<'a>>,
    pub context_roots: &'a [ContextRoot],
    pub requested_pseudo: Option<GeneratedPseudo>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RouteError {
    MissingContext(u32),
    InvalidContext,
    InvalidPath,
    UnsupportedComposed,
    ResourceLimit,
}
/// Caller-owned counters may be shared with the surrounding bulk request.
/// Counts include every context record and each physical-path ID retained here.
pub struct RouteBudget {
    pub contexts: usize,
    pub path_units: usize,
}
/// Owned scalar facts and borrowed immutable paths; no mutable source object is
/// retained. Construction authenticates the complete routing input exactly once.
pub struct ValidatedRouteFacts<'a> {
    facts: RouteFacts<'a>,
}
impl<'a> ValidatedRouteFacts<'a> {
    pub fn new(facts: &RouteFacts<'a>, budget: &mut RouteBudget) -> Result<Self, RouteError> {
        let paths = facts
            .own_path
            .len()
            .checked_add(facts.assigned_slot.map_or(0, |slot| slot.path.len()))
            .ok_or(RouteError::ResourceLimit)?;
        let contexts = budget
            .contexts
            .checked_sub(facts.context_roots.len())
            .ok_or(RouteError::ResourceLimit)?;
        let path_units = budget
            .path_units
            .checked_sub(paths)
            .ok_or(RouteError::ResourceLimit)?;
        // Charge atomically before inspecting provenance. Invalid requests do not
        // create a reusable wrapper and cannot evade the surrounding counters.
        budget.contexts = contexts;
        budget.path_units = path_units;
        validate_facts(facts)?;
        Ok(Self { facts: *facts })
    }
    #[must_use]
    pub fn facts(&self) -> &RouteFacts<'a> {
        &self.facts
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RouteKind {
    OwnTreeScope,
    HostOf,
    AssignedSlot,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RoutePath<'a> {
    Physical(&'a [u32]),
    /// A context-boundary anchor, deliberately not a claim about native parenting.
    HostBoundary([u32; 2]),
}
impl RoutePath<'_> {
    #[must_use]
    pub fn ids(&self) -> &[u32] {
        match self {
            Self::Physical(path) => path,
            Self::HostBoundary(path) => path,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MatchRoute<'a> {
    pub kind: RouteKind,
    pub tree_scope: u32,
    pub subject: u32,
    /// Selector origin: the original target, or the original assigned slot.
    pub origin: u32,
    pub path: RoutePath<'a>,
}
fn context(roots: &[ContextRoot], scope: u32) -> Result<&ContextRoot, RouteError> {
    roots
        .iter()
        .find(|root| root.tree_scope == scope)
        .ok_or(RouteError::MissingContext(scope))
}
fn context_shape(root: &ContextRoot) -> bool {
    match (root.host, root.parent_tree_scope) {
        (None, None) => true,
        (Some(host), Some(parent)) => host != root.root && parent != root.tree_scope,
        _ => false,
    }
}
fn validate_contexts(roots: &[ContextRoot]) -> Result<(), RouteError> {
    for (index, root) in roots.iter().enumerate() {
        if !context_shape(root) {
            return Err(RouteError::InvalidContext);
        }
        for other in &roots[..index] {
            if root.tree_scope == other.tree_scope || root.root == other.root {
                return Err(RouteError::InvalidContext);
            }
            if root.host.is_some() && root.host == other.host {
                return Err(RouteError::InvalidContext);
            }
        }
        validate_context_chain(roots, root)?;
    }
    Ok(())
}
fn validate_context_chain<'a>(
    roots: &'a [ContextRoot],
    mut root: &'a ContextRoot,
) -> Result<(), RouteError> {
    for _ in 0..roots.len() {
        let Some(parent) = root.parent_tree_scope else {
            return Ok(());
        };
        root = context(roots, parent)?;
    }
    Err(RouteError::InvalidContext)
}
fn physical_path(path: &[u32], start: u32, root: u32) -> Result<(), RouteError> {
    if path.first() != Some(&start) || path.last() != Some(&root) {
        return Err(RouteError::InvalidPath);
    }
    for (index, id) in path.iter().enumerate() {
        if path[..index].contains(id) {
            return Err(RouteError::InvalidPath);
        }
    }
    Ok(())
}
fn validate_facts(facts: &RouteFacts<'_>) -> Result<(), RouteError> {
    validate_contexts(facts.context_roots)?;
    let own = context(facts.context_roots, facts.own_tree_scope)?;
    physical_path(facts.own_path, facts.target, own.root)?;
    validate_host(facts)?;
    if let Some(slot) = facts.assigned_slot {
        validate_slot(facts, slot)?;
    }
    Ok(())
}
fn validate_host(facts: &RouteFacts<'_>) -> Result<(), RouteError> {
    if let Some(scope) = facts.host_of {
        let shadow = context(facts.context_roots, scope)?;
        if shadow.host != Some(facts.target)
            || shadow.parent_tree_scope != Some(facts.own_tree_scope)
        {
            return Err(RouteError::InvalidContext);
        }
    }
    Ok(())
}
fn validate_slot(facts: &RouteFacts<'_>, slot: AssignedSlot<'_>) -> Result<(), RouteError> {
    let shadow = context(facts.context_roots, slot.tree_scope)?;
    let host = shadow.host.ok_or(RouteError::InvalidContext)?;
    if shadow.parent_tree_scope != Some(facts.own_tree_scope)
        || facts.own_path.get(1) != Some(&host)
    {
        return Err(RouteError::InvalidContext);
    }
    physical_path(slot.path, slot.slot, shadow.root)
}
fn relevant(sheet: u32, facts: &RouteFacts<'_>) -> bool {
    sheet == facts.own_tree_scope
        || facts.host_of == Some(sheet)
        || facts
            .assigned_slot
            .is_some_and(|slot| slot.tree_scope == sheet)
}
fn requested(target: SelectorTarget) -> Result<Option<GeneratedPseudo>, RouteError> {
    match target {
        SelectorTarget::Element => Ok(None),
        SelectorTarget::Generated { pseudo } => Ok(Some(pseudo)),
        SelectorTarget::Slotted { pseudo } => Ok(pseudo),
        SelectorTarget::UnsupportedComposed => Err(RouteError::UnsupportedComposed),
    }
}
fn slot_route<'a>(sheet: u32, facts: &RouteFacts<'a>) -> Option<MatchRoute<'a>> {
    let slot = facts
        .assigned_slot
        .filter(|slot| slot.tree_scope == sheet)?;
    Some(MatchRoute {
        kind: RouteKind::AssignedSlot,
        tree_scope: sheet,
        subject: facts.target,
        origin: slot.slot,
        path: RoutePath::Physical(slot.path),
    })
}
fn element_route<'a>(
    sheet: u32,
    policy: &SelectorPolicy,
    facts: &RouteFacts<'a>,
    scope_anchor: bool,
) -> Result<Option<MatchRoute<'a>>, RouteError> {
    if sheet == facts.own_tree_scope {
        return Ok(Some(MatchRoute {
            kind: RouteKind::OwnTreeScope,
            tree_scope: sheet,
            subject: facts.target,
            origin: facts.target,
            path: RoutePath::Physical(facts.own_path),
        }));
    }
    if !(policy.host_reference || scope_anchor) || facts.host_of != Some(sheet) {
        return Ok(None);
    }
    let root = context(facts.context_roots, sheet)?;
    Ok(Some(MatchRoute {
        kind: RouteKind::HostOf,
        tree_scope: sheet,
        subject: facts.target,
        origin: facts.target,
        path: RoutePath::HostBoundary([facts.target, root.root]),
    }))
}
/// Select admission and mechanical ancestry only. Scope selection/limits/proximity,
/// selector answers and cascade/encapsulation order remain the canonical Rust stages.
/// The caller bounds request facts and authenticates every original native edge.
pub fn selector_route<'a>(
    sheet: u32,
    policy: &SelectorPolicy,
    validated: &ValidatedRouteFacts<'a>,
) -> Result<Option<MatchRoute<'a>>, RouteError> {
    selector_route_with_scope_anchor(sheet, policy, validated, false)
}

pub(crate) fn selector_route_with_scope_anchor<'a>(
    sheet: u32,
    policy: &SelectorPolicy,
    validated: &ValidatedRouteFacts<'a>,
    scope_anchor: bool,
) -> Result<Option<MatchRoute<'a>>, RouteError> {
    let facts = validated.facts();
    context(facts.context_roots, sheet)?;
    if !relevant(sheet, facts) {
        return Ok(None);
    }
    let expected = requested(policy.target)?;
    if expected != facts.requested_pseudo {
        return Ok(None);
    }
    if matches!(policy.target, SelectorTarget::Slotted { .. }) {
        return Ok(slot_route(sheet, facts));
    }
    element_route(sheet, policy, facts, scope_anchor)
}

/// Convenience for a single query. Bulk callers construct `ValidatedRouteFacts` once
/// and call `selector_route` directly for every selector/rule in the request.
pub fn selector_route_checked<'a>(
    sheet: u32,
    policy: &SelectorPolicy,
    facts: &RouteFacts<'a>,
    budget: &mut RouteBudget,
) -> Result<Option<MatchRoute<'a>>, RouteError> {
    selector_route(sheet, policy, &ValidatedRouteFacts::new(facts, budget)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn read_route<'a>(
        sheet: u32,
        policy: &SelectorPolicy,
        facts: &RouteFacts<'a>,
    ) -> Result<Option<MatchRoute<'a>>, RouteError> {
        selector_route_checked(
            sheet,
            policy,
            facts,
            &mut RouteBudget {
                contexts: 3,
                path_units: 32,
            },
        )
    }
    const ROOTS: [ContextRoot; 3] = [
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
        ContextRoot {
            tree_scope: 3,
            root: 30,
            host: Some(21),
            parent_tree_scope: Some(2),
        },
    ];
    fn policy(host: bool, target: SelectorTarget) -> SelectorPolicy {
        SelectorPolicy {
            index: 0,
            host_reference: host,
            target,
            specificity: None,
        }
    }
    fn facts(target: u32, scope: u32, path: &[u32]) -> RouteFacts<'_> {
        RouteFacts {
            target,
            own_tree_scope: scope,
            own_path: path,
            host_of: None,
            assigned_slot: None,
            context_roots: &ROOTS,
            requested_pseudo: None,
        }
    }
    #[test]
    fn ordinary_sheets_cannot_cross_either_shadow_boundary() {
        let input = facts(21, 2, &[21, 20]);
        let normal = policy(false, SelectorTarget::Element);
        let own = read_route(2, &normal, &input).unwrap().unwrap();
        assert_eq!(own.kind, RouteKind::OwnTreeScope);
        assert_eq!(own.path.ids(), &[21, 20]);
        assert_eq!(read_route(1, &normal, &input), Ok(None));
        assert_eq!(read_route(3, &normal, &input), Ok(None));
    }
    #[test]
    fn host_reference_needs_exact_host_of_and_keeps_nonphysical_boundary_typed() {
        let mut input = facts(11, 1, &[11, 10]);
        input.host_of = Some(2);
        let normal = policy(false, SelectorTarget::Element);
        assert_eq!(read_route(2, &normal, &input), Ok(None));
        let host = policy(true, SelectorTarget::Element);
        let route = read_route(2, &host, &input).unwrap().unwrap();
        assert_eq!(route.kind, RouteKind::HostOf);
        assert_eq!(route.subject, 11);
        assert_eq!(route.path, RoutePath::HostBoundary([11, 20]));
        input.host_of = Some(3);
        assert_eq!(
            read_route(3, &host, &input),
            Err(RouteError::InvalidContext)
        );
    }
    #[test]
    fn slotted_subject_and_origin_remain_distinct() {
        let mut input = facts(12, 1, &[12, 11, 10]);
        input.assigned_slot = Some(AssignedSlot {
            slot: 22,
            tree_scope: 2,
            path: &[22, 23, 20],
        });
        let slotted = policy(false, SelectorTarget::Slotted { pseudo: None });
        let route = read_route(2, &slotted, &input).unwrap().unwrap();
        assert_eq!((route.subject, route.origin), (12, 22));
        assert_eq!(route.path, RoutePath::Physical(&[22, 23, 20]));
        assert_eq!(read_route(1, &slotted, &input), Ok(None));
        let normal = policy(false, SelectorTarget::Element);
        assert_eq!(read_route(2, &normal, &input), Ok(None));
        input.assigned_slot = None;
        assert_eq!(read_route(2, &slotted, &input), Ok(None));
    }
    #[test]
    fn generated_pseudo_is_independent_of_origin_routing() {
        let mut input = facts(12, 1, &[12, 11, 10]);
        input.assigned_slot = Some(AssignedSlot {
            slot: 22,
            tree_scope: 2,
            path: &[22, 20],
        });
        let before = policy(
            false,
            SelectorTarget::Slotted {
                pseudo: Some(GeneratedPseudo::Before),
            },
        );
        assert_eq!(read_route(2, &before, &input), Ok(None));
        input.requested_pseudo = Some(GeneratedPseudo::After);
        assert_eq!(read_route(2, &before, &input), Ok(None));
        input.requested_pseudo = Some(GeneratedPseudo::Before);
        assert_eq!(
            read_route(2, &before, &input).unwrap().unwrap().kind,
            RouteKind::AssignedSlot
        );
        let generated = policy(
            false,
            SelectorTarget::Generated {
                pseudo: GeneratedPseudo::Before,
            },
        );
        assert_eq!(
            read_route(1, &generated, &input).unwrap().unwrap().kind,
            RouteKind::OwnTreeScope
        );
        assert_eq!(read_route(2, &generated, &input), Ok(None));
    }
    #[test]
    fn uncertain_contexts_and_paths_are_typed_failures() {
        let normal = policy(false, SelectorTarget::Element);
        let input = facts(11, 1, &[11, 10]);
        assert_eq!(
            read_route(99, &normal, &input),
            Err(RouteError::MissingContext(99))
        );
        for path in [&[10, 11][..], &[11, 11, 10], &[11]] {
            let input = facts(11, 1, path);
            assert_eq!(read_route(1, &normal, &input), Err(RouteError::InvalidPath));
        }
        let duplicate = [ROOTS[0], ROOTS[0]];
        let mut input = facts(11, 1, &[11, 10]);
        input.context_roots = &duplicate;
        assert_eq!(
            read_route(1, &normal, &input),
            Err(RouteError::InvalidContext)
        );
        input.context_roots = &ROOTS;
        input.assigned_slot = Some(AssignedSlot {
            slot: 22,
            tree_scope: 2,
            path: &[22, 20],
        });
        assert_eq!(
            read_route(2, &normal, &input),
            Err(RouteError::InvalidContext)
        );
    }
    #[test]
    fn unsupported_is_an_error_only_for_a_relevant_scope() {
        let input = facts(11, 1, &[11, 10]);
        let unsupported = policy(false, SelectorTarget::UnsupportedComposed);
        assert_eq!(
            read_route(1, &unsupported, &input),
            Err(RouteError::UnsupportedComposed)
        );
        assert_eq!(read_route(2, &unsupported, &input), Ok(None));
    }
    #[test]
    fn validated_bulk_routes_charge_once_and_snapshot_scalar_facts() {
        let mut input = facts(11, 1, &[11, 10]);
        input.host_of = Some(2);
        let mut budget = RouteBudget {
            contexts: 3,
            path_units: 2,
        };
        let validated = ValidatedRouteFacts::new(&input, &mut budget).unwrap();
        assert_eq!((budget.contexts, budget.path_units), (0, 0));
        input.host_of = None;
        assert_eq!(input.host_of, None);
        let host = policy(true, SelectorTarget::Element);
        for _ in 0..10 {
            assert_eq!(
                selector_route(2, &host, &validated).unwrap().unwrap().kind,
                RouteKind::HostOf
            );
        }
        assert_eq!((budget.contexts, budget.path_units), (0, 0));
        assert!(matches!(
            ValidatedRouteFacts::new(&input, &mut budget),
            Err(RouteError::ResourceLimit)
        ));
    }
    #[test]
    fn explicit_context_and_path_counters_enforce_zero_exact_and_cumulative_bounds() {
        let input = facts(12, 1, &[12, 11, 10]);
        for (contexts, paths) in [(0, 3), (2, 3), (3, 0), (3, 2)] {
            let mut budget = RouteBudget {
                contexts,
                path_units: paths,
            };
            assert!(matches!(
                ValidatedRouteFacts::new(&input, &mut budget),
                Err(RouteError::ResourceLimit)
            ));
            assert_eq!((budget.contexts, budget.path_units), (contexts, paths));
        }
        let mut input = facts(12, 1, &[12, 11, 10]);
        input.assigned_slot = Some(AssignedSlot {
            slot: 22,
            tree_scope: 2,
            path: &[22, 20],
        });
        let mut budget = RouteBudget {
            contexts: 6,
            path_units: 10,
        };
        let _first = ValidatedRouteFacts::new(&input, &mut budget).unwrap();
        let _second = ValidatedRouteFacts::new(&input, &mut budget).unwrap();
        assert_eq!((budget.contexts, budget.path_units), (0, 0));
        assert!(matches!(
            ValidatedRouteFacts::new(&input, &mut budget),
            Err(RouteError::ResourceLimit)
        ));
    }
}
