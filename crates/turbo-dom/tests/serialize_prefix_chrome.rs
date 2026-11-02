//! createElementNS prefixes survive HTML serialization like Chrome 156 (headless
//! 156.0.8078.47): `<h:br>` stays void, `<h:script>` escapes its text, `<h:P>` keeps case.
use turbo_dom::rtdom::serialize::{serialize_inner, serialize_inner_utf16};
use turbo_dom::rtdom::Tree;

const EXPECTED: &str = "<h:br><h:template></h:template><h:script>&lt;</h:script><m:mi></m:mi><svg:g></svg:g><h:P></h:P>";

#[test]
fn prefixed_elements_serialize_their_qualified_name_like_chrome() {
    const H: &str = "http://www.w3.org/1999/xhtml";
    let mut tree = Tree::parse("<div></div>");
    let s = tree.query_selector("div").unwrap();
    let br = tree.create_element_namespace(Some(H), "h:br");
    let tp = tree.create_element_namespace(Some(H), "h:template");
    let sc = tree.create_element_namespace(Some(H), "h:script");
    let lt = tree.create_text_node("<");
    tree.append_child(sc, lt);
    let mi = tree.create_element_namespace(Some("http://www.w3.org/1998/Math/MathML"), "m:mi");
    let g = tree.create_element_namespace(Some("http://www.w3.org/2000/svg"), "svg:g");
    let p = tree.create_element_namespace(Some(H), "h:P");
    for n in [br, tp, sc, mi, g, p] {
        tree.append_child(s, n);
    }
    assert_eq!(serialize_inner(&tree, s), EXPECTED);
    assert_eq!(
        String::from_utf16(&serialize_inner_utf16(&tree, s)).unwrap(),
        EXPECTED
    );
}
