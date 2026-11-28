//! Scope membership consumes exhaustive selector answers over an original ancestor path.
use super::model::CompiledGroup;
use super::scope_plan::{applicable_roots, ScopeFactError, ScopeWitness};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Boundary {
    Start,
    End,
}
pub use super::facts::MechanicalAnswer as BoundaryAnswer;
/// The key is (compiled group ID, boundary kind, original enclosing root ID).
pub type BoundaryAnswers = BTreeMap<(usize, Boundary, u32), BoundaryAnswer>;
/// Exact stylesheet tree context, group, boundary and original enclosing root.
pub type ContextBoundaryAnswers = BTreeMap<(u32, usize, Boundary, u32), BoundaryAnswer>;
#[derive(Clone, Copy)]
pub(crate) enum BoundaryAnswerSource<'a> {
    Legacy(&'a BoundaryAnswers),
    Context(u32, &'a ContextBoundaryAnswers),
}
impl BoundaryAnswerSource<'_> {
    fn get(&self, key: (usize, Boundary, u32)) -> Option<&BoundaryAnswer> {
        match self {
            Self::Legacy(answers) => answers.get(&key),
            Self::Context(tree_scope, answers) => answers.get(&(*tree_scope, key.0, key.1, key.2)),
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScopeError {
    InvalidPath,
    InvalidGroup,
    MissingAnswer,
    InvalidAnswer,
    ResourceLimit,
    MissingImplicitRoot,
}
impl From<ScopeFactError> for ScopeError {
    fn from(error: ScopeFactError) -> Self {
        match error {
            ScopeFactError::InvalidPath => Self::InvalidPath,
            ScopeFactError::ResourceLimit => Self::ResourceLimit,
        }
    }
}

struct ScopeRead<'a> {
    path: &'a [u32],
    answers: BoundaryAnswerSource<'a>,
    implicit_root: Option<usize>,
    owner_known: bool,
    remaining: usize,
}
impl ScopeRead<'_> {
    fn answer(
        &self,
        group: usize,
        boundary: Boundary,
        root: usize,
        node: usize,
    ) -> Result<bool, ScopeError> {
        let answer = self
            .answers
            .get((group, boundary, self.path[root]))
            .ok_or(ScopeError::MissingAnswer)?;
        answer
            .matches(self.path[node])
            .map_err(|_| ScopeError::MissingAnswer)
    }
    fn limit_matches(&mut self, group: usize, root: usize) -> Result<Vec<usize>, ScopeError> {
        let mut matches = Vec::new();
        for node in 0..=root {
            self.charge()?;
            if self.answer(group, Boundary::End, root, node)? {
                matches.push(node);
            }
        }
        Ok(matches)
    }
    fn charge(&mut self) -> Result<(), ScopeError> {
        self.remaining = self
            .remaining
            .checked_sub(1)
            .ok_or(ScopeError::ResourceLimit)?;
        Ok(())
    }
    fn start_matches(
        &self,
        id: usize,
        explicit: bool,
        parent: usize,
        root: usize,
    ) -> Result<bool, ScopeError> {
        if explicit {
            self.answer(id, Boundary::Start, parent, root)
        } else {
            if !self.owner_known {
                return Err(ScopeError::MissingImplicitRoot);
            }
            Ok(Some(root) == self.implicit_root)
        }
    }
    fn witnesses(
        &mut self,
        id: usize,
        group: &CompiledGroup,
        parents: &[usize],
    ) -> Result<Vec<ScopeWitness>, ScopeError> {
        let scope = group.scope.as_ref().ok_or(ScopeError::InvalidGroup)?;
        let mut witnesses = Vec::new();
        for parent in parents {
            for root in 0..=*parent {
                self.charge()?;
                let start = self.start_matches(id, scope.start.is_some(), *parent, root)?;
                if start {
                    let limits = if scope.end.is_some() {
                        self.limit_matches(id, root)?
                    } else {
                        Vec::new()
                    };
                    witnesses.push(ScopeWitness {
                        enclosing_root: *parent,
                        root,
                        start_matches: true,
                        limits,
                    });
                }
            }
        }
        Ok(witnesses)
    }
}

/// Return eligible original roots, nearest first, for a rule's nested scope groups.
/// The path starts with the target and ends at the actual stylesheet tree-scope root.
/// `implicit_root` is the original inline sheet owner's parent, or stylesheet root.
pub fn scope_roots(
    groups: &[CompiledGroup],
    scope_groups: &[usize],
    path: &[u32],
    implicit_root: u32,
    answers: &BoundaryAnswers,
    maximum_decisions: usize,
) -> Result<Vec<usize>, ScopeError> {
    scope_roots_with_budget(groups, scope_groups, path, implicit_root, answers, &mut {
        maximum_decisions
    })
}

/// Share a caller's cumulative decision budget across target/rule reads.
pub fn scope_roots_with_budget(
    groups: &[CompiledGroup],
    scope_groups: &[usize],
    path: &[u32],
    implicit_root: u32,
    answers: &BoundaryAnswers,
    remaining: &mut usize,
) -> Result<Vec<usize>, ScopeError> {
    if !path.contains(&implicit_root) {
        return Err(ScopeError::InvalidPath);
    }
    scope_roots_with_owner_budget(
        groups,
        scope_groups,
        path,
        Some(implicit_root),
        answers,
        remaining,
    )
}

/// A stylesheet-use owner is independent of explicit scope roots. Unknown owner
/// provenance errors only when a relevant implicit group actually consumes it.
pub fn scope_roots_with_owner_budget(
    groups: &[CompiledGroup],
    scope_groups: &[usize],
    path: &[u32],
    owner: Option<u32>,
    answers: &BoundaryAnswers,
    remaining: &mut usize,
) -> Result<Vec<usize>, ScopeError> {
    scope_roots_with_answer_budget(
        groups,
        scope_groups,
        path,
        owner,
        BoundaryAnswerSource::Legacy(answers),
        remaining,
    )
    .map(ScopeRoots::into_vec)
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum ScopeRoots {
    Single(usize),
    Many(Vec<usize>),
}
impl ScopeRoots {
    pub(crate) fn as_slice(&self) -> &[usize] {
        match self {
            Self::Single(root) => std::slice::from_ref(root),
            Self::Many(roots) => roots.as_slice(),
        }
    }
    fn into_vec(self) -> Vec<usize> {
        match self {
            Self::Single(root) => vec![root],
            Self::Many(roots) => roots,
        }
    }
}

pub(crate) fn scope_roots_with_answer_budget(
    groups: &[CompiledGroup],
    scope_groups: &[usize],
    path: &[u32],
    owner: Option<u32>,
    answers: BoundaryAnswerSource<'_>,
    remaining: &mut usize,
) -> Result<ScopeRoots, ScopeError> {
    let maximum_decisions = *remaining;
    if path.is_empty() {
        return Err(ScopeError::InvalidPath);
    }
    let implicit_root = owner.and_then(|owner| path.iter().position(|node| *node == owner));
    if path
        .iter()
        .enumerate()
        .any(|(index, node)| path[..index].contains(node))
    {
        return Err(ScopeError::InvalidPath);
    }
    if scope_groups.is_empty() {
        return Ok(ScopeRoots::Single(path.len() - 1));
    }
    let read = ScopeRead {
        path,
        answers,
        implicit_root,
        owner_known: owner.is_some(),
        remaining: maximum_decisions,
    };
    scoped_roots_with_read(groups, scope_groups, read, remaining)
}
fn scoped_roots_with_read(
    groups: &[CompiledGroup],
    scope_groups: &[usize],
    mut read: ScopeRead<'_>,
    remaining: &mut usize,
) -> Result<ScopeRoots, ScopeError> {
    let maximum_decisions = read.remaining;
    let path = read.path;
    #[cfg(test)]
    INITIAL_ROOT_VECTORS.with(|count| count.set(count.get() + 1));
    let mut parents = vec![path.len() - 1];
    for id in scope_groups {
        let group = groups.get(*id).ok_or(ScopeError::InvalidGroup)?;
        if group.invalid_scope {
            return Ok(ScopeRoots::Many(Vec::new()));
        }
        let witnesses = read.witnesses(*id, group, &parents)?;
        parents = applicable_roots(path.len(), &parents, &witnesses, maximum_decisions)?
            .into_iter()
            .map(|(_, root)| root)
            .collect();
        parents.sort_unstable();
        parents.dedup();
        if parents.is_empty() {
            break;
        }
    }
    *remaining = read.remaining;
    Ok(ScopeRoots::Many(parents))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rtdom::computed_css::{
        scope_plan::parse_scope_prelude, stylesheet::GroupKind, ComputationLimits, CssText,
    };
    fn group(source: &str) -> CompiledGroup {
        let prelude = CssText::from_units(source.encode_utf16().collect());
        let scope = parse_scope_prelude(
            &prelude,
            ComputationLimits {
                max_nesting: 16,
                max_output_units: 4096,
                max_total_units: 8192,
            },
        )
        .unwrap();
        CompiledGroup {
            kind: GroupKind::Scope,
            prelude,
            parent: None,
            scope,
            invalid_scope: false,
            scope_start: None,
            scope_end: None,
            scope_start_policy: None,
            scope_end_policy: None,
        }
    }
    fn answer(matches: &[u32]) -> BoundaryAnswer {
        BoundaryAnswer::new(
            super::super::facts::AnswerDomain::new(vec![10, 11, 12, 13]).unwrap(),
            matches.to_vec(),
        )
        .unwrap()
    }
    #[test]
    fn nearest_scope_roots_and_limits_are_chosen_by_rust_from_exhaustive_answers() {
        let groups = vec![group("(.root) to(.limit)")];
        let mut answers = BTreeMap::from([
            ((0, Boundary::Start, 13), answer(&[11, 12])),
            ((0, Boundary::End, 11), answer(&[])),
            ((0, Boundary::End, 12), answer(&[])),
        ]);
        let path = [10, 11, 12, 13];
        assert_eq!(
            scope_roots(&groups, &[0], &path, 13, &answers, 64).unwrap(),
            vec![1, 2]
        );
        answers.insert((0, Boundary::End, 11), answer(&[11]));
        assert_eq!(
            scope_roots(&groups, &[0], &path, 13, &answers, 64).unwrap(),
            vec![2]
        );
        answers.insert((0, Boundary::End, 12), answer(&[10]));
        assert!(scope_roots(&groups, &[0], &path, 13, &answers, 64)
            .unwrap()
            .is_empty());
    }
    #[test]
    fn nested_root_start_answers_are_anchored_to_the_actual_enclosing_root() {
        let groups = vec![group("(.outer)"), group("(:scope .inner)")];
        let answers = BTreeMap::from([
            ((0, Boundary::Start, 13), answer(&[12])),
            ((1, Boundary::Start, 12), answer(&[11])),
        ]);
        assert_eq!(
            scope_roots(&groups, &[0, 1], &[10, 11, 12, 13], 13, &answers, 64).unwrap(),
            vec![1]
        );
        assert_eq!(
            scope_roots(
                &groups,
                &[0, 1],
                &[10, 11, 12, 13],
                13,
                &BTreeMap::new(),
                64
            ),
            Err(ScopeError::MissingAnswer)
        );
    }
    #[test]
    fn missing_and_malformed_facts_remain_errors_and_all_decisions_are_bounded() {
        let groups = vec![group("")];
        let path = [10, 11, 12, 13];
        let empty = BTreeMap::new();
        assert_eq!(
            scope_roots(&groups, &[0], &path, 12, &empty, 4).unwrap(),
            vec![2]
        );
        assert_eq!(
            scope_roots(&groups, &[0], &path, 12, &empty, 0),
            Err(ScopeError::ResourceLimit)
        );
        assert_eq!(
            scope_roots(&groups, &[1], &path, 12, &empty, 4),
            Err(ScopeError::InvalidGroup)
        );
        assert_eq!(
            scope_roots(&groups, &[], &[10, 10], 10, &empty, 4),
            Err(ScopeError::InvalidPath)
        );
        assert_eq!(
            scope_roots(&groups, &[], &path, 99, &empty, 4),
            Err(ScopeError::InvalidPath)
        );
        let domain = super::super::facts::AnswerDomain::new(vec![10]).unwrap();
        assert!(matches!(
            BoundaryAnswer::new(domain, vec![99]),
            Err(super::super::facts::FactError::OutsideDomain)
        ));
    }
    #[test]
    fn contextual_boundaries_never_reuse_another_sheet_context_or_legacy_answers() {
        let groups = vec![group("(.root) to(.limit)")];
        let path = [10, 11, 12, 13];
        let mut answers = ContextBoundaryAnswers::from([
            ((7, 0, Boundary::Start, 13), answer(&[11])),
            ((7, 0, Boundary::End, 11), answer(&[])),
            ((9, 0, Boundary::Start, 13), answer(&[11])),
            ((9, 0, Boundary::End, 11), answer(&[10])),
        ]);
        let read = |context, answers: &ContextBoundaryAnswers| {
            scope_roots_with_answer_budget(
                &groups,
                &[0],
                &path,
                Some(13),
                BoundaryAnswerSource::Context(context, answers),
                &mut 64,
            )
        };
        assert_eq!(read(7, &answers).unwrap().as_slice(), &[1]);
        assert!(read(9, &answers).unwrap().as_slice().is_empty());
        assert_eq!(read(8, &answers), Err(ScopeError::MissingAnswer));
        answers.insert((7, 0, Boundary::Start, 13), answer(&[]));
        assert!(read(7, &answers).unwrap().as_slice().is_empty());
        answers.insert(
            (7, 0, Boundary::Start, 13),
            BoundaryAnswer::new(
                super::super::facts::AnswerDomain::new(vec![10]).unwrap(),
                vec![],
            )
            .unwrap(),
        );
        assert_eq!(read(7, &answers), Err(ScopeError::MissingAnswer));
    }
}

#[cfg(test)]
thread_local! { static INITIAL_ROOT_VECTORS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }
#[cfg(test)]
#[path = "unscoped_roots_tests.rs"]
mod unscoped_tests;
