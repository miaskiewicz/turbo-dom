//! The turbo-dom `Tree` style input's own contract (`TreeView`, `Roots::Document` /
//! `Roots::Detached`, the scope `Navigator`, markers). Kept from the consolidation S2
//! gate `tree_input_equivalence.rs` when its legacy-input comparison was retired at S5
//! (the owned-`Node` input is gone; the parity harness covers the corpora).

mod common;

use turbo_dom_layout::style::TokenSet;
use turbo_dom_layout::{
    parse_html_tree, style_input, Roots, StyleInput, StyleOptions, StyledNode, TreeView,
};

#[test]
fn document_styling_records_sources_and_skips_non_content() {
    let tree = parse_html_tree(
        "<head><title>t</title></head><body><!--c--><p id=a>x<b>y</b></p><style>p{}</style><template><i>t</i></template></body>",
    );
    let view = TreeView::new(&tree);
    let styled = style_input(
        &StyleInput {
            view: view.strip_non_visual(true),
            roots: Roots::Document,
        },
        &turbo_dom_layout::build_cascade("", "", TokenSet::default()),
        &StyleOptions::default(),
    );
    assert_eq!(styled.len(), 1, "the root element");
    let StyledNode::Element(html) = &styled[0] else {
        panic!("html element")
    };
    assert_eq!(html.source, view.find_element("html"));
    let [StyledNode::Element(head), StyledNode::Element(body)] = &html.children[..] else {
        panic!("head + body, got {:?}", html.children)
    };
    assert!(head.children.is_empty(), "title stripped");
    assert_eq!(body.source, view.find_element("body"));
    assert_eq!(
        body.children.len(),
        1,
        "comment, style and template are not content"
    );
    let StyledNode::Element(p) = &body.children[0] else {
        panic!("p element")
    };
    assert_eq!(view.attr(p.source.unwrap(), "id"), Some("a"));
    let StyledNode::Text(text) = &p.children[0] else {
        panic!("text")
    };
    assert_eq!(&**text, "x");
    assert_eq!(view.text(text.source.unwrap()), "x");
    // Without stripping the style + template elements are content (template empty).
    let body = view.find_element("body").unwrap();
    assert_eq!(view.children(body).len(), 3);
    assert_eq!(view.find_element("missing"), None);
}

#[test]
fn markers_classify_by_local_name_and_never_strip() {
    let tree = parse_html_tree("<body><x-marker-3-i>n</x-marker-3-i><script>s</script></body>");
    let view = TreeView::new(&tree)
        .markers(&common::marker_of)
        .strip_non_visual(true);
    let body = view.find_element("body").unwrap();
    let kids = view.children(body);
    assert_eq!(kids.len(), 1, "script stripped, marker kept");
    let kind = view.marker(kids[0]).expect("marker");
    assert_eq!((kind.id, kind.inline), (3, true));
    assert_eq!(view.tag(kids[0]), turbo_dom_layout::Tag::Marker(kind));
    assert_eq!(view.tag(body), turbo_dom_layout::Tag::Html("body".into()));
}

#[test]
fn navigator_scopes_detached_and_document_matching() {
    use turbo_dom_layout::style::{Navigator, SiblingPosition};
    let tree = parse_html_tree(
        "<html class=h><body class=b><!--c--><p>a<i>x</i></p>t<p></p><style>s</style></body></html>",
    );
    let view = TreeView::new(&tree);
    let body = view.find_element("body").unwrap();
    let html = view.find_element("html").unwrap();
    let roots = tree.children(body);

    let detached = Navigator::new(&view, Roots::Detached(&roots));
    let kept = detached.roots().to_vec();
    assert_eq!(kept.len(), 4, "p, text, p, style (comment skipped)");
    assert!(!detached.is_document_root(kept[0]));
    assert_eq!(
        detached.parent(kept[0]),
        None,
        "matching stops at the scope root"
    );
    assert_eq!(detached.siblings(kept[2]), kept);
    assert_eq!(
        detached.position(kept[2]),
        Some(SiblingPosition {
            index: 2,
            of_type_index: 2,
            of_type_total: 2,
            siblings: 3
        })
    );
    assert_eq!(detached.position(kept[1]), None, "text has no position");
    assert!(detached.is_empty(kept[2]) && !detached.is_empty(kept[0]));
    let i = detached.children(kept[0])[1];
    assert_eq!(detached.parent(i), Some(kept[0]));
    assert_eq!(detached.siblings(i).len(), 2);

    let stripped = view.strip_non_visual(true);
    let document = Navigator::new(&stripped, Roots::Document);
    assert_eq!(document.roots(), [html]);
    assert!(document.is_document_root(html) && !document.is_document_root(body));
    let head = view.find_element("head").unwrap();
    assert_eq!(
        document.children(html),
        [head, body],
        "head is kept, positioned"
    );
    assert_eq!(document.children(body).len(), 3, "style stripped");
    assert_eq!(document.parent(document.children(body)[0]), Some(body));
    assert_eq!(document.parent(body), Some(html));
    assert_eq!(document.parent(html), None);
    assert_eq!(document.siblings(html), [html]);
    assert!(!document.is_empty(html) && !document.is_empty(body) && document.is_empty(head));
    assert_eq!(
        document.position(html),
        Some(SiblingPosition {
            index: 1,
            of_type_index: 1,
            of_type_total: 1,
            siblings: 1
        }),
        "the root element is the document's only element child"
    );
    assert_eq!(
        document.position(body),
        Some(SiblingPosition {
            index: 2,
            of_type_index: 1,
            of_type_total: 1,
            siblings: 2
        }),
        "body follows head"
    );
}
