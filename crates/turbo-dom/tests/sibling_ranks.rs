use turbo_dom::rtdom::{
    query::{LiveNavigator, Navigator},
    tree::{FormProp, Namespace},
    Tree,
};

#[test]
fn structural_ranks_follow_reordering_reparenting_and_detachment() {
    let mut tree = Tree::parse(
        "<main>text<span id=a></span><!--x--><b></b><span id=b></span></main><aside></aside>",
    );
    let main = tree.query_selector("main").unwrap();
    let aside = tree.query_selector("aside").unwrap();
    let a = tree.query_selector("#a").unwrap();
    let b = tree.query_selector("#b").unwrap();
    assert!(tree.matches_scoped(
        b,
        ":scope > span:nth-child(3):nth-of-type(2):last-of-type:last-child",
        main
    ));
    assert!(tree.matches(
        a,
        ":first-child:first-of-type:nth-last-child(3):nth-last-of-type(2)"
    ));
    tree.insert_before(main, b, Some(a));
    assert!(tree.matches(b, ":first-child:first-of-type:nth-last-of-type(2)"));
    assert!(tree.matches(a, ":nth-child(2):last-of-type:nth-of-type(2)"));
    tree.append_child(aside, a);
    assert!(tree.matches_scoped(a, ":scope > :only-child:only-of-type", aside));
    assert!(tree.matches(b, ":first-child:only-of-type"));
    tree.remove_child(aside, a);
    assert!(tree.matches(a, ":only-child:only-of-type"));
    tree.append_child(main, a);
    assert!(!tree.matches(a, ":only-of-type"));
    let fragment = tree.create_document_fragment();
    tree.append_child(fragment, a);
    assert!(tree.matches(a, ":only-child:only-of-type"));
    let sibling = tree.create_element("span");
    tree.append_child(fragment, sibling);
    assert!(tree.matches(a, ":first-child:nth-last-of-type(2)"));
    tree.remove_child(fragment, sibling);
    assert!(tree.matches(a, ":only-child:only-of-type"));
    let shadow = tree.attach_shadow(aside);
    tree.append_child(shadow, b);
    assert!(tree.matches_scoped(b, ":only-child:only-of-type", aside));
    tree.append_child(shadow, a);
    assert!(tree.matches(a, ":nth-of-type(2):nth-last-child(1)"));
}

#[test]
fn element_type_uses_exact_namespace_local_name_and_utf16_units() {
    let mut tree = Tree::parse("<main></main>");
    let parent = tree.query_selector("main").unwrap();
    let html = tree.create_element_ns("x", Namespace::Html);
    let svg = tree.create_element_ns("x", Namespace::Svg);
    let math = tree.create_element_ns("x", Namespace::MathMl);
    let upper = tree.create_element_ns("X", Namespace::Html);
    let first = tree.create_element_namespace(Some("urn:a"), "p:x");
    let second = tree.create_element_namespace(Some("urn:a"), "q:x");
    let other = tree.create_element_namespace(Some("urn:b"), "x");
    let null = tree.create_element_namespace(None, "x");
    let lone = tree.create_element_ns_units(&[0xd800], Namespace::Html);
    let lone2 = tree.create_element_ns_units(&[0xd800], Namespace::Html);
    let different = tree.create_element_ns_units(&[0xd801], Namespace::Html);
    let replacement = tree.create_element_ns_units(&[0xfffd], Namespace::Html);
    let uri1 = tree.create_element_namespace_units(Some(&[0xd800]), &[120]);
    let uri2 = tree.create_element_namespace_units(Some(&[0xd801]), &[120]);
    for node in [
        html,
        svg,
        math,
        upper,
        first,
        second,
        other,
        null,
        lone,
        lone2,
        different,
        replacement,
        uri1,
        uri2,
    ] {
        tree.append_child(parent, node);
    }
    for node in [
        html,
        svg,
        math,
        upper,
        other,
        null,
        different,
        replacement,
        uri1,
        uri2,
    ] {
        assert!(tree.matches(node, ":only-of-type"), "{node:?}");
    }
    for (first, second) in [(first, second), (lone, lone2)] {
        assert!(tree.matches(first, ":nth-of-type(1):nth-last-of-type(2)"));
        assert!(tree.matches(second, ":nth-of-type(2):nth-last-of-type(1)"));
    }
}

#[test]
fn filtered_siblings_and_dirty_form_state_are_not_cached_verdicts() {
    let mut tree = Tree::parse("<main><input class=flag type=checkbox><input class=flag type=checkbox><input type=checkbox></main>");
    let nodes = tree.query_selector_all("input");
    assert!(tree.matches(nodes[1], ":nth-of-type(2):nth-child(2 of .flag)"));
    tree.set_attribute(nodes[0], "class", "");
    assert!(tree.matches(nodes[1], ":nth-child(1 of .flag)"));
    tree.set_form_property(nodes[0], FormProp::Checked, true);
    tree.set_form_property(nodes[1], FormProp::Checked, true);
    assert!(tree.matches(nodes[1], ":nth-of-type(2):nth-child(2 of :checked)"));
    tree.set_form_property(nodes[0], FormProp::Checked, false);
    assert!(tree.matches(nodes[1], ":nth-of-type(2):nth-child(1 of :checked)"));
    assert!(!tree.matches(nodes[0], ":checked"));
}

#[test]
fn navigator_handles_parentless_and_non_element_nodes() {
    let mut tree = Tree::parse("<main>text<span></span></main><aside>only text</aside>");
    let main = tree.query_selector("main").unwrap();
    let aside = tree.query_selector("aside").unwrap();
    let detached = tree.create_text_node("detached");
    let text = tree.first_child(main).unwrap();
    let only_text = tree.first_child(aside).unwrap();
    let nav = LiveNavigator(&tree);
    assert_eq!(nav.child_position(detached), (1, 1));
    assert_eq!(nav.type_position(detached), (1, 1));
    assert_eq!(nav.child_position(text), (0, 1));
    assert_eq!(nav.child_position(only_text), (0, 0));
    for node in [text, only_text] {
        assert_eq!(nav.type_position(node), (0, 0));
    }
}
