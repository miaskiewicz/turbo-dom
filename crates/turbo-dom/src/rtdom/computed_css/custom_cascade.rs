//! Candidate rollback surrounds the canonical variable resolver; it never substitutes tokens.
use super::candidates::{select, Candidate, Evaluation, Rollback};
use super::properties::wide_keyword;
use super::{
    compute_custom_properties_detailed, ComputationLimits, ComputedCssError, CssText,
    CustomComputation, CustomProperties,
};
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
pub struct CustomCascadeLimits {
    pub computation: ComputationLimits,
    pub rollbacks: usize,
}
#[derive(Debug)]
pub enum CustomCascadeError {
    Computation(ComputedCssError),
    UnknownRollbackProperty(String),
}

pub fn resolve_custom_candidates<'a>(
    parent: &BTreeMap<String, CssText>,
    candidates: &'a BTreeMap<String, Vec<Candidate<'a>>>,
    limits: CustomCascadeLimits,
) -> Result<CustomProperties, CustomCascadeError> {
    let mut restrictions: BTreeMap<String, Vec<Rollback<'a>>> = BTreeMap::new();
    let mut retries = 0;
    loop {
        let mut selected = BTreeMap::new();
        let mut own = BTreeMap::new();
        for (name, values) in candidates {
            let cuts = restrictions.get(name).map_or(&[][..], Vec::as_slice);
            let winner = select(values, cuts, |candidate| {
                Ok::<_, ComputedCssError>(Evaluation {
                    keyword: wide_keyword(candidate.value),
                    value: (),
                })
            })
            .map_err(CustomCascadeError::Computation)?;
            if let Some((candidate, ())) = winner {
                own.insert(name.clone(), candidate.value.clone());
                selected.insert(name.as_str(), candidate);
            }
        }
        match compute_custom_properties_detailed(parent, &own, limits.computation)
            .map_err(CustomCascadeError::Computation)?
        {
            CustomComputation::Computed(values) => return Ok(values),
            CustomComputation::RequiresCascade { property, keyword } => {
                if retries >= limits.rollbacks {
                    return Err(CustomCascadeError::Computation(
                        ComputedCssError::ResourceLimit,
                    ));
                }
                let candidate = selected
                    .get(property.as_str())
                    .ok_or_else(|| CustomCascadeError::UnknownRollbackProperty(property.clone()))?;
                restrictions
                    .entry(property)
                    .or_default()
                    .push(Rollback::from_candidate(candidate, keyword));
                retries += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rtdom::computed_css::{cascade_order::CascadeOrigin, prepared::LayerRank};
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
        name: &'a str,
        value: &'a CssText,
        layer: &'a LayerRank,
        order: usize,
    ) -> Candidate<'a> {
        Candidate {
            pending_source: None,
            property: name,
            value,
            important: false,
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
    fn limits(rollbacks: usize) -> CustomCascadeLimits {
        CustomCascadeLimits {
            computation: ComputationLimits {
                max_nesting: 32,
                max_output_units: 4096,
                max_total_units: 8192,
            },
            rollbacks,
        }
    }
    #[test]
    fn resolved_layer_rollback_retries_canonical_dependencies_with_fresh_memoization() {
        let red = text("red");
        let retry = text("var(--missing,revert-layer)");
        let outer = text("var(--x)");
        let early = layer(0);
        let late = layer(1);
        let candidates = BTreeMap::from([
            (
                "--x".to_owned(),
                vec![
                    candidate("--x", &red, &early, 0),
                    candidate("--x", &retry, &late, 1),
                ],
            ),
            (
                "--out".to_owned(),
                vec![candidate("--out", &outer, &late, 2)],
            ),
        ]);
        let result = resolve_custom_candidates(&BTreeMap::new(), &candidates, limits(1)).unwrap();
        assert_eq!(result.get("--x").unwrap().as_units(), red.as_units());
        assert_eq!(result.get("--out").unwrap().as_units(), red.as_units());
        assert!(matches!(
            resolve_custom_candidates(&BTreeMap::new(), &candidates, limits(0)),
            Err(CustomCascadeError::Computation(
                ComputedCssError::ResourceLimit
            ))
        ));
    }
    #[test]
    fn plain_wide_and_inherited_computed_values_use_existing_resolver_without_extra_retry() {
        let blue = text("blue");
        let inherit = text("var(--missing,inherit)");
        let first = layer(0);
        let parent = BTreeMap::from([("--x".to_owned(), blue.clone())]);
        let candidates = BTreeMap::from([(
            "--x".to_owned(),
            vec![candidate("--x", &inherit, &first, 0)],
        )]);
        let result = resolve_custom_candidates(&parent, &candidates, limits(0)).unwrap();
        assert_eq!(result.get("--x").unwrap().as_units(), blue.as_units());
    }
}
