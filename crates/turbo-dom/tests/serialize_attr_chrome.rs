//! Attribute values escape `<` and `>` when serialized, as in Chrome 156 (headless
//! 156.0.8078.47: `setAttribute('t', 'a<b>&"')` -> `t="a&lt;b&gt;&amp;&quot;"`).
use turbo_dom::rtdom::serialize::{serialize_outer, serialize_outer_utf16};
use turbo_dom::rtdom::Tree;

#[test]
fn attribute_values_escape_angle_brackets_like_chrome() {
    let mut tree = Tree::parse("<i></i>");
    let i = tree.query_selector("i").unwrap();
    tree.set_attribute(i, "t", "a<b>&\"\u{a0}'");
    let want = "<i t=\"a&lt;b&gt;&amp;&quot;&nbsp;'\"></i>";
    assert_eq!(serialize_outer(&tree, i), want);
    assert_eq!(
        String::from_utf16(&serialize_outer_utf16(&tree, i)).unwrap(),
        want
    );
}
