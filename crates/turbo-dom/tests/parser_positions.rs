use turbo_dom::rtdom::serialize::serialize_outer;
use turbo_dom::rtdom::{SourcePosition, Tree};

#[test]
fn original_utf16_offsets_and_crlf_columns_are_preserved() {
    let tree = Tree::parse_with_source_positions("😀\r\n<style>x{}</style><div style='x'>");
    let style = tree.query_selector("style").unwrap();
    let div = tree.query_selector("div").unwrap();
    assert_eq!(
        tree.source_position(style),
        Some(SourcePosition {
            utf16_offset: 11,
            line: 2,
            column: 8
        })
    );
    assert_eq!(
        tree.source_position(div),
        Some(SourcePosition {
            utf16_offset: 37,
            line: 2,
            column: 34
        })
    );
    for name in ["html", "head", "body"] {
        assert_eq!(
            tree.source_position(tree.query_selector(name).unwrap()),
            None,
            "implied {name}"
        );
    }
}
#[test]
fn duplicate_sources_have_distinct_actual_token_positions() {
    let tree =
        Tree::parse_with_source_positions("<!doctype html><style>a{}</style><style>a{}</style>");
    let styles = tree.query_selector_all("style");
    assert_eq!(
        tree.source_position(styles[0]),
        Some(SourcePosition {
            utf16_offset: 22,
            line: 1,
            column: 23
        })
    );
    assert_eq!(
        tree.source_position(styles[1]),
        Some(SourcePosition {
            utf16_offset: 40,
            line: 1,
            column: 41
        })
    );
}
#[test]
fn unit_transport_markers_and_lone_units_do_not_inflate_positions() {
    let mut units = vec![0xd800];
    units.extend("\u{f0000}😀".encode_utf16());
    units.push(0xdc00);
    units.extend("<div style='a&#10;b'>".encode_utf16());
    let tree = Tree::parse_utf16_with_source_positions(&units);
    let div = tree.query_selector("div").unwrap();
    assert_eq!(
        tree.source_position(div),
        Some(SourcePosition {
            utf16_offset: 27,
            line: 1,
            column: 28
        })
    );
    assert_eq!(
        tree.text_content_utf16(tree.query_selector("body").unwrap()),
        units[..6]
    );
    assert_eq!(
        tree.get_attribute_utf16(div, "style"),
        Some("a\nb".encode_utf16().collect())
    );
}
#[test]
fn reconstructed_formatting_and_synthetic_table_nodes_have_no_lexical_span() {
    let tree =
        Tree::parse_with_source_positions("<p><b class=x>1</p>2<table><tr><td>3</td></tr></table>");
    let bold = tree.query_selector_all("b");
    assert_eq!(bold.len(), 2);
    assert!(tree.source_position(bold[0]).is_some());
    assert_eq!(tree.source_position(bold[1]), None);
    assert_eq!(
        tree.source_position(tree.query_selector("tbody").unwrap()),
        None
    );
    assert!(tree
        .source_position(tree.query_selector("tr").unwrap())
        .is_some());
}
#[test]
fn optional_provenance_preserves_existing_tree_construction() {
    for html in [
        "<table><div style='x'>foster</div><tr><td>a</table>",
        "<p><b>one<i>two</b>three</i>",
        "<template><table><td>contents</template>",
        "<svg><foreignObject><style>p{}</style><div style='x'></div></foreignObject></svg>",
        "<select><option><div>ignored</div><optgroup><option>x",
        "<form><form><div style='x'>same</div></form>",
        "<style> &amp; <style> x </style>",
        "<noscript><style>a{}</style></noscript>",
        "<html><head><head><body><body><div>duplicate",
        "<div a='&notit;' style='a&#13;&#10;b'>text&#0;</div>",
        "<div style='x'",
        "<!doctype foo><body>quirks",
    ] {
        let original = Tree::parse(html);
        let tracked = Tree::parse_with_source_positions(html);
        assert_eq!(
            serialize_outer(&tracked, tracked.root()),
            serialize_outer(&original, original.root()),
            "{html}"
        );
        assert_eq!(tracked.compat_mode(), original.compat_mode(), "{html}");
    }
}

