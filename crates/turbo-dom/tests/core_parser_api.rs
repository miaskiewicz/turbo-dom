//! The public parser core (`turbo_dom::core`): the `SoA` buffer entry points, the raw node
//! count and the html5lib-style fragment contexts, asserted value by value.
use turbo_dom::core::{
    parse_html_document_count, parse_html_fragment_context, parse_html_fragment_in,
    parse_html_soa, parse_html_soa_with_compat_mode, Node, COMMENT_NODE, DOCUMENT_NODE,
    DOCUMENT_TYPE_NODE, ELEMENT_NODE, TEXT_NODE,
};

/// `(nodeType, name, namespace, value)` of each node, depth first.
fn flatten(node: &Node, out: &mut Vec<(u8, String, String, String)>) {
    out.push((
        node.node_type,
        node.name.clone(),
        node.namespace.clone(),
        node.value.clone(),
    ));
    for child in &node.children {
        flatten(child, out);
    }
}

fn shape(node: &Node) -> Vec<(u8, String, String, String)> {
    let mut out = Vec::new();
    for child in &node.children {
        flatten(child, &mut out);
    }
    out
}

fn row(t: u8, name: &str, ns: &str, value: &str) -> (u8, String, String, String) {
    (t, name.into(), ns.into(), value.into())
}

#[test]
fn soa_columns_link_every_parsed_node() {
    let html = "<!DOCTYPE html><p id=a>x</p><!--c-->";
    let soa = parse_html_soa(html);
    // document, doctype, html, head, body, p, "x", comment
    assert_eq!(
        soa.node_type,
        [
            DOCUMENT_NODE,
            DOCUMENT_TYPE_NODE,
            ELEMENT_NODE,
            ELEMENT_NODE,
            ELEMENT_NODE,
            ELEMENT_NODE,
            TEXT_NODE,
            COMMENT_NODE
        ]
    );
    assert_eq!(soa.parent, [-1, 0, 0, 2, 2, 4, 5, 4]);
    assert_eq!(soa.first_child, [1, -1, 3, -1, 5, 6, -1, -1]);
    assert_eq!(soa.next_sib, [-1, 2, -1, 4, -1, 7, -1, -1]);
    let tag = |i: usize| soa.tag_names[soa.tag_id[i] as usize].as_str();
    assert_eq!([tag(2), tag(3), tag(4), tag(5)], ["html", "head", "body", "p"]);
    assert_eq!(soa.strings[soa.text_id[1] as usize], "html");
    assert_eq!(soa.strings[soa.text_id[6] as usize], "x");
    assert_eq!(soa.strings[soa.text_id[7] as usize], "c");
    assert_eq!((soa.attr_start[5], soa.attr_count[5]), (0, 1));
    assert_eq!(soa.attr_names[soa.attr_name_id[0] as usize], "id");
    assert_eq!(soa.attr_values[soa.attr_value_id[0] as usize], "a");
    // the plain entry is the compat-mode entry without the mode
    let (with_mode, mode) = parse_html_soa_with_compat_mode(html);
    assert_eq!(with_mode.node_type, soa.node_type);
    assert_eq!(with_mode.parent, soa.parent);
    assert_eq!(mode, "CSS1Compat");
    assert_eq!(parse_html_soa_with_compat_mode("<p>").1, "BackCompat");
}

#[test]
fn document_count_counts_every_node_once() {
    // document, html, head, body
    assert_eq!(parse_html_document_count(""), 4);
    // + doctype, p, text
    assert_eq!(parse_html_document_count("<!DOCTYPE html><p>x</p>"), 7);
    // implied </p> + two text nodes + a comment
    assert_eq!(parse_html_document_count("<p>a<p>b<!--c-->"), 9);
    // template content is not a child, so it is not counted
    assert_eq!(parse_html_document_count("<template><b>x</b></template>"), 5);
}

#[test]
fn fragment_contexts_select_namespace_and_local_name() {
    // "" is <body>: an HTML element; `td` keeps table-cell content
    assert_eq!(
        shape(&parse_html_fragment_context("<b>x</b>", "")),
        [row(ELEMENT_NODE, "b", "", ""), row(TEXT_NODE, "#text", "", "x")]
    );
    assert_eq!(
        shape(&parse_html_fragment_context("<td>x</td>", "tr")),
        [row(ELEMENT_NODE, "td", "", ""), row(TEXT_NODE, "#text", "", "x")]
    );
    // a foreign context: children are foreign unless an integration point
    assert_eq!(
        shape(&parse_html_fragment_context("<circle/>", "svg svg")),
        [row(ELEMENT_NODE, "circle", "svg", "")]
    );
    assert_eq!(
        shape(&parse_html_fragment_context("<mi/>", "math math")),
        [row(ELEMENT_NODE, "mi", "math", "")]
    );
    // a text integration point (MathML mi) parses HTML
    assert_eq!(
        shape(&parse_html_fragment_context("<b/>", "math mi")),
        [row(ELEMENT_NODE, "b", "", "")]
    );
    // any other prefix is an HTML context named by the local part
    assert_eq!(
        shape(&parse_html_fragment_context("<td>x</td>", "html tr")),
        shape(&parse_html_fragment_context("<td>x</td>", "tr"))
    );
    assert_eq!(
        shape(&parse_html_fragment_context("<circle/>", "x svg")),
        [row(ELEMENT_NODE, "circle", "", "")]
    );
    // the explicit form, every namespace
    assert_eq!(
        shape(&parse_html_fragment_in("<path/>", "svg", "g")),
        [row(ELEMENT_NODE, "path", "svg", "")]
    );
    assert_eq!(
        shape(&parse_html_fragment_in("<mn/>", "math", "math")),
        [row(ELEMENT_NODE, "mn", "math", "")]
    );
    assert_eq!(
        shape(&parse_html_fragment_in("<path/>", "", "div")),
        [row(ELEMENT_NODE, "path", "", "")]
    );
    let root = parse_html_fragment_in("a", "", "div");
    assert_eq!((root.node_type, root.name.as_str()), (DOCUMENT_NODE, "#document-fragment"));
}
