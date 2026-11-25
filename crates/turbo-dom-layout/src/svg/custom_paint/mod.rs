//! Certified solid-color custom-property paint through the shared CSS engine.
mod rules;
mod state;
mod xml;
use super::{usvg::roxmltree, SvgResourceError, SVG_NS};
use crate::style::{cascade_matched_tokens, ordered_matched_declarations, MatchedDeclaration};
use rules::Rules;
use state::{Policy, State};
use turbo_dom::rtdom::computed_css::CssDeclaration;

use std::sync::Arc;

pub(super) fn prepare(
    document: &roxmltree::Document<'_>,
) -> Result<Option<String>, SvgResourceError> {
    let mut policy = Policy::new(document.input_text());
    if !source_references(document, &policy)? {
        return Ok(None);
    }
    let rules = Rules::new(document, &policy)?;
    certify_context(document, &rules)?;
    let view = xml::SelectorTree::new(document);
    let edits = resolve(document, &rules, &view, &mut policy)?;
    xml::apply(document.input_text(), edits, policy.limits.max_output_units)
}

fn source_references(
    document: &roxmltree::Document<'_>,
    policy: &Policy,
) -> Result<bool, SvgResourceError> {
    for node in document.descendants().filter(roxmltree::Node::is_element) {
        if node.has_tag_name((SVG_NS, "style")) {
            let css: String = node
                .children()
                .filter(roxmltree::Node::is_text)
                .filter_map(|child| child.text())
                .collect();
            if state::has_reference(&css, policy)? {
                return Ok(true);
            }
        }
        if rules::node_has_references(node, policy)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn certify_context(
    document: &roxmltree::Document<'_>,
    rules: &Rules,
) -> Result<(), SvgResourceError> {
    let uncertain = document
        .descendants()
        .filter(roxmltree::Node::is_element)
        .any(uncertain_node);
    if rules.uncertain || uncertain || document.input_text().contains("<!DOCTYPE") {
        return Err(SvgResourceError::UnsupportedCustomProperties);
    }
    Ok(())
}

fn uncertain_node(node: roxmltree::Node<'_, '_>) -> bool {
    if node.tag_name().namespace() != Some(SVG_NS)
        || matches!(
            node.tag_name().name(),
            "use" | "animate" | "animateMotion" | "animateTransform" | "set" | "text" | "tspan"
        )
    {
        return true;
    }
    if !node.has_tag_name((SVG_NS, "style")) {
        return false;
    }
    let media = node.attribute("media").unwrap_or_default().trim();
    let kind = node.attribute("type").unwrap_or("text/css").trim();
    (!media.is_empty() && !media.eq_ignore_ascii_case("all"))
        || !kind.eq_ignore_ascii_case("text/css")
        || node.children().any(|child| child.is_element())
}

fn resolve(
    document: &roxmltree::Document<'_>,
    rules: &Rules,
    view: &xml::SelectorTree,
    policy: &mut Policy,
) -> Result<Vec<xml::Edit>, SvgResourceError> {
    let mut pending = vec![(document.root_element(), Arc::new(State::default()))];
    let mut edits = xml::Edits::new(document.input_text(), policy.limits.max_output_units);
    for edit in xml::empty_styles(document) {
        edits.push(edit)?;
    }
    while let Some((node, parent)) = pending.pop() {
        let (state, node_edits) = node_projection(node, &parent, rules, view, policy)?;
        for edit in node_edits {
            edits.push(edit)?;
        }
        let state = Arc::new(state);
        for child in node.children().filter(roxmltree::Node::is_element) {
            pending.push((child, state.clone()));
        }
    }
    Ok(edits.finish())
}

fn node_projection(
    node: roxmltree::Node<'_, '_>,
    parent: &State,
    rules: &Rules,
    view: &xml::SelectorTree,
    policy: &mut Policy,
) -> Result<(State, Vec<xml::Edit>), SvgResourceError> {
    let mut edits = Vec::new();
    let matched = rules.matched(&view.tree, view.handles[&node.id()], node, policy)?;
    let winners = cascade_matched_tokens(&matched.declarations);
    let (state, paint) = State::resolve(parent, &winners, policy)?;
    for (name, value) in paint {
        edits.push(xml::attribute_edit(
            node,
            &name,
            &value,
            node.document().input_text(),
            policy.limits.max_output_units,
        )?);
    }
    if let Some(edit) = inline_edit(
        node,
        &matched.declarations[matched.css_start..],
        policy.limits.max_output_units,
    )? {
        edits.push(edit);
    }
    Ok((state, edits))
}

fn inline_edit(
    node: roxmltree::Node<'_, '_>,
    matched: &[MatchedDeclaration],
    limit: usize,
) -> Result<Option<xml::Edit>, SvgResourceError> {
    let style = inline_style(matched, limit)?;
    if style.is_empty() && node.attribute("style").is_none() {
        return Ok(None);
    }
    xml::attribute_edit(node, "style", &style, node.document().input_text(), limit).map(Some)
}

fn inline_style(matched: &[MatchedDeclaration], limit: usize) -> Result<String, SvgResourceError> {
    let mut style = String::new();
    let mut units = 0usize;
    for declaration in ordered_matched_declarations(matched) {
        if state::is_paint(&declaration.property) || declaration.property.starts_with("--") {
            continue;
        }
        // Ranking is already frozen on the original tree. Removing importance
        // makes usvg's last-normal-wins store preserve this canonical order.
        let declaration = CssDeclaration {
            name: declaration.property,
            value: state::text(&declaration.value),
            important: false,
        }
        .serialized();
        units = units
            .checked_add(declaration.as_units().len())
            .filter(|&units| units <= limit)
            .ok_or(SvgResourceError::CustomPropertyBytes)?;
        let value = String::from_utf16(declaration.as_units())
            .map_err(|_| SvgResourceError::UnsupportedCustomProperties)?;
        style.push_str(&value);
    }
    Ok(style)
}

#[cfg(test)]
mod tests {
    #[test]
    fn prepared_solid_paint_is_xml_escaped_and_canonical() {
        let source = r#"<svg xmlns="http://www.w3.org/2000/svg" width="80" height="80"><rect width="80" height="80" fill="var(--missing,#ee2244)"/></svg>"#;
        let document = super::super::parse_xml(source.as_bytes()).unwrap();
        let prepared = super::prepare(&document).unwrap().unwrap();
        let document = super::super::parse_xml(prepared.as_bytes()).unwrap();
        assert!(document
            .descendants()
            .any(|node| node.attribute("fill") == Some("#ee2244")));
    }
}
