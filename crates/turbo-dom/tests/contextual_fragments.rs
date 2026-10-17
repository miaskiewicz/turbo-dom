use turbo_dom::rtdom::{serialize::serialize_inner, Tree};

fn units(value: &str) -> Vec<u16> {
    value.encode_utf16().collect()
}

#[test]
fn contextual_fragments_preserve_actual_context_and_pending_mutations() {
    let mut tree=Tree::parse("<!doctype html><body><form><table><tbody><tr><td>existing</td></tr></tbody></table><div id=context><i>keep</i></div></form></body>");
    let context = tree.query_selector("#context").unwrap();
    let tbody = tree.query_selector("tbody").unwrap();
    let existing = tree.children(context);
    tree.start_recording();
    tree.set_attribute(context, "data-before", "yes");
    let fragment = tree.create_contextual_fragment_utf16(
        tbody,
        &units("<tr><td data-new=yes>new</td></tr>"),
        true,
    );
    assert_eq!(
        serialize_inner(&tree, fragment),
        "<tr><td data-new=\"yes\">new</td></tr>"
    );
    assert_eq!(tree.children(context), existing);
    assert_eq!(serialize_inner(&tree, tbody), "<tr><td>existing</td></tr>");
    let form = tree.create_contextual_fragment_utf16(
        context,
        &units("<form id=nested><b>child</b></form>"),
        true,
    );
    assert_eq!(serialize_inner(&tree, form), "<b>child</b>");
    assert_eq!(tree.take_mutation_events().len(), 1);
    assert_eq!(tree.parent(fragment), None);
}

#[test]
fn contextual_fragments_preserve_namespace_attributes_and_original_units() {
    let mut tree=Tree::parse("<!doctype html><body><svg><foreignObject id=html></foreignObject><g id=svg></g></svg><math><annotation-xml id=math encoding=text/html></annotation-xml></math>");
    let html = tree.query_selector("#html").unwrap();
    let svg = tree.query_selector("#svg").unwrap();
    let math = tree.query_selector("#math").unwrap();
    let fragment = tree.create_contextual_fragment_utf16(html, &units("<p>html</p>"), true);
    assert_eq!(
        tree.namespace_uri(tree.first_child(fragment).unwrap()),
        Some("http://www.w3.org/1999/xhtml")
    );
    let fragment = tree.create_contextual_fragment_utf16(svg, &units("<circle></circle>"), true);
    assert_eq!(
        tree.namespace_uri(tree.first_child(fragment).unwrap()),
        Some("http://www.w3.org/2000/svg")
    );
    let fragment = tree.create_contextual_fragment_utf16(math, &units("<p>html</p>"), true);
    assert_eq!(
        tree.namespace_uri(tree.first_child(fragment).unwrap()),
        Some("http://www.w3.org/1999/xhtml")
    );
    let mut markup = units("<text>");
    markup.extend([0xd800, 0xfeff]);
    markup.extend(units("</text>"));
    let fragment = tree.create_contextual_fragment_utf16(svg, &markup, true);
    let text = tree
        .first_child(tree.first_child(fragment).unwrap())
        .unwrap();
    assert_eq!(tree.text_content_utf16(text), [0xd800, 0xfeff]);
}

#[test]
fn contextual_fragment_scripting_selects_noscript_tokenization_without_execution() {
    let mut tree = Tree::parse("<!doctype html><body><div id=context></div>");
    let context = tree.query_selector("#context").unwrap();
    let source=units("<noscript><b>fallback</b></noscript><script>throw Error('never execute')</script><i>after</i>");
    let scripted = tree.create_contextual_fragment_utf16(context, &source, true);
    let disabled = tree.create_contextual_fragment_utf16(context, &source, false);
    let noscript = tree.first_child(scripted).unwrap();
    assert_eq!(tree.node_type_id(tree.first_child(noscript).unwrap()), 3);
    let noscript = tree.first_child(disabled).unwrap();
    assert_eq!(
        tree.local_name(tree.first_child(noscript).unwrap()),
        Some("b")
    );
    for fragment in [scripted, disabled] {
        let children = tree.children(fragment);
        assert_eq!(children.len(), 3);
        assert_eq!(tree.local_name(children[1]), Some("script"));
        assert_eq!(tree.local_name(children[2]), Some("i"));
        assert_eq!(tree.text_content(children[2]), "after");
    }
}

#[test]
fn contextual_fragment_inherits_context_document_quirks_mode() {
    let source = units("<p>before<table><tr><td>cell</td></tr></table>");
    let mut standards = Tree::parse("<!doctype html><body><div id=context></div>");
    let mut quirks = Tree::parse("<body><div id=context></div>");
    let context = standards.query_selector("#context").unwrap();
    let fragment = standards.create_contextual_fragment_utf16(context, &source, true);
    assert_eq!(standards.children(fragment).len(), 2);
    let context = quirks.query_selector("#context").unwrap();
    let fragment = quirks.create_contextual_fragment_utf16(context, &source, true);
    assert_eq!(quirks.children(fragment).len(), 1);
    assert_eq!(
        quirks.local_name(quirks.first_child(fragment).unwrap()),
        Some("p")
    );
    assert_eq!(
        quirks.local_name(
            *quirks
                .children(quirks.first_child(fragment).unwrap())
                .last()
                .unwrap()
        ),
        Some("table")
    );
}
