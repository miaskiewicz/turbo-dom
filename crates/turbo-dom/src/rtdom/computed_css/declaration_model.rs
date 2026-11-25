//! Admission and expansion reuse the canonical ordinary-property/shorthand grammars.
use super::grammar::{computed_declaration_value, ConditionError, DeclarationAdmission};
use super::{
    valid_custom_value_syntax, ComputationLimits, ComputedCssError, CssDeclaration, CssSource,
    CssText,
};
use crate::rtdom::css_shorthand::{
    expand_css as expand, is_shorthand, longhand_alias, shorthand_value, Declaration,
};

pub struct PropertyTemplate {
    pub name: String,
    /// None borrows the original declaration payload; expanded static values own text.
    pub value: Option<CssText>,
    pub pending_shorthand: bool,
}
pub struct CompiledDeclaration {
    pub source_index: usize,
    pub properties: Vec<PropertyTemplate>,
    /// Admitted declared shorthand spelling, independent of computed longhands.
    pub declared_text_decoration: Option<CssText>,
}

pub fn compile_declarations(
    source: &[CssDeclaration],
    limits: ComputationLimits,
) -> Result<Vec<CompiledDeclaration>, ComputedCssError> {
    let mut output = Vec::new();
    for (index, declaration) in source.iter().enumerate() {
        if let Some(admitted) = admit(declaration, limits)? {
            output.push(CompiledDeclaration {
                source_index: index,
                declared_text_decoration: admitted.declared_text_decoration,
                properties: admitted.properties,
            });
        }
    }
    Ok(output)
}
struct AdmittedDeclaration {
    properties: Vec<PropertyTemplate>,
    declared_text_decoration: Option<CssText>,
}
fn unmarked(properties: Vec<PropertyTemplate>) -> AdmittedDeclaration {
    AdmittedDeclaration {
        properties,
        declared_text_decoration: None,
    }
}
fn declared_decoration(
    declaration: &CssDeclaration,
    properties: &[PropertyTemplate],
    limits: ComputationLimits,
) -> Result<Option<CssText>, ComputedCssError> {
    if !declaration.name.eq_ignore_ascii_case("text-decoration") {
        return Ok(None);
    }
    if properties.iter().any(|property| property.pending_shorthand) {
        return Ok(Some(declaration.value.clone()));
    }
    // Static expanded values originate in Rust Strings immediately above. Pending
    // raw code-unit payloads use the separate branch and never pass this boundary.
    let values: Vec<_> = properties
        .iter()
        .map(|property| {
            String::from_utf16(
                property
                    .value
                    .as_ref()
                    .expect("static expanded shorthand owns value")
                    .as_units(),
            )
            .expect("static expanded shorthand is encoded from Rust text")
        })
        .collect();
    let entries: Vec<_> = properties
        .iter()
        .zip(&values)
        .map(|(property, value)| Declaration {
            name: &property.name,
            value,
            important: false,
            pending: None,
        })
        .collect();
    let value = shorthand_value("text-decoration", &entries);
    if value.encode_utf16().count() > limits.max_output_units {
        return Err(ComputedCssError::ResourceLimit);
    }
    Ok(Some(CssText::from_units(value.encode_utf16().collect())))
}
fn raw_property(name: String) -> Vec<PropertyTemplate> {
    vec![PropertyTemplate {
        name,
        value: None,
        pending_shorthand: false,
    }]
}
/// Both compilation and post-substitution admission preserve the same typed
/// failures. Only the condition evaluator turns missing grammar into an error.
pub(super) fn admit_ordinary(
    name: &str,
    raw: &str,
    limits: ComputationLimits,
) -> Result<DeclarationAdmission, ComputedCssError> {
    match computed_declaration_value(name, raw, limits) {
        Ok(admission) => Ok(admission),
        Err(ConditionError::InvalidSyntax) => Ok(DeclarationAdmission::Invalid),
        Err(ConditionError::NestingLimit) => Err(ComputedCssError::NestingLimit),
        Err(ConditionError::Computation(error)) => Err(error),
        Err(ConditionError::RequiresGrammar(_)) => {
            unreachable!("ordinary admission returns a grammar capability value")
        }
    }
}

