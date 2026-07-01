//! HTML serialization for innerHTML / outerHTML (WHATWG-ish fragment serializer).
//!
//! Pure-Rust port of `src/runtime/html-serialize.mjs`. Output is byte-identical to
//! the JS serializer: same VOID set, same `RAW_TEXT` set (unescaped content), and the
//! same exact escape character sets for text (`& <space> < >`) and attributes
//! (`& <space> "`).

use crate::rtdom::tree::{Handle, NodeType, Tree};

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

/// Append `s` text-escaped (`&`→`&amp;`, `<`→`&lt;`, `>`→`&gt;`) into `out` in a single
/// pass. Regular spaces are kept AS spaces — escaping them to `&nbsp;` (a quirk of the
/// old JS serializer) is wrong for HTML: it changes the text to non-breaking spaces, so
/// a re-parse / accessible-name / text query sees U+00A0 instead of a space and
/// whitespace-normalized matching (getByRole/getByText/toHaveText) fails. Fast path: no
/// special byte → push the slice whole (zero alloc, no scan-rewrite).
fn push_escaped_text(out: &mut String, s: &str) {
    // 0xC2 is the UTF-8 lead byte of U+00A0 (the non-breaking space we DO escape).
    if !s.bytes().any(|b| matches!(b, b'&' | b'<' | b'>' | 0xC2)) {
        out.push_str(s);
        return;
    }
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\u{A0}' => out.push_str("&nbsp;"),
            _ => out.push(c),
        }
    }
}

/// Append `s` attribute-escaped (`&`→`&amp;`, `"`→`&quot;`) into `out`. Spaces are kept
/// (a quoted attribute value preserves them verbatim — no need for `&nbsp;`).
fn push_escaped_attr(out: &mut String, s: &str) {
    if !s.bytes().any(|b| matches!(b, b'&' | b'"' | 0xC2)) {
        out.push_str(s);
        return;
    }
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '\u{A0}' => out.push_str("&nbsp;"),
            _ => out.push(c),
        }
    }
}

/// Serialize `h` and its subtree (the node itself + descendants). `parent_tag` is the
/// lowercase local name of the parent element, used only for the raw-text text-node rule.
fn serialize_node(tree: &Tree, h: Handle, parent_tag: Option<&str>, out: &mut String) {
    match tree.node_type(h) {
        NodeType::Element => {
            // local_name borrows &Tree; serialize is all-immutable, so no String alloc.
            let tag = tree.local_name(h).unwrap_or("");
            out.push('<');
            out.push_str(tag);
            // Foreign-content attributes carry a namespace prefix (xlink/xml/xmlns) that
            // is stored separately from the local name; the HTML serialization re-qualifies
            // them as `prefix:name` (e.g. `xlink:href`). Plain attributes have an empty
            // prefix and serialize as just the name.
            tree.for_each_attr_full(h, |prefix, name, value| {
                out.push(' ');
                if !prefix.is_empty() {
                    out.push_str(prefix);
                    out.push(':');
                }
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
        NodeType::Text => {
            let data = tree.node_value(h).unwrap_or_default();
            if parent_tag.is_some_and(is_raw_text) {
                out.push_str(&data);
            } else {
                push_escaped_text(out, &data);
            }
        }
        NodeType::Comment => {
            out.push_str("<!--");
            out.push_str(&tree.node_value(h).unwrap_or_default());
            out.push_str("-->");
        }
        NodeType::Doctype => {
            // Serialize `<!DOCTYPE ` + the node's name + `>` (the JS serializer emits
            // `<!DOCTYPE ${node.name}>`; public/system ids are not part of the HTML
            // fragment serialization). Falls back to "html" only if the name is somehow
            // unavailable (e.g. a runtime-created doctype, which has no constructor here).
            out.push_str("<!DOCTYPE ");
            out.push_str(tree.doctype_name(h).unwrap_or("html"));
            out.push('>');
        }
        NodeType::Fragment => {
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
    let parent_tag = tree.local_name(h).map(std::string::ToString::to_string);
    serialize_children(tree, h, parent_tag.as_deref().unwrap_or(""), &mut out);
    out
}

/// outerHTML: serialize `h` itself + its subtree.
pub fn serialize_outer(tree: &Tree, h: Handle) -> String {
    let mut out = String::new();
    let parent_tag = tree.parent(h).and_then(|p| tree.local_name(p).map(std::string::ToString::to_string));
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
        // & → &amp;, " → &quot;; spaces kept verbatim in a quoted attribute value.
        assert_eq!(html, "<span title=\"a &amp; &quot;b&quot;\"></span>");
    }

    #[test]
    fn doctype_fragment_document_branches() {
        let mut tree = Tree::parse("<!doctype html><html><body><p>x</p></body></html>");
        let root = tree.root();
        // DOCTYPE branch
        let dt = tree.children(root).into_iter().find(|&h| tree.node_type(h) == NodeType::Doctype).unwrap();
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
    fn doctype_serializes_actual_name() {
        // The HTML serialization of a DocumentType is `<!DOCTYPE ` + its name + `>`.
        // The common case is "html", but a non-html doctype name must be preserved
        // (the JS serializer emits `<!DOCTYPE ${node.name}>`).
        let tree = Tree::parse("<!DOCTYPE math><html></html>");
        let dt = tree
            .handles()
            .find(|&h| tree.node_type(h) == NodeType::Doctype)
            .unwrap();
        assert_eq!(serialize_outer(&tree, dt), "<!DOCTYPE math>");
    }

    #[test]
    fn foreign_attr_prefix_preserved() {
        // An SVG element's prefixed attribute (xlink:href) must serialize WITH its
        // prefix — browsers report `<use xlink:href="#i">`, not `<use href="#i">`.
        // The parser stores the prefix separately from the local name; the serializer
        // must re-qualify it.
        let tree = Tree::parse("<svg><use xlink:href=\"#i\"></use></svg>");
        let u = tree.get_elements_by_tag_name("use")[0];
        assert_eq!(serialize_outer(&tree, u), "<use xlink:href=\"#i\"></use>");
    }

    #[test]
    fn regular_spaces_kept_nbsp_char_escaped() {
        // Regular spaces serialize AS spaces; only the non-breaking space (U+00A0) → &nbsp;.
        let tree = Tree::parse("<div title=\"a b\">x y</div>");
        let div = first_div(&tree);
        assert_eq!(serialize_outer(&tree, div), "<div title=\"a b\">x y</div>");
        let tree2 = Tree::parse("<p>x\u{A0}y</p>");
        let p = tree2.get_elements_by_tag_name("p")[0];
        assert_eq!(serialize_outer(&tree2, p), "<p>x&nbsp;y</p>");
    }
}
