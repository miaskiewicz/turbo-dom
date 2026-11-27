//! Cascade selection operates over already-verified matching facts, not DOM copies.
use super::candidates::{select, Candidate, Evaluation};
use super::custom_cascade::{resolve_custom_candidates, CustomCascadeError, CustomCascadeLimits};
use super::ordinary::{computed_property_tokens, OrdinaryError};
use super::properties::wide_keyword;
use super::{CssText, CustomProperties};
use std::collections::BTreeMap;

pub struct SelectedStyle {
    pub custom: CustomProperties,
    /// A present `None` is an invalid-at-computed-value winner, resolved as unset.
    pub ordinary: BTreeMap<String, Option<CssText>>,
}
#[derive(Debug)]
pub enum SelectionError {
    Custom(CustomCascadeError),
    Computation(OrdinaryError),
}

pub fn select_style<'a>(
    parent_custom: &BTreeMap<String, CssText>,
    custom: &'a BTreeMap<String, Vec<Candidate<'a>>>,
    ordinary: &'a BTreeMap<String, Vec<Candidate<'a>>>,
    limits: CustomCascadeLimits,
) -> Result<SelectedStyle, SelectionError> {
    let custom =
        resolve_custom_candidates(parent_custom, custom, limits).map_err(SelectionError::Custom)?;
    let mut selected = BTreeMap::new();
    for (name, candidates) in ordinary {
        let winner = select(candidates, &[], |candidate| {
            let value = computed_property_tokens(
                name,
                candidate.pending_source,
                candidate.value,
                &custom,
                limits.computation,
            )?;
            Ok::<_, OrdinaryError>(Evaluation {
                keyword: value.as_ref().and_then(wide_keyword),
                value,
            })
        })
        .map_err(SelectionError::Computation)?;
        if let Some((_, value)) = winner {
            selected.insert(name.clone(), value);
        }
    }
    Ok(SelectedStyle {
        custom,
        ordinary: selected,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rtdom::computed_css::{
        cascade_order::CascadeOrigin, prepared::LayerRank, ComputationLimits,
    };
    use std::sync::Arc;
    fn text(source: &str) -> CssText {
        CssText::from_units(source.encode_utf16().collect())
    }
    fn rank(index: usize) -> LayerRank {
        LayerRank {
            normal: Arc::from([index]),
            important: Arc::from([usize::MAX - index]),
        }
    }
    fn candidate<'a>(
        name: &'a str,
        value: &'a CssText,
        layer: &'a LayerRank,
        order: usize,
    ) -> Candidate<'a> {
        Candidate {
            property: name,
            value,
            pending_source: None,
            important: false,
            origin: CascadeOrigin::Author,
            scope_depth: 0,
            inline: false,
            layer_context: 0,
            layer_rank: layer,
            specificity: (0, 1, 0),
            proximity: usize::MAX,
            order: (order, 0),
        }
    }
    fn limits() -> CustomCascadeLimits {
        CustomCascadeLimits {
            computation: ComputationLimits {
                max_nesting: 16,
                max_output_units: 4096,
                max_total_units: 8192,
            },
            rollbacks: 16,
        }
    }
    #[test]
    fn unresolved_winner_is_unset_and_does_not_restore_a_lower_declaration() {
        let red = text("red");
        let missing = text("var(--missing)");
        let layer = rank(0);
        let ordinary = BTreeMap::from([(
            "color".to_owned(),
            vec![
                candidate("color", &red, &layer, 0),
                candidate("color", &missing, &layer, 1),
            ],
        )]);
        let result = select_style(&BTreeMap::new(), &BTreeMap::new(), &ordinary, limits()).unwrap();
        assert_eq!(result.ordinary.get("color"), Some(&None));
    }
    #[test]
    fn rollback_produced_by_substitution_reselects_an_earlier_layer() {
        let rollback = text("revert-layer");
        let parent = BTreeMap::from([("--rollback".to_owned(), rollback)]);
        let blue = text("blue");
        let deferred = text("var(--rollback)");
        let first = rank(0);
        let second = rank(1);
        let ordinary = BTreeMap::from([(
            "color".to_owned(),
            vec![
                candidate("color", &blue, &first, 0),
                candidate("color", &deferred, &second, 1),
            ],
        )]);
        let result = select_style(&parent, &BTreeMap::new(), &ordinary, limits()).unwrap();
        assert_eq!(result.ordinary.get("color"), Some(&Some(blue)));
    }
}