fn admit(
    declaration: &CssDeclaration,
    limits: ComputationLimits,
) -> Result<Option<AdmittedDeclaration>, ComputedCssError> {
    if declaration.name.starts_with("--") {
        return Ok((declaration.name.len() > 2
            && valid_custom_value_syntax(&declaration.value, limits)?)
        .then(|| unmarked(raw_property(declaration.name.clone()))));
    }
    // Full ordinary admission already performs the same strict component/var
    // syntax validation; do not tokenize every ordinary declaration twice.
    let name = declaration.name.to_ascii_lowercase();
    let source = CssSource::new(declaration.value.as_units());
    let admitted = match admit_ordinary(&name, source.scalar(), limits)? {
        DeclarationAdmission::Valid(value) => Some(value),
        DeclarationAdmission::Invalid => None,
        // Keep unsupported grammar until matching/cascade selects the candidate.
        DeclarationAdmission::RequiresGrammar(_) => return Ok(Some(unmarked(raw_property(name)))),
    };
    let Some(value) = admitted else {
        return Ok(None);
    };
    if !is_shorthand(&name) {
        return Ok(Some(unmarked(raw_property(
            longhand_alias(&name).unwrap_or(&name).to_owned(),
        ))));
    }
    let Some(expanded) = expand(&name, &value) else {
        return Ok(None);
    };
    let properties: Vec<_> = expanded
        .into_iter()
        .map(|property| PropertyTemplate {
            name: property.name.to_owned(),
            value: property
                .pending
                .is_none()
                .then(|| CssText::from_units(property.value.encode_utf16().collect())),
            pending_shorthand: property.pending.is_some(),
        })
        .collect();
    let declared_text_decoration = declared_decoration(declaration, &properties, limits)?;
    Ok(Some(AdmittedDeclaration {
        properties,
        declared_text_decoration,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn limits() -> ComputationLimits {
        ComputationLimits {
            max_nesting: 32,
            max_output_units: 4096,
            max_total_units: 8192,
        }
    }
    #[test]
    fn registry_declarations_remain_candidates_even_before_selected_grammar_is_available() {
        let declarations = super::super::declarations(
            &"trigger-scope:--card;anchor-name:--card"
                .encode_utf16()
                .collect::<Vec<_>>(),
        );
        let compiled = compile_declarations(&declarations, limits()).unwrap();
        assert_eq!(compiled.len(), 2);
        assert_eq!(compiled[0].properties[0].name, "trigger-scope");
        assert_eq!(compiled[1].properties[0].name, "anchor-name");
        assert!(compiled
            .iter()
            .all(|item| item.properties[0].value.is_none()));
    }
    #[test]
    fn malformed_var_is_omitted_before_cascade_but_valid_pending_values_keep_longhand_slots() {
        let declarations = super::super::declarations(
            &"margin-left:10px;margin:var();padding:var(--space);--Case:var(--missing,blue)"
                .encode_utf16()
                .collect::<Vec<_>>(),
        );
        let compiled = compile_declarations(&declarations, limits()).unwrap();
        assert_eq!(
            compiled
                .iter()
                .map(|item| item.source_index)
                .collect::<Vec<_>>(),
            [0, 2, 3]
        );
        assert_eq!(compiled[1].properties.len(), 4);
        assert!(compiled[1]
            .properties
            .iter()
            .all(|property| property.pending_shorthand && property.value.is_none()));
        assert_eq!(compiled[2].properties[0].name, "--Case");
    }
    #[test]
    fn static_shorthand_expansion_uses_one_shared_grammar_and_retains_raw_declaration() {
        let declarations = super::super::declarations(
            &"margin:1em 2px;invented-property:red;color:blue"
                .encode_utf16()
                .collect::<Vec<_>>(),
        );
        let compiled = compile_declarations(&declarations, limits()).unwrap();
        assert_eq!(compiled.len(), 2);
        assert_eq!(compiled[0].properties[0].name, "margin-top");
        assert_eq!(
            String::from_utf16(compiled[0].properties[0].value.as_ref().unwrap().as_units())
                .unwrap(),
            "1em"
        );
        assert_eq!(
            declarations[0].value.as_units(),
            "1em 2px".encode_utf16().collect::<Vec<_>>()
        );
        assert_eq!(compiled[1].properties[0].name, "color");
    }
    #[test]
    fn ordinary_admission_limits_remain_errors_instead_of_discarding_declarations() {
        let nested =
            super::super::declarations(&"color:rgb(1 2 3)".encode_utf16().collect::<Vec<_>>());
        let mut bounded = limits();
        bounded.max_nesting = 0;
        assert!(matches!(
            compile_declarations(&nested, bounded),
            Err(ComputedCssError::NestingLimit)
        ));
        let literal = super::super::declarations(&"color:blue".encode_utf16().collect::<Vec<_>>());
        bounded = limits();
        bounded.max_output_units = 1;
        assert!(matches!(
            compile_declarations(&literal, bounded),
            Err(ComputedCssError::ResourceLimit)
        ));
    }
}
