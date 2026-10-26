//! Style-system tests (§4): CSS parsing, selector matching + specificity,
//! tokens, node-style injection, the 6-level cascade, and inheritance.
//!
//! Split at the S1 relocation: the cases that need `TKind` directives, templating or the
//! PDF emitter stay in turbo-html2pdf-core's `tests/style.rs`.

mod common;

use turbo_dom_layout::style::{
    build_cascade, parse_stylesheet, StyledElement, StyledNode, TokenSet,
};

fn find<'a>(
    nodes: &'a [StyledNode],
    pred: &dyn Fn(&StyledElement) -> bool,
) -> Option<&'a StyledElement> {
    for node in nodes {
        if let StyledNode::Element(el) = node {
            if pred(el) {
                return Some(el);
            }
            if let Some(found) = find(&el.children, pred) {
                return Some(found);
            }
        }
    }
    None
}

fn by_id<'a>(nodes: &'a [StyledNode], id: &str) -> &'a StyledElement {
    find(nodes, &|el| {
        el.attrs.iter().any(|a| a.name == "id" && a.value == id)
    })
    .expect("element by id")
}

fn prop(nodes: &[StyledNode], id: &str, property: &str) -> Option<String> {
    by_id(nodes, id).style.get(property).map(str::to_string)
}

// --------------------------------------------------------------------------
// parser
// --------------------------------------------------------------------------

#[test]
fn parses_rules_comments_and_at_rules() {
    let sheet = parse_stylesheet(
        "/* c */ .a { color: red; font-weight: 700 !important } @page { size: A4 }",
    );
    assert_eq!(sheet.rules.len(), 1);
    assert_eq!(sheet.rules[0].declarations.len(), 2);
    assert!(sheet.rules[0].declarations[1].important);
    assert_eq!(sheet.at_rules.len(), 1);
    assert_eq!(sheet.at_rules[0].name, "page");
    assert_eq!(sheet.at_rules[0].body, "size: A4");
}

#[test]
fn malformed_declarations_are_skipped() {
    let sheet = parse_stylesheet(".a { ; color: red; novalue: ; :bad ; width: 10px }");
    assert_eq!(sheet.rules[0].declarations.len(), 2);
    let empty = parse_stylesheet("{ color: red }");
    assert!(empty.rules.is_empty());
}

#[test]
fn unterminated_comment_consumes_rest() {
    let sheet = parse_stylesheet(".a { color: red } /* dangling");
    assert_eq!(sheet.rules.len(), 1);
}

#[test]
fn nested_braces_in_at_rule_stay_intact() {
    let sheet = parse_stylesheet("@media print { .a { color: red } }");
    assert_eq!(sheet.rules.len(), 0);
    assert_eq!(sheet.at_rules.len(), 1);
    assert_eq!(sheet.at_rules[0].name, "media");
    assert!(sheet.at_rules[0].body.contains(".a"));
}

#[test]
fn at_rules_remember_their_source_position() {
    let mut sheet = parse_stylesheet(
        "@media screen { .a { color: red } } .b { color: blue } @page { size: A4 } .c { x: y }",
    );
    assert_eq!(sheet.rules.len(), 2);
    assert_eq!(sheet.at_rule_position(0), 0);
    assert_eq!(sheet.at_rule_position(1), 1);
    // An at-rule pushed by hand has no recorded position: it follows every rule.
    sheet.at_rules.push(sheet.at_rules[1].clone());
    assert_eq!(sheet.at_rule_position(2), 2);
    // A recorded position never points past the rules a caller kept.
    sheet.rules.clear();
    assert_eq!(sheet.at_rule_position(1), 0);
}

#[test]
fn conditional_rules_cascade_at_their_source_position() {
    // miaskiewicz/turbo-dom#24 item 1 (Chrome 156: crates/turbo-dom-layout/tests/
    // fixtures/html_document_chrome156.json `media-*` / `supports-*` cases).
    let order = |css: &str| -> Vec<String> {
        build_cascade(css, "", TokenSet::default())
            .rules
            .iter()
            .filter(|r| r.level == 1)
            .map(|r| r.rule.selectors.prelude().to_string())
            .collect()
    };
    assert_eq!(
        order(
            "@media screen { .m { a: b } } .p { a: b } @supports (display: grid) { .s { a: b } }"
        ),
        [".m", ".p", ".s"]
    );
    assert_eq!(
        order(
            "@supports (display: block) { @media screen { .n { a: b } } .s { a: b } } .p { a: b }"
        ),
        [".n", ".s", ".p"]
    );
    // A block whose condition fails keeps nothing, and leaves the order of the rest.
    assert_eq!(
        order(".p { a: b } @media print { .x { a: b } } .q { a: b } @font-face { a: b }"),
        [".p", ".q"]
    );
}

#[test]
fn empty_selector_list_produces_no_rule() {
    let sheet = parse_stylesheet(", { color: red }");
    assert!(sheet.rules.is_empty());
}

#[test]
fn root_pseudo_matches_html_element() {
    // `:root` (cascade.rs Root → ctx.tag == Some("html")). The compiler strips the
    // `<html>` wrapper before render, so build a tree with a top-level html element
    // by hand and style it directly.
    use turbo_dom_layout::test_support::literal::{Element, Node};
    use turbo_dom_layout::{Attr, Tag};
    let html = Node::Element(Element {
        tag: Tag::Html("html".to_string()),
        attrs: vec![Attr {
            name: "id".to_string(),
            value: "r".to_string(),
        }],
        children: vec![Node::Text("x".to_string())],
    });
    let cascade = build_cascade(":root { color: teal }", "", TokenSet::new());
    let styled = common::style_nodes(&[html], &cascade);
    assert_eq!(prop(&styled, "r", "color").as_deref(), Some("teal"));
}
