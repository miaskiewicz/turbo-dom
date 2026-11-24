//! Stylesheet selector metadata must retain raw quoted DOM-string units.
use std::borrow::Cow;
use turbo_dom::rtdom::query::{
    match_for_cascade, parse_for_cascade, parse_for_cascade_units, LiveNavigator,
};
use turbo_dom::Tree;

fn units(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

#[test]
fn raw_quoted_units_do_not_alias_scalar_replacement_or_escaped_units() {
    let mut tree = Tree::parse("<div id=raw></div><div id=repaired></div>");
    let raw = tree.query_selector("#raw").unwrap();
    let repaired = tree.query_selector("#repaired").unwrap();
    tree.set_attribute_utf16(raw, "data-x", &[0xd800]);
    tree.set_attribute_utf16(repaired, "data-x", &[0xfffd]);
    let mut source = units("[data-x=\"");
    source.push(0xd800);
    source.extend(units("\"]"));
    let list = parse_for_cascade_units(&source).unwrap();
    let selector = &list.selectors()[0];
    assert_eq!(selector.text_units().as_ref(), source);
    assert_eq!(selector.specificity(), (0, 1, 0));
    assert!(match_for_cascade(&LiveNavigator(&tree), raw, selector));
    assert!(!match_for_cascade(
        &LiveNavigator(&tree),
        repaired,
        selector
    ));
    let scalar = parse_for_cascade(&String::from_utf16_lossy(&source)).unwrap();
    assert!(!match_for_cascade(
        &LiveNavigator(&tree),
        raw,
        &scalar.selectors()[0]
    ));
    assert!(match_for_cascade(
        &LiveNavigator(&tree),
        repaired,
        &scalar.selectors()[0]
    ));
    let escaped = parse_for_cascade_units(&units("[data-x=\"\\d800 \"]")).unwrap();
    assert!(match_for_cascade(
        &LiveNavigator(&tree),
        repaired,
        &escaped.selectors()[0]
    ));
    assert!(!match_for_cascade(
        &LiveNavigator(&tree),
        raw,
        &escaped.selectors()[0]
    ));
    source.clear();
    assert!(selector.text_units().contains(&0xd800));
    tree.set_attribute_utf16(raw, "data-x", &[0xfffd]);
    assert!(!match_for_cascade(&LiveNavigator(&tree), raw, selector));
}

#[test]
fn identifier_replacement_and_literal_supplementary_names_stay_distinct() {
    let mut tree = Tree::parse("<div id=raw></div><div id=repaired></div><div id=marker></div>");
    let raw = tree.query_selector("#raw").unwrap();
    let repaired = tree.query_selector("#repaired").unwrap();
    let marker = tree.query_selector("#marker").unwrap();
    tree.set_attribute_utf16(raw, "class", &[120, 0xd800]);
    tree.set_attribute_utf16(repaired, "class", &[120, 0xfffd]);
    tree.set_attribute(marker, "class", "x\u{f0000}\u{f0800}");
    let list = parse_for_cascade_units(&[46, 120, 0xd800]).unwrap();
    assert!(match_for_cascade(
        &LiveNavigator(&tree),
        repaired,
        &list.selectors()[0]
    ));
    assert!(!match_for_cascade(
        &LiveNavigator(&tree),
        raw,
        &list.selectors()[0]
    ));
    let literal = parse_for_cascade_units(&units(".x\u{f0000}\u{f0800}")).unwrap();
    assert!(match_for_cascade(
        &LiveNavigator(&tree),
        marker,
        &literal.selectors()[0]
    ));
    assert!(!match_for_cascade(
        &LiveNavigator(&tree),
        repaired,
        &literal.selectors()[0]
    ));
}

#[test]
fn unit_metadata_keeps_pseudo_specificity_and_stylesheet_validity() {
    let parsed = parse_for_cascade_units(&units("[data-x='v']::before,.x")).unwrap();
    assert_eq!(parsed.selectors()[0].pseudo_element(), Some("before"));
    assert_eq!(parsed.selectors()[0].specificity(), (0, 1, 1));
    assert_eq!(parsed.selectors()[1].specificity(), (0, 1, 0));
    assert!(matches!(
        parsed.selectors()[0].text_units(),
        Cow::Borrowed(_)
    ));
    let old = parse_for_cascade(".x").unwrap();
    assert_eq!(old.selectors()[0].text_units().as_ref(), units(".x"));
    for invalid in [".x,", "div/**/p", "[data-x=\"open", ":unknown-pseudo"] {
        assert!(
            parse_for_cascade_units(&units(invalid)).is_err(),
            "{invalid}"
        );
    }
}
