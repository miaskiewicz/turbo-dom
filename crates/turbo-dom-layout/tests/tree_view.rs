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

#[test]
fn stripped_elements_still_count_for_selectors() {
    // miaskiewicz/turbo-dom#25 item 2 (Chrome 156: tests/fixtures/
    // html_document_chrome156.json `*-empty` / `*-sibling*` / `has-style-child` cases):
    // a stripped non-visual element has no styled node, but it is a child, a sibling
    // and a `:has()` descendant like any other.
    use turbo_dom_layout::style::{Navigator, SiblingPosition};
    let tree = parse_html_tree(
        "<body><div id=s><style></style></div><div id=e><!--c--></div><script></script><p id=p></p><template></template></body>",
    );
    let view = TreeView::new(&tree).strip_non_visual(true);
    let nav = Navigator::new(&view, Roots::Document);
    let by_id = |id| tree.get_element_by_id(id).unwrap();
    let body = view.find_element("body").unwrap();
    assert!(!nav.is_empty(by_id("s")), "a stripped <style> child");
    assert!(nav.is_empty(by_id("e")), "a comment is not a child");
    assert!(nav.children(by_id("s")).is_empty(), "nothing to lay out");
    assert_eq!(nav.children(body).len(), 3, "div, div, p are laid out");
    assert_eq!(
        nav.siblings(by_id("p")).len(),
        5,
        "script and template count"
    );
    assert_eq!(
        nav.position(by_id("p")),
        Some(SiblingPosition {
            index: 4,
            of_type_index: 1,
            of_type_total: 1,
            siblings: 5
        })
    );
    let styled = style_input(
        &StyleInput {
            view,
            roots: Roots::Document,
        },
        &turbo_dom_layout::build_cascade(
            "#s:empty{color:red} div:has(style){width:1px} script + p{height:2px} p:last-child{color:blue}",
            "",
            TokenSet::default(),
        ),
        &StyleOptions::default(),
    );
    let StyledNode::Element(html) = &styled[0] else {
        panic!("html")
    };
    let StyledNode::Element(body) = &html.children[1] else {
        panic!("body")
    };
    let get = |i: usize, property: &str| match &body.children[i] {
        StyledNode::Element(element) => element.style.get(property).map(str::to_string),
        StyledNode::Text(_) => None,
    };
    assert_eq!(get(0, "color"), None, "#s is not :empty");
    assert_eq!(get(0, "width").as_deref(), Some("1px"));
    assert_eq!(get(2, "height").as_deref(), Some("2px"));
    assert_eq!(get(2, "color"), None, "a template follows the p");
}

#[test]
fn selector_navigator_siblings_and_types_with_markers() {
    // The turbo-dom `Navigator` contract over the scope navigator: element siblings step
    // over text, a non-element has no element neighbours, and a marker is its own
    // element type (a marker of another kind is a different type again).
    use turbo_dom::rtdom::query::Navigator as _;
    use turbo_dom_layout::style::Navigator;
    let tree = parse_html_tree(
        "<body><x-marker-3-i>m</x-marker-3-i><p>a</p>t<p></p><x-marker-3-i></x-marker-3-i><x-marker-4-b></x-marker-4-b></body>",
    );
    let view = TreeView::new(&tree).markers(&common::marker_of);
    let nav = Navigator::new(&view, Roots::Document);
    let body = view.find_element("body").unwrap();
    let kids = tree.children(body);
    let [m1, p1, text, p2, m2, m4] = kids[..] else {
        panic!("six children, got {kids:?}")
    };
    assert_eq!(nav.next_element_sibling(m1), Some(p1));
    assert_eq!(nav.next_element_sibling(p1), Some(p2), "text skipped");
    assert_eq!(nav.next_element_sibling(m4), None, "last element");
    assert_eq!(nav.previous_element_sibling(p2), Some(p1));
    assert_eq!(nav.previous_element_sibling(m1), None, "first element");
    assert_eq!(
        nav.next_element_sibling(text),
        None,
        "a text node has no index"
    );
    assert_eq!(nav.previous_element_sibling(text), None);
    assert!(nav.same_type(p1, p2) && nav.same_type(m1, m2));
    assert!(!nav.same_type(m1, p1) && !nav.same_type(m1, m4));
    assert_eq!(nav.element_siblings(p1), [m1, p1, p2, m2, m4]);
}

#[test]
fn marker_siblings_are_their_own_type_for_of_type_selectors() {
    let tree = parse_html_tree(
        "<body><x-marker-3-i>m</x-marker-3-i><p id=a>a</p><p id=b>b</p><x-marker-3-i></x-marker-3-i></body>",
    );
    let view = TreeView::new(&tree).markers(&common::marker_of);
    let styled = style_input(
        &StyleInput {
            view,
            roots: Roots::Document,
        },
        &turbo_dom_layout::build_cascade(
            "p:first-of-type{color:red} p:last-of-type{color:blue} p:first-child{width:1px} p:has(+ p){height:2px}",
            "",
            TokenSet::default(),
        ),
        &StyleOptions::default(),
    );
    let StyledNode::Element(html) = &styled[0] else {
        panic!("html")
    };
    let StyledNode::Element(body) = &html.children[1] else {
        panic!("body")
    };
    let get = |i: usize, property: &str| match &body.children[i] {
        StyledNode::Element(element) => element.style.get(property).map(str::to_string),
        StyledNode::Text(_) => None,
    };
    assert_eq!(get(1, "color").as_deref(), Some("red"));
    assert_eq!(get(2, "color").as_deref(), Some("blue"));
    assert_eq!(get(1, "width"), None, "the marker is the first child");
    assert_eq!(get(1, "height").as_deref(), Some("2px"), "followed by a p");
    assert_eq!(get(2, "height"), None, "followed by a marker");
}

#[test]
fn without_sources_clears_provenance_recursively() {
    let html = "<p id=a>x<b>y</b></p>";
    let style = |tree: &turbo_dom::Tree| {
        let view = TreeView::new(tree);
        let body = view.find_element("body").unwrap();
        let roots = tree.children(body);
        style_input(
            &StyleInput {
                view,
                roots: Roots::Detached(&roots),
            },
            &turbo_dom_layout::build_cascade("b{color:red}", "", TokenSet::default()),
            &StyleOptions::default(),
        )
    };
    let (one, two) = (
        parse_html_tree(html),
        parse_html_tree(&format!("<i></i>{html}")),
    );
    let a = style(&one);
    let b = style(&two);
    let StyledNode::Element(p) = &a[0] else {
        panic!("p")
    };
    assert!(p.source.is_some());
    let stripped = a[0].without_sources();
    assert_ne!(a[0], b[1], "different trees, different sources");
    assert_eq!(stripped, b[1].without_sources());
    let StyledNode::Element(sp) = &stripped else {
        panic!("p")
    };
    assert_eq!(sp.source, None);
    assert_eq!(
        (sp.tag.clone(), sp.style.clone(), sp.attrs.clone()),
        (p.tag.clone(), p.style.clone(), p.attrs.clone())
    );
    let StyledNode::Text(text) = &sp.children[0] else {
        panic!("text")
    };
    assert_eq!((&**text, text.source), ("x", None));
    let StyledNode::Element(bold) = &sp.children[1] else {
        panic!("b")
    };
    assert_eq!((bold.source, bold.style.get("color")), (None, Some("red")));
    let StyledNode::Text(y) = &bold.children[0] else {
        panic!("y")
    };
    assert_eq!((&**y, y.source), ("y", None));
}
