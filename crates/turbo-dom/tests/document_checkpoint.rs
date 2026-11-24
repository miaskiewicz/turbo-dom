use turbo_dom::rtdom::tree::FormProp;
use turbo_dom::rtdom::{DocumentCheckpoint, Tree};

fn checkpoint(tree: &Tree) -> DocumentCheckpoint {
    tree.document_checkpoint().expect("ordinary document")
}

#[test]
fn detached_react_construction_keeps_connected_document_checkpoint() {
    let mut tree = Tree::parse("<main><input checked><div id=anchor>anchor</div></main>");
    let saved = checkpoint(&tree);
    let version = tree.version;
    for _ in 0..6 {
        let div = tree.create_element("div");
        let input = tree.create_element("input");
        tree.set_attribute(div, "class", "MuiAutocomplete-inputRoot");
        tree.set_attribute(div, "style", "width:100px");
        tree.remove_attribute(div, "hidden");
        tree.set_text_content(div, "detached");
        tree.append_child(div, input);
        tree.set_form_property(input, FormProp::Checked, true);
        tree.set_attribute(input, "form", "connected-form");
        assert_eq!(checkpoint(&tree), saved);
        assert!(tree.document_unchanged_since(saved));
    }
    assert!(
        tree.version > version,
        "full storage provenance still advances"
    );
}

#[test]
fn connected_changes_and_detach_invalidate_before_ownership_is_lost() {
    let mut tree = Tree::parse("<main><input id=input><div id=anchor>old</div></main>");
    let anchor = tree.get_element_by_id("anchor").unwrap();
    let input = tree.get_element_by_id("input").unwrap();
    let mut saved = checkpoint(&tree);
    tree.set_attribute(anchor, "style", "width:360px");
    assert!(!tree.document_unchanged_since(saved));
    saved = checkpoint(&tree);
    tree.set_text_content(anchor, "new");
    assert!(!tree.document_unchanged_since(saved));
    saved = checkpoint(&tree);
    tree.set_form_property(input, FormProp::Checked, true);
    assert!(!tree.document_unchanged_since(saved));
    let detached = tree.create_element("section");
    saved = checkpoint(&tree);
    tree.append_child(detached, anchor);
    assert!(
        !tree.document_unchanged_since(saved),
        "old connected parent matters"
    );
    saved = checkpoint(&tree);
    tree.set_attribute(anchor, "style", "width:12px");
    assert!(tree.document_unchanged_since(saved));
    let main = tree.query_selector("main").unwrap();
    tree.append_child(main, detached);
    assert!(!tree.document_unchanged_since(saved));
}

#[test]
fn foreign_tree_untracked_version_and_shadow_refuse_equivalence() {
    let mut tree = Tree::parse("<div id=host></div>");
    let saved = checkpoint(&tree);
    let other = Tree::parse("<div id=host></div>");
    assert!(!other.document_unchanged_since(saved));
    tree.version += 1;
    assert!(tree.document_checkpoint().is_none());
    assert!(!tree.document_unchanged_since(saved));
    tree.create_element("div");
    assert!(!tree.document_unchanged_since(saved));
    let host = tree.get_element_by_id("host").unwrap();
    tree.attach_shadow(host);
    assert!(tree.document_checkpoint().is_none());
}

