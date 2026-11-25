//! Post-cascade substitution and pending shorthand expansion share admission grammar.
use super::declaration_model::admit_ordinary;
use super::grammar::DeclarationAdmission;
use super::properties::wide_keyword;
use super::{
    substitute, ComputationLimits, ComputedCssError, CssSource, CssText, CustomProperties,
};
use crate::rtdom::css_shorthand::expand_css as expand;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OrdinaryError {
    Computation(ComputedCssError),
    RequiresGrammar(String),
    RequiresValueContext(String),
}
impl From<ComputedCssError> for OrdinaryError {
    fn from(error: ComputedCssError) -> Self {
        Self::Computation(error)
    }
}

/// `pending_source` is present only for a shorthand whose substitution was deferred.
/// Invalid-at-computed-value winners return `None`; they never expose a lower candidate.
pub fn computed_property_tokens(
    property: &str,
    pending_source: Option<&str>,
    raw: &CssText,
    custom: &CustomProperties,
    limits: ComputationLimits,
) -> Result<Option<CssText>, OrdinaryError> {
    let Some(value) = substitute(raw, custom, limits)? else {
        return Ok(None);
    };
    if wide_keyword(&value).is_some() {
        return Ok(Some(value));
    }
    let source = CssSource::new(value.as_units());
    let name = pending_source.unwrap_or(property);
    let admitted = match admit_ordinary(name, source.scalar(), limits)? {
        DeclarationAdmission::Valid(value) => Some(value),
        DeclarationAdmission::Invalid => None,
        DeclarationAdmission::RequiresGrammar(name) => {
            return Err(OrdinaryError::RequiresGrammar(name))
        }
    };
    let Some(admitted) = admitted else {
        return Ok(None);
    };
    if pending_source.is_none() {
        return Ok(Some(value));
    }
    let Some(properties) = expand(name, &admitted) else {
        return Ok(None);
    };
    let Some(item) = properties.into_iter().find(|item| item.name == property) else {
        return Ok(None);
    };
    if item.pending.is_some() {
        return Err(OrdinaryError::RequiresValueContext(name.to_owned()));
    }
    Ok(Some(CssText::from_units(
        item.value.encode_utf16().collect(),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    fn text(value: &str) -> CssText {
        CssText::from_units(value.encode_utf16().collect())
    }
    fn limits() -> ComputationLimits {
        ComputationLimits {
            max_nesting: 16,
            max_output_units: 2048,
            max_total_units: 4096,
        }
    }
    #[test]
    fn registry_admission_does_not_silently_drop_selected_unknown_grammar() {
        let custom = CustomProperties::default();
        assert_eq!(
            computed_property_tokens("anchor-name", None, &text("--card"), &custom, limits())
                .unwrap(),
            Some(text("--card"))
        );
        assert_eq!(
            computed_property_tokens("trigger-scope", None, &text("--card"), &custom, limits()),
            Err(OrdinaryError::RequiresGrammar("trigger-scope".to_owned()))
        );
    }
    #[test]
    fn system_font_winner_requires_context_instead_of_becoming_invalid_css() {
        assert_eq!(
            computed_property_tokens(
                "font-family",
                Some("font"),
                &text("menu"),
                &CustomProperties::default(),
                limits()
            ),
            Err(OrdinaryError::RequiresValueContext("font".to_owned()))
        );
    }
    #[test]
    fn mismatched_pending_property_never_exposes_an_unrelated_longhand() {
        let custom = CustomProperties::default();
        assert_eq!(
            computed_property_tokens("color", Some("margin"), &text("1px"), &custom, limits())
                .unwrap(),
            None
        );
        assert_eq!(
            computed_property_tokens("color", Some("color"), &text("red"), &custom, limits())
                .unwrap(),
            None
        );
    }
    #[test]
    fn pending_shorthands_expand_only_after_selected_variables_are_computed() {
        let own = BTreeMap::from([("--spacing".to_owned(), text("1px 2px"))]);
        let custom =
            super::super::compute_custom_properties(&BTreeMap::new(), &own, limits()).unwrap();
        assert_eq!(
            computed_property_tokens(
                "margin-left",
                Some("margin"),
                &text("var(--spacing)"),
                &custom,
                limits()
            )
            .unwrap(),
            Some(text("2px"))
        );
        assert_eq!(
            computed_property_tokens(
                "margin-top",
                Some("margin"),
                &text("var(--spacing)"),
                &custom,
                limits()
            )
            .unwrap(),
            Some(text("1px"))
        );
        assert_eq!(
            computed_property_tokens(
                "margin-left",
                Some("margin"),
                &text("var(--absent)"),
                &custom,
                limits()
            )
            .unwrap(),
            None
        );
    }
    #[test]
    fn wide_results_and_invalid_computed_winners_are_distinct_from_limits() {
        let custom = CustomProperties::default();
        assert_eq!(
            computed_property_tokens("color", None, &text("var(--x, inherit)"), &custom, limits())
                .unwrap(),
            Some(text("inherit"))
        );
        assert_eq!(
            computed_property_tokens("color", None, &text("var(--x, 2px)"), &custom, limits())
                .unwrap(),
            None
        );
        assert_eq!(
            computed_property_tokens("width", None, &text("50%"), &custom, limits()).unwrap(),
            Some(text("50%"))
        );
        let mut small = limits();
        small.max_output_units = 1;
        assert_eq!(
            computed_property_tokens("color", None, &text("red"), &custom, small),
            Err(OrdinaryError::Computation(ComputedCssError::ResourceLimit))
        );
    }
}
