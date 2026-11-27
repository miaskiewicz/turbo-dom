//! SVG adapters consume the shared raw CSS syntax and selector metadata.
use super::{
    super::{usvg::roxmltree, SvgResourceError},
    state::{self, Policy},
};
use crate::style::{CascadeOrigin, Declaration, MatchedDeclaration};

use turbo_dom::rtdom::computed_css::{
    self,
    stylesheet::{parse_sheet, GroupMode, SheetError, SheetLimits, SheetRule},
};
use turbo_dom::rtdom::query::{
    match_for_cascade, parse_for_cascade_units, LiveNavigator, SelectorList,
};
use turbo_dom::{Handle, Tree};

struct Rule {
    selectors: SelectorList,
    declarations: Vec<Declaration>,
}

#[derive(Default)]
pub(super) struct Rules {
    rules: Vec<Rule>,
    pub uncertain: bool,
}

fn scalar(text: &computed_css::CssText) -> Result<String, SvgResourceError> {
    String::from_utf16(text.as_units()).map_err(|_| SvgResourceError::UnsupportedCustomProperties)
}

pub(super) fn declarations(
    text: &str,
    policy: &Policy,
) -> Result<Vec<Declaration>, SvgResourceError> {
    computed_css::check_component_limits(&state::text(text), policy.limits)
        .map_err(state::kernel_error)?;
    let mut result = Vec::new();
    for declaration in computed_css::declarations(&text.encode_utf16().collect::<Vec<_>>()) {
        if !computed_css::valid_custom_value_syntax(&declaration.value, policy.limits)
            .map_err(state::kernel_error)?
        {
            continue;
        }
        let value = scalar(&declaration.value)?;
        if let Some(value) = admitted(&declaration.name, value, policy)? {
            result.push(Declaration {
                property: declaration.name,
                value,
                important: declaration.important,
            });
        }
    }
    Ok(result)
}

fn admitted(
    name: &str,
    value: String,
    policy: &Policy,
) -> Result<Option<String>, SvgResourceError> {
    use turbo_dom::rtdom::css_shorthand::{longhand_value, Spelled};
    if name.starts_with("--") || state::is_paint(name) {
        return Ok(Some(value));
    }
    match longhand_value(name, &value) {
        Spelled::Invalid => Ok(None),
        Spelled::Valid(normalized) => validated_nonpaint(name, value, normalized),
        Spelled::Unchecked(_) => {
            let environment =
                computed_css::has_function_reference(&state::text(&value), "env", policy.limits)
                    .map_err(state::kernel_error)?;
            if environment || state::has_reference(&value, policy)? {
                Ok(Some(value))
            } else {
                Err(SvgResourceError::UnsupportedCustomProperties)
            }
        }
    }
}

fn validated_nonpaint(
    name: &str,
    value: String,
    normalized: String,
) -> Result<Option<String>, SvgResourceError> {
    if !supported_nonpaint(name, &value, &normalized) {
        return Err(SvgResourceError::UnsupportedCustomProperties);
    }
    Ok(Some(if matches!(name, "display" | "visibility") {
        normalized
    } else {
        value
    }))
}

fn supported_nonpaint(name: &str, raw: &str, normalized: &str) -> bool {
    match name {
        "opacity" => raw.trim().parse::<f64>().is_ok_and(f64::is_finite),
        "display" => matches!(normalized, "none" | "inline" | "block"),
        "visibility" => matches!(normalized, "visible" | "hidden"),
        "font-family" => !matches!(
            normalized,
            "inherit" | "initial" | "unset" | "revert" | "revert-layer"
        ),
        _ => false,
    }
}

pub(super) struct Matched {
    pub declarations: Vec<MatchedDeclaration>,
    pub css_start: usize,
}

impl Rules {
    pub fn new(
        document: &roxmltree::Document<'_>,
        policy: &Policy,
    ) -> Result<Self, SvgResourceError> {
        let mut result = Self::default();
        for node in document
            .descendants()
            .filter(|node| node.has_tag_name((super::super::SVG_NS, "style")))
        {
            let css: String = node
                .children()
                .filter(roxmltree::Node::is_text)
                .filter_map(|child| child.text())
                .collect();
            let limits = SheetLimits {
                source_units: policy.limits.max_output_units,
                rules: policy.limits.max_output_units,
                group_depth: 16,
            };
            let parsed = parse_sheet(
                &css.encode_utf16().collect::<Vec<_>>(),
                limits,
                GroupMode::Recursive,
            )
            .map_err(sheet_error)?;
            result.absorb(parsed, policy)?;
        }
        Ok(result)
    }