#[test]
fn checkpoint_covers_connected_utf16_attributes_states_and_global_invalidation() {
    let mut tree = Tree::parse("<main id=target><input id=input><x-control></x-control></main>");
    let target = tree.get_element_by_id("target").unwrap();
    let input = tree.get_element_by_id("input").unwrap();
    let state: Vec<_> = "active".encode_utf16().collect();
    let mut saved = checkpoint(&tree);
    tree.set_custom_state_utf16(target, &state, true);
    assert!(!tree.document_unchanged_since(saved));
    saved = checkpoint(&tree);
    tree.clear_custom_states(target);
    assert!(!tree.document_unchanged_since(saved));
    saved = checkpoint(&tree);
    tree.set_text_content_utf16(target, &[0xd800, 0x61]);
    assert!(!tree.document_unchanged_since(saved));
    saved = checkpoint(&tree);
    let attr = tree.create_attribute(None, "data-proof", "changed");
    tree.set_attribute_node(target, attr).unwrap();
    assert!(!tree.document_unchanged_since(saved));
    saved = checkpoint(&tree);
    tree.set_attribute_value(attr, "again");
    assert!(!tree.document_unchanged_since(saved));
    saved = checkpoint(&tree);
    tree.invalidate_style(target, true);
    assert!(
        !tree.document_unchanged_since(saved),
        "quiet style invalidation is conservative"
    );
    saved = checkpoint(&tree);
    tree.set_fullscreen_stack(&[input]);
    assert!(!tree.document_unchanged_since(saved));
}

#[test]
fn storage_collection_and_replacement_never_reuse_a_checkpoint() {
    use turbo_dom::rtdom::tree::CollectionBudget;
    let mut tree = Tree::parse("<main>retained</main>");
    tree.create_element("garbage");
    let saved = checkpoint(&tree);
    let version = tree.version;
    tree.collect_unreachable(
        &[],
        CollectionBudget {
            max_nodes: 100,
            max_edges: 1000,
            max_payload_bytes: 10000,
        },
    )
    .unwrap();
    assert_eq!(
        tree.version, version,
        "collection retains the existing provenance contract"
    );
    assert!(!tree.document_unchanged_since(saved));
    tree = Tree::parse("<main>retained</main>");
    assert!(!tree.document_unchanged_since(saved));
}

#[test]
fn detached_namespaced_nodes_utf16_and_fragments_preserve_checkpoint() {
    use turbo_dom::rtdom::tree::Namespace;
    let mut tree = Tree::parse("<main>connected</main>");
    let saved = checkpoint(&tree);
    let fragment = tree.create_document_fragment();
    let svg = tree.create_element_ns("svg", Namespace::Svg);
    tree.append_child(fragment, svg);
    let text = tree.create_text_node_utf16(&[0xd800]);
    tree.append_child(svg, text);
    tree.set_node_value(text, "changed");
    assert!(tree.document_unchanged_since(saved));
    tree.create_element_units(&[0x64, 0x69, 0x76]);
    tree.create_element_namespace_units(
        Some(
            &"http://www.w3.org/2000/svg"
                .encode_utf16()
                .collect::<Vec<_>>(),
        ),
        &[0x73, 0x76, 0x67],
    );
    tree.create_comment_utf16(&[0xd800]);
    tree.create_cdata_section_utf16(&[0xd800]);
    tree.create_processing_instruction("test", "data");
    tree.create_comment("comment");
    assert!(tree.document_unchanged_since(saved));
}

#[test]
fn checkpoint_does_not_inherit_selector_filter_omissions() {
    use turbo_dom::rtdom::StyleFilter;
    let mut tree = Tree::parse("<div id=target></div>");
    let target = tree.get_element_by_id("target").unwrap();
    tree.set_style_filter(Some(StyleFilter::default()));
    let saved = checkpoint(&tree);
    tree.set_attribute(target, "data-unreferenced", "new");
    assert!(!tree.document_unchanged_since(saved));
    let saved = checkpoint(&tree);
    let style = tree.create_element("style");
    tree.set_attribute(style, "media", "screen");
    assert!(
        !tree.document_unchanged_since(saved),
        "stylesheet owner invalidation remains conservative"
    );
}

#[test]
fn malformed_low_level_document_reparenting_never_proves_equivalence() {
    let mut tree = Tree::parse("<main>connected</main>");
    let saved = checkpoint(&tree);
    let detached = tree.create_element("div");
    tree.append_child(detached, tree.root());
    assert!(tree.document_checkpoint().is_none());
    assert!(!tree.document_unchanged_since(saved));
    tree.remove_child(detached, tree.root());
    assert!(!tree.document_unchanged_since(saved));
}
