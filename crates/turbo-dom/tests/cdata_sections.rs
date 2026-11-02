//! CDATA sections (turbo-dom#186): the node type XML parsing and `createCDATASection`
//! produce. The expected values are Chrome 156's, from the `serializer` rows of
//! crates/turbo-dom-browser-env/src/xml_documents_chrome156.json (`children`,
//! `cdataInHtml`, `cdataEdits`), which the binding's XML documents test replays end to end.
use turbo_dom::rtdom::serialize::{serialize_inner, serialize_inner_utf16, serialize_outer};
use turbo_dom::rtdom::tree::{CollectionBudget, NodeType, Tree};

fn units(value: &str) -> Vec<u16> {
    value.encode_utf16().collect()
}

#[test]
fn cdata_sections_are_character_data_with_type_4() {
    let mut tree = Tree::parse("<div></div>");
    let cdata = tree.create_cdata_section_utf16(&units("c]>d"));
    // children: [4, "#cdata-section", "c]>d"]
    assert_eq!(tree.node_type(cdata), NodeType::CDataSection);
    assert_eq!(tree.node_type_id(cdata), 4);
    assert_eq!(tree.node_value(cdata).as_deref(), Some("c]>d"));
    assert_eq!(tree.node_value_utf16(cdata), Some(units("c]>d")));
    assert_eq!(tree.character_data(cdata).as_deref(), Some("c]>d"));
    assert_eq!(tree.text_content(cdata), "c]>d");
    assert_eq!(tree.text_content_utf16(cdata), units("c]>d"));
}

#[test]
fn cdata_sections_edit_like_text() {
    // cdataEdits: data = "x<y" reads back through textContent; textContent = "z" then
    // appendData("!") gives data / nodeValue "z!".
    let mut tree = Tree::parse("<div></div>");
    let cdata = tree.create_cdata_section_utf16(&units("abc"));
    tree.set_node_value(cdata, "x<y");
    assert_eq!(tree.text_content(cdata), "x<y");
    tree.set_text_content(cdata, "z");
    assert_eq!(tree.node_value(cdata).as_deref(), Some("z"));
    tree.set_text_content_utf16(cdata, &units("z!"));
    assert_eq!(tree.node_value_utf16(cdata), Some(units("z!")));
    assert_eq!(tree.character_data(cdata).as_deref(), Some("z!"));
}

#[test]
fn cdata_sections_count_as_text_for_text_content_empty_and_html_serialization() {
    // cdataInHtml: <div>"t" + CDATA "a<b&"</div> -> innerHTML "ta&lt;b&amp;", textContent
    // "ta<b&"; cdataEdits: an element whose only child is an empty CDATA section is
    // :empty, and stops being so once the section has data.
    let mut tree = Tree::parse("<div id=d>t</div><p id=e></p>");
    let div = tree.query_selector("#d").unwrap();
    let cdata = tree.create_cdata_section_utf16(&units("a<b&"));
    tree.append_child(div, cdata);
    assert_eq!(serialize_inner(&tree, div), "ta&lt;b&amp;");
    assert_eq!(serialize_inner_utf16(&tree, div), units("ta&lt;b&amp;"));
    assert_eq!(tree.text_content(div), "ta<b&");
    assert_eq!(tree.text_content_utf16(div), units("ta<b&"));
    let e = tree.query_selector("#e").unwrap();
    let empty = tree.create_cdata_section_utf16(&[]);
    tree.append_child(e, empty);
    assert!(tree.matches(e, ":empty"));
    tree.set_node_value(empty, "q");
    assert!(!tree.matches(e, ":empty"));
    assert_eq!(tree.text_content(e), "q");
}

#[test]
fn cdata_sections_survive_storage_collection() {
    let mut tree = Tree::parse("<div id=d></div>");
    let div = tree.query_selector("#d").unwrap();
    let cdata = tree.create_cdata_section_utf16(&units("kept"));
    tree.append_child(div, cdata);
    tree.set_node_value(cdata, "kept!");
    let before = tree.storage_stats();
    let budget = CollectionBudget {
        max_nodes: 1000,
        max_edges: 1000,
        max_payload_bytes: 1_000_000,
    };
    tree.collect_unreachable(&[], budget).unwrap();
    // The override folds into the section: the same payload as a section created "kept!".
    let mut fresh = Tree::parse("<div id=d></div>");
    let fresh_div = fresh.query_selector("#d").unwrap();
    let fresh_cdata = fresh.create_cdata_section_utf16(&units("kept!"));
    fresh.append_child(fresh_div, fresh_cdata);
    fresh.collect_unreachable(&[], budget).unwrap();
    assert!(before.payload_bytes > tree.storage_stats().payload_bytes);
    assert_eq!(
        tree.storage_stats().payload_bytes,
        fresh.storage_stats().payload_bytes
    );
    assert_eq!(tree.node_value(cdata).as_deref(), Some("kept!"));
    assert_eq!(serialize_outer(&tree, div), "<div id=\"d\">kept!</div>");
}
