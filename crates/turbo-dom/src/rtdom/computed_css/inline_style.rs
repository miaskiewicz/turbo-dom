//! Inline declarations use the same parser and admission as compiled stylesheet rules.
use super::candidate_set::CandidateSet;
use super::candidates::Candidate;
use super::cascade_order::CascadeOrigin;
use super::declaration_model::{compile_declarations, CompiledDeclaration};
use super::prepared::LayerRank;
use super::{
    check_component_limits, declarations, ComputationLimits, ComputedCssError, CssDeclaration,
    CssText,
};
use std::sync::Arc;

pub struct InlineStyle {
    declarations: Vec<CssDeclaration>,
    compiled: Vec<CompiledDeclaration>,
    rank: LayerRank,
}
#[derive(Clone, Copy)]
pub struct InlineLimits {
    pub computation: ComputationLimits,
    pub declarations: usize,
    pub properties: usize,
}
impl InlineStyle {
    pub fn compile(source: &CssText, limits: InlineLimits) -> Result<Self, ComputedCssError> {
        check_component_limits(source, limits.computation)?;
        let declarations = declarations(source.as_units());
        if declarations.len() > limits.declarations {
            return Err(ComputedCssError::ResourceLimit);
        }
        let compiled = compile_declarations(&declarations, limits.computation)?;
        let count = compiled
            .iter()
            .try_fold(0usize, |total, item| {
                total.checked_add(item.properties.len())
            })
            .ok_or(ComputedCssError::ResourceLimit)?;
        if count > limits.properties {
            return Err(ComputedCssError::ResourceLimit);
        }
        Ok(Self {
            declarations,
            compiled,
            rank: LayerRank {
                normal: Arc::from([usize::MAX]),
                important: Arc::from([0]),
            },
        })
    }
    pub fn append<'a>(
        &'a self,
        set: &mut CandidateSet<'a>,
        scope_depth: u32,
        layer_context: usize,
    ) {
        for declaration in &self.compiled {
            let source = &self.declarations[declaration.source_index];
            for (index, property) in declaration.properties.iter().enumerate() {
                let candidate = Candidate {
                    property: &property.name,
                    value: property.value.as_ref().unwrap_or(&source.value),
                    pending_source: property.pending_shorthand.then_some(source.name.as_str()),
                    important: source.important,
                    origin: CascadeOrigin::Author,
                    scope_depth,
                    inline: true,
                    layer_context,
                    layer_rank: &self.rank,
                    specificity: (0, 0, 0),
                    proximity: usize::MAX,
                    order: (usize::MAX, declaration.source_index),
                };
                let marker = if index == 0 {
                    declaration.declared_text_decoration.as_ref()
                } else {
                    None
                };
                set.push_property(candidate, marker);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    fn limits() -> InlineLimits {
        InlineLimits {
            computation: ComputationLimits {
                max_nesting: 16,
                max_output_units: 4096,
                max_total_units: 4096,
            },
            declarations: 8,
            properties: 16,
        }
    }
    fn text(raw: &str) -> CssText {
        CssText::from_units(raw.encode_utf16().collect())
    }
    #[test]
    fn inline_budgets_reject_expansion_before_any_candidate_can_escape() {
        let mut policy = limits();
        policy.declarations = 0;
        assert!(matches!(
            InlineStyle::compile(&text("color:red"), policy),
            Err(ComputedCssError::ResourceLimit)
        ));
        policy = limits();
        policy.properties = 3;
        assert!(matches!(
            InlineStyle::compile(&text("margin:1px"), policy),
            Err(ComputedCssError::ResourceLimit)
        ));
        policy.properties = 4;
        assert!(InlineStyle::compile(&text("margin:1px"), policy).is_ok());
    }
    #[test]
    fn inline_custom_tokens_keep_original_case_and_importance_in_the_shared_cascade() {
        let inline =
            InlineStyle::compile(&text("--Case: red !important;color:var(--Case)"), limits())
                .unwrap();
        let mut set = CandidateSet {
            custom: BTreeMap::new(),
            declared_text_decoration: Vec::new(),
            ordinary: BTreeMap::new(),
        };
        inline.append(&mut set, 2, 4);
        assert!(set.custom["--Case"][0].important);
        assert_eq!(set.custom["--Case"][0].scope_depth, 2);
        assert_eq!(set.ordinary["color"][0].pending_source, None);
        assert!(!set.custom.contains_key("--case"));
    }
}
