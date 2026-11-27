#![cfg(feature = "computed-style-grammar")]
use cssparser::{Parser, ParserInput};
use turbo_dom::rtdom::computed_css::grammar::{
    declaration_value, parse_media_condition, zoom_declaration,
};
#[test]
fn legacy_scroll_and_tree_counting_paths_keep_admission_contracts() {
    assert_eq!(
        declaration_value("scroll-behavior", " var(--scroll) ").as_deref(),
        Some("var(--scroll)")
    );
    assert_eq!(declaration_value("width", "sibling-index()"), None);
    assert!(declaration_value("width", "calc(sibling-index() * var(--unit))").is_some());
    assert_eq!(
        declaration_value("width", "calc(sibling-index() * sibling-future())"),
        None
    );
}
#[test]
fn invalid_if_operands_do_not_gain_deferred_admission() {
    for value in [
        "if(123: red)",
        "if(((style(--x)): red)",
        "if(style(--x) or: red)",
    ] {
        assert_eq!(declaration_value("color", value), None, "{value}");
    }
}
#[test]
fn native_zoom_declaration_rejects_non_deferred_other_values() {
    for raw in ["red", "calc(1px)", "none"] {
        let mut source = ParserInput::new(raw);
        let mut input = Parser::new(&mut source);
        assert!(zoom_declaration(&mut input).is_err(), "{raw}");
    }
}
#[test]
fn general_enclosed_lists_keep_same_operator_and_media_type_boundaries() {
    for query in [
        "(future(foo)) and (width:1px) and (height:2px)",
        "screen, (future(foo))",
        "screen and (future(foo))",
    ] {
        assert!(parse_media_condition(query).is_ok(), "{query}");
    }
    for query in [
        "(width:1px) and (height:2px) or (color:8)",
        "screen or (future(foo))",
    ] {
        assert!(parse_media_condition(query).is_err(), "{query}");
    }
}
