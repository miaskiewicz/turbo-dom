//! Shared stylesheet syntax must preserve strings without changing the legacy raw policy.
use turbo_dom::rtdom::cascade::{computed_style, get_property_value};
use turbo_dom::Tree;

#[test]
fn quoted_stylesheet_tokens_are_not_declaration_or_block_delimiters() {
    for (payload, expected) in [
        ("\"a; } { ; b\"", "\"a; } { ; b\""),
        ("\"/* literal comment */\"", "\"/* literal comment */\""),
        (
            "url(\"data:text/plain;a{b}:c\")",
            "url(\"data:text/plain;a{b}:c\")",
        ),
    ] {
        let tree = Tree::parse(&format!("<style>.target{{--token:{payload};color:rgb(1, 2, 3)}}</style><div class=target></div>"));
        let target = tree.query_selector(".target").unwrap();
        let style = computed_style(&tree, target);
        assert_eq!(get_property_value(&style, "--token"), expected);
        assert_eq!(get_property_value(&style, "color"), "rgb(1, 2, 3)");
    }
}

#[test]
fn shared_syntax_keeps_the_existing_core_raw_value_and_group_policy() {
    let tree = Tree::parse("<style>.target{width:2em;--token:var(--raw)} @media(min-width:0px){.target{width:3px}} @supports(color:red){.target{width:4px}}</style><div class=target></div>");
    let target = tree.query_selector(".target").unwrap();
    let style = computed_style(&tree, target);
    assert_eq!(get_property_value(&style, "width"), "2em");
    assert_eq!(get_property_value(&style, "--token"), "var(--raw)");
}

#[test]
fn legacy_stylesheet_comments_are_omitted_without_touching_quoted_content() {
    let tree = Tree::parse("<style>.target{color:/* note */red;width:/* basis */2em;--token:\"/* literal */\"}</style><div class=target></div>");
    let target = tree.query_selector(".target").unwrap();
    let style = computed_style(&tree, target);
    assert_eq!(get_property_value(&style, "color"), "rgb(255, 0, 0)");
    assert_eq!(get_property_value(&style, "width"), "2em");
    assert_eq!(get_property_value(&style, "--token"), "\"/* literal */\"");
}
