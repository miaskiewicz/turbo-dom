use super::*;

fn style() -> ComputedStyle {
    ComputedStyle::from_pairs([
        ("color", "red"),
        ("margin-left", "4px"),
        ("margin-right", "5px"),
        ("padding-left", "6px"),
        ("border-left-width", "2px"),
    ])
}

#[test]
fn unchanged_style_clones_share_property_storage() {
    let original = style();
    let mut cloned = original.clone();
    assert!(Arc::ptr_eq(&original.map, &cloned.map));
    cloned.set("color", "red".to_owned());
    assert_eq!(cloned.remove("absent-property"), None);
    assert!(Arc::ptr_eq(&original.map, &cloned.map));
    assert_eq!(original, cloned);
    assert_eq!(
        original.iter().collect::<Vec<_>>(),
        cloned.iter().collect::<Vec<_>>()
    );
}

#[test]
fn setting_one_clones_properties_without_mutating_shared_styles() {
    let original = style();
    let mut changed = original.clone();
    changed.set("color", "blue".to_owned());
    assert!(!Arc::ptr_eq(&original.map, &changed.map));
    assert_eq!(original.get("color"), Some("red"));
    assert_eq!(changed.get("color"), Some("blue"));
    assert_ne!(original, changed);
    assert_eq!(original, style());
}

#[test]
fn removing_one_clones_properties_without_mutating_shared_styles() {
    let original = style();
    let mut changed = original.clone();
    assert_eq!(changed.remove("color"), Some("red".to_owned()));
    assert!(!Arc::ptr_eq(&original.map, &changed.map));
    assert_eq!(original.get("color"), Some("red"));
    assert_eq!(changed.get("color"), None);
    assert_eq!(original, style());
}

#[test]
fn fragmented_inline_edges_do_not_change_the_source_style() {
    let original = style();
    let fragment = original.inline_fragment_style(false, true);
    assert!(!Arc::ptr_eq(&original.map, &fragment.map));
    assert_eq!(fragment.get("margin-left"), Some("0px"));
    assert_eq!(fragment.get("padding-left"), Some("0px"));
    assert_eq!(fragment.get("border-left-width"), Some("0px"));
    assert_eq!(fragment.get("margin-right"), Some("5px"));
    assert_eq!(original, style());
    let retained = original.inline_fragment_style(true, true);
    assert!(Arc::ptr_eq(&original.map, &retained.map));
}

#[test]
fn inherited_custom_properties_are_independent_after_cascade_resolution() {
    let mut parent = ComputedStyle::from_pairs([("--brand", "red"), ("font-size", "20px")]);
    let child = inherit(
        BTreeMap::from([("color".to_owned(), "var(--brand)".to_owned())]),
        &parent,
    );
    parent.set("--brand", "blue".to_owned());
    assert_eq!(child.get("--brand"), Some("red"));
    assert_eq!(child.get("color"), Some("red"));
    assert_eq!(child.get("font-size"), Some("20px"));
    assert_eq!(parent.get("--brand"), Some("blue"));
}