#[test]
fn same_tag_reconstruction_does_not_steal_the_actual_token_position() {
    let tree = Tree::parse_with_source_positions("<p><b style='x'>1</p><b style='x'>2");
    let nodes = tree.query_selector_all("b");
    assert_eq!(nodes.len(), 3);
    assert_eq!(
        tree.source_position(nodes[0]),
        Some(SourcePosition {
            utf16_offset: 16,
            line: 1,
            column: 17
        })
    );
    assert_eq!(tree.source_position(nodes[1]), None);
    assert_eq!(
        tree.source_position(nodes[2]),
        Some(SourcePosition {
            utf16_offset: 34,
            line: 1,
            column: 35
        })
    );
}
#[test]
fn metadata_does_not_retain_detached_storage_or_invent_created_positions() {
    use turbo_dom::rtdom::tree::CollectionBudget;
    let mut tree = Tree::parse_with_source_positions("<div><style>a{}</style></div>");
    let div = tree.query_selector("div").unwrap();
    let style = tree.query_selector("style").unwrap();
    let position = tree.source_position(style);
    assert!(position.is_some());
    let clone = tree.create_element("style");
    tree.set_text_content(clone, &tree.text_content(style));
    assert_eq!(tree.source_position(clone), None);
    tree.remove_child(tree.parent(div).unwrap(), div);
    tree.remove_child(div, style);
    tree.collect_unreachable(
        &[style],
        CollectionBudget {
            max_nodes: 256,
            max_edges: 4096,
            max_payload_bytes: 1024 * 1024,
        },
    )
    .unwrap();
    assert_eq!(tree.source_position(style), position);
    assert_eq!(tree.source_position(div), None);
    assert_eq!(tree.source_position(clone), None);
}

#[test]
fn attributes_added_by_later_body_tags_keep_their_own_token_position() {
    let tree = Tree::parse_with_source_positions("<body id=x><body style='x' id=y><div>z</div>");
    let body = tree.query_selector("body").unwrap();
    assert_eq!(
        tree.source_position(body),
        Some(SourcePosition {
            utf16_offset: 11,
            line: 1,
            column: 12
        })
    );
    assert_eq!(
        tree.source_attribute_position(body, "id"),
        Some(SourcePosition {
            utf16_offset: 11,
            line: 1,
            column: 12
        })
    );
    assert_eq!(
        tree.source_attribute_position(body, "style"),
        Some(SourcePosition {
            utf16_offset: 32,
            line: 1,
            column: 33
        })
    );
    assert_eq!(tree.get_attribute(body, "id"), Some("x"));
}
#[test]
fn source_checkpoints_preserve_columns_beyond_optimized_tokenizer_chunks() {
    let prefix = "😀\r\n".repeat(1024);
    let source = format!("{prefix}<div style='x'>");
    let tree = Tree::parse_with_source_positions(&source);
    let node = tree.query_selector("div").unwrap();
    assert_eq!(
        tree.source_position(node),
        Some(SourcePosition {
            utf16_offset: 4111,
            line: 1025,
            column: 16
        })
    );
}

#[test]
fn reconstructed_attribute_installation_has_context_without_a_fake_node_span() {
    let tree = Tree::parse_with_source_positions("<p><b style='x'>1</p><b style='x'>2");
    let nodes = tree.query_selector_all("b");
    assert_eq!(tree.source_position(nodes[1]), None);
    assert_eq!(
        tree.source_attribute_position(nodes[1], "style"),
        Some(SourcePosition {
            utf16_offset: 34,
            line: 1,
            column: 35
        })
    );
    assert_eq!(
        tree.source_attribute_position(nodes[2], "style"),
        Some(SourcePosition {
            utf16_offset: 34,
            line: 1,
            column: 35
        })
    );
}
#[test]
fn lone_transport_in_raw_script_and_reconstructed_text_never_repairs_units() {
    let mut units = "<p><b style='x'>1</p>".encode_utf16().collect::<Vec<_>>();
    units.push(0xd800);
    units.extend("<script>let a='".encode_utf16());
    units.push(0xdc00);
    units.extend("';</script>".encode_utf16());
    let tracked = Tree::parse_utf16_with_source_positions(&units);
    let original = Tree::parse_utf16(&units);
    assert_eq!(
        tracked.text_content_utf16(tracked.root()),
        original.text_content_utf16(original.root())
    );
    assert_eq!(
        tracked.get_elements_by_tag_name("b").len(),
        original.get_elements_by_tag_name("b").len()
    );
}
