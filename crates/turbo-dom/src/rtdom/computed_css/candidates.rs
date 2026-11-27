//! Winner selection and rollback over declarations already admitted by shared grammar.
use super::cascade_order::{encapsulation_rank, CascadeOrigin, CascadeRank};
use super::prepared::LayerRank;
use super::properties::WideKeyword;
use super::CssText;
use std::cmp::Reverse;

#[derive(Clone, Copy)]
pub struct Candidate<'a> {
    pub property: &'a str,
    pub value: &'a CssText,
    pub pending_source: Option<&'a str>,
    pub important: bool,
    pub origin: CascadeOrigin,
    pub scope_depth: u32,
    pub inline: bool,
    pub layer_context: usize,
    pub layer_rank: &'a LayerRank,
    pub specificity: (u32, u32, u32),
    pub proximity: usize,
    pub order: (usize, usize),
}
impl Candidate<'_> {
    #[must_use]
    pub fn rank(&self) -> CascadeRank<'_> {
        CascadeRank {
            level: self.origin.level(self.important),
            encapsulation: encapsulation_rank(self.scope_depth, self.important),
            inline: self.inline,
            layer: if self.important {
                &self.layer_rank.important
            } else {
                &self.layer_rank.normal
            },
            specificity: self.specificity,
            proximity: Reverse(self.proximity),
            order: self.order,
        }
    }
}

pub enum Rollback<'a> {
    Origin(u8),
    Layer {
        origin: CascadeOrigin,
        context: usize,
        inline: bool,
        normal: &'a [usize],
    },
}
impl<'a> Rollback<'a> {
    #[must_use]
    pub fn from_candidate(candidate: &Candidate<'a>, keyword: super::RollbackKeyword) -> Self {
        match keyword {
            super::RollbackKeyword::Revert => Self::Origin(candidate.origin.normal_order()),
            super::RollbackKeyword::RevertLayer => Self::Layer {
                origin: candidate.origin,
                context: candidate.layer_context,
                inline: candidate.inline,
                normal: candidate.layer_rank.normal.as_ref(),
            },
        }
    }

    fn excludes(&self, candidate: &Candidate<'_>) -> bool {
        match self {
            Self::Origin(origin) => candidate.origin.normal_order() >= *origin,
            Self::Layer {
                origin,
                context,
                inline,
                normal,
            } => {
                candidate.origin == *origin
                    && candidate.layer_context == *context
                    && (candidate.inline, candidate.layer_rank.normal.as_ref())
                        >= (*inline, *normal)
            }
        }
    }
}

/// The caller can use raw or already-substituted keyword recognition. Important
/// rollback follows earlier *normal layer order*, not merely the next candidate
/// in reversed important order (Chrome159 source-hashed inverse controls).
pub struct Evaluation<T> {
    pub keyword: Option<WideKeyword>,
    pub value: T,
}

pub fn select<'a, T, E, F>(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rtdom::computed_css::properties::wide_keyword;
    use std::sync::Arc;
    fn text(value: &str) -> CssText {
        CssText::from_units(value.encode_utf16().collect())
    }
    fn layer(index: usize) -> LayerRank {
        LayerRank {
            normal: Arc::from([index, usize::MAX]),
            important: Arc::from([usize::MAX - index, 0]),
        }
    }
    fn candidate<'a>(
        value: &'a CssText,
        layer: &'a LayerRank,
        order: usize,
        important: bool,
    ) -> Candidate<'a> {
        Candidate {
            pending_source: None,
            property: "color",
            value,
            important,
            origin: CascadeOrigin::Author,
            scope_depth: 0,
            inline: false,
            layer_context: 0,
            layer_rank: layer,
            specificity: (1, 0, 0),
            proximity: usize::MAX,
            order: (order, 0),
        }
    }
    fn pick<'a>(declarations: &'a [Candidate<'a>]) -> Option<&'a Candidate<'a>> {
        select(declarations, &[], |candidate| {
            Ok::<_, ()>(Evaluation {
                keyword: wide_keyword(candidate.value),
                value: (),
            })
        })
        .unwrap()
        .map(|(candidate, ())| candidate)
    }
    #[test]
    fn important_revert_layer_does_not_select_later_normal_or_important_layers() {
        let rollback = text("revert-layer");
        let blue = text("blue");
        let first = layer(0);
        let second = layer(1);
        let declarations = [
            candidate(&rollback, &first, 0, true),
            candidate(&blue, &second, 1, true),
        ];
        assert!(pick(&declarations).is_none());
        let declarations = [
            candidate(&blue, &first, 0, false),
            candidate(&rollback, &second, 1, true),
        ];
        assert_eq!(
            pick(&declarations).unwrap().value.as_units(),
            blue.as_units()
        );
    }
    #[test]
    fn normal_and_inline_rollback_select_earlier_source_layers_without_specificity_leaks() {
        let red = text("red");
        let rollback = text("revert-layer");
        let first = layer(0);
        let second = layer(1);
        let declarations = [
            candidate(&red, &first, 0, false),
            candidate(&rollback, &second, 1, false),
        ];
        assert_eq!(
            pick(&declarations).unwrap().value.as_units(),
            red.as_units()
        );
        let mut inline = candidate(&rollback, &second, 1, true);
        inline.inline = true;
        let declarations = [candidate(&red, &second, 0, true), inline];
        assert_eq!(
            pick(&declarations).unwrap().value.as_units(),
            red.as_units()
        );
    }
    #[test]
    fn origin_rollback_and_explicit_restrictions_do_not_reveal_other_author_values() {
        let blue = text("blue");
        let red = text("red");
        let rollback = text("revert");
        let rank = layer(0);
        let mut user = candidate(&red, &rank, 0, false);
        user.origin = CascadeOrigin::User;
        let author = candidate(&blue, &rank, 1, false);
        let reverted = candidate(&rollback, &rank, 2, false);
        let declarations = [user, author, reverted];
        assert_eq!(
            pick(&declarations).unwrap().value.as_units(),
            red.as_units()
        );
        let restrictions = [Rollback::from_candidate(
            &declarations[1],
            super::super::RollbackKeyword::Revert,
        )];
        let selected = select(&declarations, &restrictions, |item| {
            Ok::<_, ()>(Evaluation {
                keyword: None,
                value: item.value,
            })
        })
        .unwrap()
        .unwrap()
        .1;
        assert_eq!(selected.as_units(), red.as_units());
        assert!(Rollback::from_candidate(
            &declarations[1],
            super::super::RollbackKeyword::RevertLayer
        )
        .excludes(&declarations[1]));
    }
}
