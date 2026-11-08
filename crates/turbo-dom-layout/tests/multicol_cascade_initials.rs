//! miaskiewicz/turbo-dom#398 PR-A: the cascade knows multi-column initial values, so
//! `column-count: initial` resolves to `auto` (and the other non-inherited multicol / break
//! properties resolve to their CSS-defined initials), and an explicit `column-count: 3`
//! survives to the styled element's `ComputedStyle` untouched.
use turbo_dom_layout::{parse_html_tree, style_html_tree, HtmlOptions, StyledNode};

/// Multi-column / fragmentation initials as specified by CSS Multi-column 1 and CSS
/// Fragmentation 3 (all non-inherited). The cascade reads these for the `initial` and
/// `unset` CSS-wide keywords -- turbo-dom#398's INITIAL table.
const MULTICOL_INITIALS: &[(&str, &str)] = &[
    ("column-count", "auto"),
    ("column-width", "auto"),
    ("column-gap", "normal"),
    ("column-rule-width", "medium"),
    ("column-rule-style", "none"),
    ("column-rule-color", "currentcolor"),
    ("column-span", "none"),
    ("column-fill", "balance"),
    ("break-before", "auto"),
    ("break-after", "auto"),
    ("break-inside", "auto"),
];

fn find_target<'a>(node: &'a StyledNode, id: &str) -> Option<&'a turbo_dom_layout::StyledElement> {
    let element = node.as_element()?;
    for attr in &element.attrs {
        if attr.name == "id" && attr.value == id {
            return Some(element);
        }
    }
    for child in &element.children {
        if let Some(e) = find_target(child, id) {
            return Some(e);
        }
    }
    None
}

#[test]
fn initial_keyword_resolves_multicol_defaults() {
    use std::fmt::Write as _;
    let declarations: String =
        MULTICOL_INITIALS
            .iter()
            .fold(String::new(), |mut acc, (name, _)| {
                let _ = write!(acc, "{name}: initial;");
                acc
            });
    let html = format!(
        "<!doctype html><html><body><div id=t style='{declarations}'>x</div></body></html>"
    );
    let tree = parse_html_tree(&html);
    let styled = style_html_tree(&tree, "", 1280.0, &HtmlOptions::default());
    let target = styled
        .iter()
        .find_map(|node| find_target(node, "t"))
        .expect("target div");
    for (name, want) in MULTICOL_INITIALS {
        assert_eq!(
            target.style.get(name),
            Some(*want),
            "initial for {name} should be {want:?}"
        );
    }
}

#[test]
fn explicit_column_count_survives_cascade() {
    let tree = parse_html_tree(
        "<!doctype html><html><body><div id=t style='column-count: 3'>x</div></body></html>",
    );
    let styled = style_html_tree(&tree, "", 1280.0, &HtmlOptions::default());
    let target = styled
        .iter()
        .find_map(|node| find_target(node, "t"))
        .expect("target div");
    assert_eq!(target.style.get("column-count"), Some("3"));
}
