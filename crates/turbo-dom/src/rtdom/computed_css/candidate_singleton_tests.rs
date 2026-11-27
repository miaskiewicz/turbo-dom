use super::*;
use std::cell::Cell;
use std::sync::Arc;

fn original_select<'a, T, E, F>(
    candidates: &'a [Candidate<'a>],
    restrictions: &[Rollback<'a>],
    mut evaluate: F,
) -> Result<Option<(&'a Candidate<'a>, T)>, E>
where
    F: FnMut(&Candidate<'a>) -> Result<Evaluation<T>, E>,
{
    let mut ordered: Vec<_> = candidates.iter().collect();
    ordered.sort_by(|left, right| right.rank().cmp(&left.rank()));
    let mut rollback = Vec::new();
    for candidate in ordered {
        if restrictions
            .iter()
            .chain(rollback.iter())
            .any(|rule: &Rollback<'_>| rule.excludes(candidate))
        {
            continue;
        }
        let evaluation = evaluate(candidate)?;
        match evaluation.keyword {
            Some(WideKeyword::Revert) => {
                rollback.push(Rollback::Origin(candidate.origin.normal_order()));
            }
            Some(WideKeyword::RevertLayer) => rollback.push(Rollback::Layer {
                origin: candidate.origin,
                context: candidate.layer_context,
                inline: candidate.inline,
                normal: &candidate.layer_rank.normal,
            }),
            _ => return Ok(Some((candidate, evaluation.value))),
        }
    }
    Ok(None)
}

fn layer() -> LayerRank {
    LayerRank {
        normal: Arc::from([1, usize::MAX]),
        important: Arc::from([usize::MAX - 1, 0]),
    }
}
fn candidate<'a>(value: &'a CssText, layer: &'a LayerRank) -> Candidate<'a> {
    Candidate {
        property: "color",
        value,
        pending_source: None,
        important: false,
        origin: CascadeOrigin::Author,
        scope_depth: 1,
        inline: false,
        layer_context: 2,
        layer_rank: layer,
        specificity: (1, 2, 3),
        proximity: 4,
        order: (5, 6),
    }
}
#[test]
fn singleton_selection_avoids_the_ordered_vector_path() {
    let value = CssText::from_units(vec![114, 101, 100]);
    let layer = layer();
    let rows = [candidate(&value, &layer)];
    SORTED_SELECTIONS.with(|count| count.set(0));
    let selected = select(&rows, &[], |_| {
        Ok::<_, ()>(Evaluation {
            keyword: None,
            value: 17,
        })
    })
    .unwrap()
    .unwrap();
    assert!(std::ptr::eq(selected.0, &raw const rows[0]));
    assert_eq!(selected.1, 17);
    assert_eq!(
        SORTED_SELECTIONS.with(Cell::get),
        0,
        "singleton must avoid collecting/sorting an ordered vector"
    );
}
#[test]
fn empty_and_multiple_candidates_keep_the_original_ordered_path() {
    let value = CssText::from_units(vec![114]);
    let layer = layer();
    let mut newer = candidate(&value, &layer);
    newer.order = (9, 9);
    let rows = [candidate(&value, &layer), newer];
    SORTED_SELECTIONS.with(|count| count.set(0));
    let mut calls = 0;
    assert!(select(&[], &[], |_| {
        calls += 1;
        Ok::<_, ()>(Evaluation {
            keyword: None,
            value: 0,
        })
    })
    .unwrap()
    .is_none());
    let selected = select(&rows, &[], |_| {
        calls += 1;
        Ok::<_, ()>(Evaluation {
            keyword: None,
            value: 17,
        })
    })
    .unwrap()
    .unwrap();
    assert!(std::ptr::eq(selected.0, &raw const rows[1]));
    assert_eq!(calls, 1);
    assert_eq!(SORTED_SELECTIONS.with(Cell::get), 2);
}
fn compare_case(
    rows: &[Candidate<'_>],
    restrictions: &[Rollback<'_>],
    keyword: Option<WideKeyword>,
) {
    let payload = Box::new([1_u8, 2, 3]);
    let before_calls = Cell::new(0);
    let after_calls = Cell::new(0);
    let before = original_select(rows, restrictions, |_| {
        before_calls.set(before_calls.get() + 1);
        Ok::<_, ()>(Evaluation {
            keyword,
            value: &payload,
        })
    })
    .unwrap();
    let after = select(rows, restrictions, |_| {
        after_calls.set(after_calls.get() + 1);
        Ok::<_, ()>(Evaluation {
            keyword,
            value: &payload,
        })
    })
    .unwrap();
    assert_eq!(before_calls.get(), after_calls.get());
    assert!(after_calls.get() <= 1);
    let before = before
        .map(|(candidate, payload)| (std::ptr::from_ref(candidate), std::ptr::from_ref(payload)));
    let after = after
        .map(|(candidate, payload)| (std::ptr::from_ref(candidate), std::ptr::from_ref(payload)));
    assert_eq!(before, after);
}
fn check_keywords(rows: &[Candidate<'_>], restrictions: &[Rollback<'_>]) {
    for keyword in [
        None,
        Some(WideKeyword::Initial),
        Some(WideKeyword::Inherit),
        Some(WideKeyword::Unset),
        Some(WideKeyword::Revert),
        Some(WideKeyword::RevertLayer),
    ] {
        compare_case(rows, restrictions, keyword);
    }
}
fn check_restrictions(rows: &[Candidate<'_>]) {
    check_keywords(rows, &[]);
    for origin in [
        CascadeOrigin::UserAgent,
        CascadeOrigin::User,
        CascadeOrigin::Author,
    ] {
        check_keywords(rows, &[Rollback::Origin(origin.normal_order())]);
        for context in [1, 2, 3] {
            for inline in [false, true] {
                for normal in [&[0_usize][..], &[1, usize::MAX][..], &[2][..]] {
                    check_keywords(
                        rows,
                        &[Rollback::Layer {
                            origin,
                            context,
                            inline,
                            normal,
                        }],
                    );
                }
            }
        }
    }
}
#[test]
fn singleton_matches_original_for_keywords_and_origin_layer_restrictions() {
    let value = CssText::from_units(vec![114]);
    let rank = layer();
    for origin in [
        CascadeOrigin::UserAgent,
        CascadeOrigin::User,
        CascadeOrigin::Author,
    ] {
        for important in [false, true] {
            for inline in [false, true] {
                let mut row = candidate(&value, &rank);
                row.origin = origin;
                row.important = important;
                row.inline = inline;
                check_restrictions(&[row]);
            }
        }
    }
}
#[test]
fn singleton_evaluator_errors_keep_identity_and_exclusions_never_evaluate() {
    let value = CssText::from_units(vec![114]);
    let rank = layer();
    let rows = [candidate(&value, &rank)];
    let sentinel = Box::new([8_u8]);
    let calls = Cell::new(0);
    let error = select::<(), _, _>(&rows, &[], |_| {
        calls.set(calls.get() + 1);
        Err(&sentinel)
    })
    .err()
    .unwrap();
    assert!(std::ptr::eq(error, &raw const sentinel));
    assert_eq!(calls.get(), 1);
    let result = select::<(), _, _>(&rows, &[Rollback::Origin(0)], |_| {
        calls.set(calls.get() + 1);
        Err(&sentinel)
    })
    .unwrap();
    assert!(result.is_none());
    assert_eq!(calls.get(), 1);
}