    fn absorb(&mut self, rules: Vec<SheetRule>, policy: &Policy) -> Result<(), SvgResourceError> {
        let mut pending: Vec<_> = rules.into_iter().rev().collect();
        while let Some(rule) = pending.pop() {
            match rule {
                SheetRule::Style { selector, body } => {
                    computed_css::check_component_limits(&selector, policy.limits)
                        .map_err(state::kernel_error)?;
                    if let Ok(selectors) = parse_for_cascade_units(selector.as_units()) {
                        self.uncertain |= selectors
                            .selectors()
                            .iter()
                            .any(|selector| !selector.certifies_structural_matching());
                        self.rules.push(Rule {
                            selectors,
                            declarations: declarations(&scalar(&body)?, policy)?,
                        });
                    }
                }
                SheetRule::Group { rules, .. } => {
                    self.uncertain = true;
                    pending.extend(rules.into_iter().rev());
                }
                SheetRule::Statement { .. } | SheetRule::Opaque { .. } => self.uncertain = true,
                SheetRule::Ignored => {}
            }
        }
        Ok(())
    }

    pub fn matched(
        &self,
        tree: &Tree,
        handle: Handle,
        node: roxmltree::Node<'_, '_>,
        policy: &Policy,
    ) -> Result<Matched, SvgResourceError> {
        let mut matched = Vec::new();
        append(&mut matched, presentations(node), false, (0, 0, 0));
        let css_start = matched.len();
        for rule in &self.rules {
            let specificity = rule
                .selectors
                .selectors()
                .iter()
                .filter(|selector| match_for_cascade(&LiveNavigator(tree), handle, selector))
                .map(turbo_dom::rtdom::query::CascadeSelector::specificity)
                .max();
            if let Some(specificity) = specificity {
                append(&mut matched, rule.declarations.clone(), false, specificity);
            }
        }
        append(
            &mut matched,
            declarations(node.attribute("style").unwrap_or_default(), policy)?,
            true,
            (0, 0, 0),
        );
        Ok(Matched {
            declarations: matched,
            css_start,
        })
    }
}

fn sheet_error(error: SheetError) -> SvgResourceError {
    match error {
        SheetError::GroupDepth => SvgResourceError::CustomPropertyNesting,
        SheetError::RuleCount | SheetError::SourceUnits => SvgResourceError::CustomPropertyBytes,
    }
}

fn append(
    target: &mut Vec<MatchedDeclaration>,
    declarations: Vec<Declaration>,
    inline: bool,
    specificity: (u32, u32, u32),
) {
    for declaration in declarations {
        target.push(MatchedDeclaration {
            declaration,
            origin: CascadeOrigin::Author,
            scope_depth: 0,
            inline,
            specificity,
            order: target.len(),
        });
    }
}

pub(super) fn presentations(node: roxmltree::Node<'_, '_>) -> Vec<Declaration> {
    node.attributes()
        .filter(|attr| attr.namespace().is_none() && is_presentation(attr.name()))
        .map(|attr| Declaration {
            property: attr.name().to_owned(),
            value: attr.value().to_owned(),
            important: false,
        })
        .collect()
}

// Non-color properties are tracked only to refuse uncertified variable geometry.
fn is_presentation(name: &str) -> bool {
    const PROPERTIES: &[&str] = &[
        "color",
        "fill",
        "stroke",
        "stop-color",
        "flood-color",
        "lighting-color",
        "width",
        "height",
        "x",
        "y",
        "rx",
        "ry",
        "cx",
        "cy",
        "r",
        "opacity",
        "stroke-width",
        "stroke-opacity",
        "fill-opacity",
        "font-size",
        "font-family",
        "font-weight",
        "display",
        "visibility",
        "transform",
        "clip-path",
        "mask",
        "filter",
        "marker-start",
        "marker-end",
        "marker-mid",
        "stop-opacity",
        "flood-opacity",
        "text-anchor",
    ];
    PROPERTIES.contains(&name)
}

pub(super) fn node_has_references(
    node: roxmltree::Node<'_, '_>,
    policy: &Policy,
) -> Result<bool, SvgResourceError> {
    if state::has_reference(node.attribute("style").unwrap_or_default(), policy)? {
        return Ok(true);
    }
    for declaration in presentations(node) {
        if state::has_reference(&declaration.value, policy)? {
            return Ok(true);
        }
    }
    Ok(false)
}
