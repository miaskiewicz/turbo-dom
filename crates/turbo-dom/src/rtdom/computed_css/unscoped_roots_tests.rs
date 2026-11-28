use super::*;
use crate::rtdom::computed_css::{
    scope_plan::parse_scope_prelude, stylesheet::GroupKind, ComputationLimits, CssText,
};
use std::cell::Cell;
fn original_scope_roots(
    groups: &[CompiledGroup],
    scope_groups: &[usize],
    path: &[u32],
    owner: Option<u32>,
    answers: BoundaryAnswerSource<'_>,
    remaining: &mut usize,
) -> Result<Vec<usize>, ScopeError> {
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
    let mut read = ScopeRead {
        path,
        answers,
        implicit_root,
        owner_known: owner.is_some(),
        remaining: maximum_decisions,
    };
    let mut parents = vec![path.len() - 1];
    for id in scope_groups {
        let group = groups.get(*id).ok_or(ScopeError::InvalidGroup)?;
        if group.invalid_scope {
            return Ok(Vec::new());
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
    Ok(parents)
}

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
#[test]
fn private_unscoped_roots_do_not_allocate_initial_vector() {
    let answers = BTreeMap::new();
    INITIAL_ROOT_VECTORS.with(|count| count.set(0));
    for owner in [None, Some(8), Some(999)] {
        let mut remaining = 0;
        let roots = scope_roots_with_answer_budget(
            &[],
            &[],
            &[3, 8],
            owner,
            BoundaryAnswerSource::Legacy(&answers),
            &mut remaining,
        )
        .unwrap();
        assert_eq!(roots.as_slice(), &[1]);
        assert_eq!(remaining, 0);
    }
    assert_eq!(INITIAL_ROOT_VECTORS.with(Cell::get), 0);
}
fn compare(
    groups: &[CompiledGroup],
    ids: &[usize],
    path: &[u32],
    owner: Option<u32>,
    answers: BoundaryAnswerSource<'_>,
    budget: usize,
) {
    let mut old_budget = budget;
    let mut new_budget = budget;
    let before = original_scope_roots(groups, ids, path, owner, answers, &mut old_budget);
    let after = scope_roots_with_answer_budget(groups, ids, path, owner, answers, &mut new_budget)
        .map(|roots| roots.as_slice().to_vec());
    assert_eq!(before, after);
    assert_eq!(old_budget, new_budget);
}
#[test]
fn private_root_storage_preserves_paths_owners_budgets_and_errors() {
    let mut invalid = group("(.invalid)");
    invalid.invalid_scope = true;
    let groups = [group(""), group("(.explicit)"), invalid];
    let answers = BTreeMap::new();
    for path in [
        &[][..],
        &[1][..],
        &[10, 11, 12, 13][..],
        &[1, 1][..],
        &[u32::MAX, 0][..],
    ] {
        for owner in [None, Some(1), Some(12), Some(99)] {
            for ids in [
                &[][..],
                &[0][..],
                &[1][..],
                &[2][..],
                &[0, 1][..],
                &[99][..],
            ] {
                for budget in [0, 1, 4, 64] {
                    compare(
                        &groups,
                        ids,
                        path,
                        owner,
                        BoundaryAnswerSource::Legacy(&answers),
                        budget,
                    );
                }
            }
        }
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
fn scoped_roots_retain_nested_contextual_answers_and_remaining_debits() {
    let groups = [group("(.outer)"), group("(:scope .inner) to(.end)")];
    let legacy = BoundaryAnswers::from([
        ((0, Boundary::Start, 13), answer(&[12])),
        ((1, Boundary::Start, 12), answer(&[11])),
        ((1, Boundary::End, 11), answer(&[])),
    ]);
    let context = ContextBoundaryAnswers::from([
        ((7, 0, Boundary::Start, 13), answer(&[12])),
        ((7, 1, Boundary::Start, 12), answer(&[11])),
        ((7, 1, Boundary::End, 11), answer(&[])),
    ]);
    for budget in [0, 1, 4, 64] {
        compare(
            &groups,
            &[0, 1],
            &[10, 11, 12, 13],
            Some(13),
            BoundaryAnswerSource::Legacy(&legacy),
            budget,
        );
        compare(
            &groups,
            &[0, 1],
            &[10, 11, 12, 13],
            Some(13),
            BoundaryAnswerSource::Context(7, &context),
            budget,
        );
        compare(
            &groups,
            &[0, 1],
            &[10, 11, 12, 13],
            Some(13),
            BoundaryAnswerSource::Context(8, &context),
            budget,
        );
    }
}
#[test]
fn public_vec_api_and_implicit_owner_membership_remain_unchanged() {
    let empty = BoundaryAnswers::new();
    assert_eq!(
        scope_roots_with_owner_budget(&[], &[], &[1, 2], None, &empty, &mut 0),
        Ok(vec![1])
    );
    assert_eq!(
        scope_roots_with_owner_budget(&[], &[], &[1, 2], Some(99), &empty, &mut 0),
        Ok(vec![1])
    );
    assert_eq!(
        scope_roots_with_budget(&[], &[], &[1, 2], 99, &empty, &mut 0),
        Err(ScopeError::InvalidPath)
    );
    assert_eq!(
        scope_roots_with_owner_budget(&[], &[], &[1, 1], None, &empty, &mut 0),
        Err(ScopeError::InvalidPath)
    );
    let groups = [group("")];
    assert_eq!(
        scope_roots_with_owner_budget(&groups, &[0], &[1, 2], Some(2), &empty, &mut 8),
        Ok(vec![1])
    );
}
