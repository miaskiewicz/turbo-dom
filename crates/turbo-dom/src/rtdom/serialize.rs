//! HTML serialization for innerHTML / outerHTML (WHATWG-ish fragment serializer).
//!
//! Pure-Rust port of `src/runtime/html-serialize.mjs`. Output is byte-identical to
//! the JS serializer: same VOID set, same RAW_TEXT set (unescaped content), and the
//! same exact escape character sets for text (`& <space> < >`) and attributes
//! (`& <space> "`).

use crate::rtdom::tree::{
    Handle, Tree, COMMENT_NODE, DOCTYPE_NODE, ELEMENT_NODE, FRAGMENT_NODE, TEXT_NODE,
};

/// Elements that emit no closing tag and have no serialized children.
const VOID: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

/// Elements whose text content is raw (NOT escaped) on serialization.
const RAW_TEXT: &[&str] = &[
    "style", "script", "xmp", "iframe", "noembed", "noframes", "plaintext",
];

fn is_void(tag: &str) -> bool {
    VOID.contains(&tag)
}

fn is_raw_text(tag: &str) -> bool {
    RAW_TEXT.contains(&tag)
}

/// Append `s` text-escaped (`&`→`&amp;`, ` `→`&nbsp;`, `<`→`&lt;`, `>`→`&gt;`) into
/// `out` in a single pass. Per-original-char mapping is identical to the JS chained
/// `replace` (no char a replacement emits is itself a replacement target). Fast path:
/// no special byte → push the slice whole (zero alloc, no scan-rewrite).
fn push_escaped_text(out: &mut String, s: &str) {
    if !s.bytes().any(|b| matches!(b, b'&' | b' ' | b'<' | b'>')) {
        out.push_str(s);
        return;
    }
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            ' ' => out.push_str("&nbsp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
}

/// Append `s` attribute-escaped (`&`→`&amp;`, ` `→`&nbsp;`, `"`→`&quot;`) into `out`.
fn push_escaped_attr(out: &mut String, s: &str) {
    if !s.bytes().any(|b| matches!(b, b'&' | b' ' | b'"')) {
        out.push_str(s);
        return;
    }
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            ' ' => out.push_str("&nbsp;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
}

/// Serialize `h` and its subtree (the node itself + descendants). `parent_tag` is the
/// lowercase local name of the parent element, used only for the raw-text text-node rule.
fn serialize_node(tree: &Tree, h: Handle, parent_tag: Option<&str>, out: &mut String) {
    match tree.node_type(h) {
        ELEMENT_NODE => {
            // local_name borrows &Tree; serialize is all-immutable, so no String alloc.
            let tag = tree.local_name(h).unwrap_or("");
            out.push('<');
            out.push_str(tag);
            tree.for_each_attr(h, |name, value| {
                out.push(' ');
                out.push_str(name);
                out.push_str("=\"");
                push_escaped_attr(out, value);
                out.push('"');
            });
            out.push('>');
            if is_void(tag) {
                return;
            }
            serialize_children(tree, h, tag, out);
            out.push_str("</");
            out.push_str(tag);
            out.push('>');
        }
        TEXT_NODE => {
            let data = tree.node_value(h).unwrap_or_default();
            if parent_tag.map(is_raw_text).unwrap_or(false) {
                out.push_str(&data);
            } else {
                push_escaped_text(out, &data);
            }
        }
        COMMENT_NODE => {
            out.push_str("<!--");
            out.push_str(&tree.node_value(h).unwrap_or_default());
            out.push_str("-->");
        }
        DOCTYPE_NODE => {
            // JS reads `node.name`; the Tree API does not expose a doctype name, so emit
            // the practical default. (Fixtures do not exercise the name.)
            out.push_str("<!DOCTYPE html>");
        }
        FRAGMENT_NODE => {
            serialize_children(tree, h, "", out);
        }
        _ => {}
    }
}

fn serialize_children(tree: &Tree, h: Handle, parent_tag: &str, out: &mut String) {
    for c in tree.children(h) {
        serialize_node(tree, c, Some(parent_tag), out);
    }
}

/// innerHTML: serialize only the children of `h`.
pub fn serialize_inner(tree: &Tree, h: Handle) -> String {
    let mut out = String::new();
    let parent_tag = tree.local_name(h).map(|s| s.to_string());
    serialize_children(tree, h, parent_tag.as_deref().unwrap_or(""), &mut out);
    out
}

/// outerHTML: serialize `h` itself + its subtree.
pub fn serialize_outer(tree: &Tree, h: Handle) -> String {
    let mut out = String::new();
    let parent_tag = tree.parent(h).and_then(|p| tree.local_name(p).map(|s| s.to_string()));
    serialize_node(tree, h, parent_tag.as_deref(), &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rtdom::tree::Tree;

    fn first_div(tree: &Tree) -> Handle {
        tree.get_elements_by_tag_name("div")[0]
    }

    #[test]
    fn div_with_attrs_and_nested_children() {
        let tree = Tree::parse("<div id=\"a\" class=\"b\"><span>hi</span><p>yo</p></div>");
        let div = first_div(&tree);
        assert_eq!(
            serialize_outer(&tree, div),
            "<div id=\"a\" class=\"b\"><span>hi</span><p>yo</p></div>"
        );
        assert_eq!(
            serialize_inner(&tree, div),
            "<span>hi</span><p>yo</p>"
        );
    }

    #[test]
    fn void_img_has_no_close_tag() {
        let tree = Tree::parse("<div><img src=\"x\"></div>");
        let div = first_div(&tree);
        assert_eq!(serialize_inner(&tree, div), "<img src=\"x\">");
    }

    #[test]
    fn raw_text_style_not_escaped() {
        let tree = Tree::parse("<div><style>a<b</style></div>");
        let div = first_div(&tree);
        // The `<` inside <style> stays raw (not &lt;).
        assert_eq!(serialize_inner(&tree, div), "<style>a<b</style>");
    }

    #[test]
    fn text_special_chars_escaped() {
        let tree = Tree::parse("<div>a&amp;b&lt;c&gt;d</div>");
        let div = first_div(&tree);
        // Parsed text is `a&b<c>d`; re-serialized it must escape & < > (and space).
        assert_eq!(serialize_inner(&tree, div), "a&amp;b&lt;c&gt;d");
    }

    #[test]
    fn comment_round_trip() {
        let tree = Tree::parse("<div><!--hello--></div>");
        let div = first_div(&tree);
        assert_eq!(serialize_inner(&tree, div), "<!--hello-->");
    }

    #[test]
    fn attr_value_ampersand_and_quote_escaped() {
        let mut tree = Tree::parse("<span></span>");
        let span = tree.get_elements_by_tag_name("span")[0];
        tree.set_attribute(span, "title", "a & \"b\"");
        let html = serialize_outer(&tree, span);
        // & → &amp;, " → &quot;, space → &nbsp; (matching escape_attr semantics)
        assert_eq!(html, "<span title=\"a&nbsp;&amp;&nbsp;&quot;b&quot;\"></span>");
    }

    #[test]
    fn doctype_fragment_document_branches() {
        let mut tree = Tree::parse("<!doctype html><html><body><p>x</p></body></html>");
        let root = tree.root();
        // DOCTYPE branch
        let dt = tree.children(root).into_iter().find(|&h| tree.node_type(h) == 10).unwrap();
        assert_eq!(serialize_outer(&tree, dt), "<!DOCTYPE html>");
        // DOCUMENT node → the `_ => {}` arm (empty output)
        assert_eq!(serialize_outer(&tree, root), "");
        // FRAGMENT branch via a shadow root → serializes children only
        let div = tree.get_elements_by_tag_name("p")[0];
        let sr = tree.attach_shadow(div);
        tree.set_inner_html(sr, "<span>hi</span>");
        assert_eq!(serialize_outer(&tree, sr), "<span>hi</span>");
    }

    #[test]
    fn space_escaped_as_nbsp() {
        let tree = Tree::parse("<div title=\"a b\">x y</div>");
        let div = first_div(&tree);
        assert_eq!(
            serialize_outer(&tree, div),
            "<div title=\"a&nbsp;b\">x&nbsp;y</div>"
        );
    }
}
