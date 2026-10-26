//! The LEGACY markup -> owned [`Node`] conversion (html5ever + `markup5ever_rcdom`).
//!
//! The layout pipeline now styles a turbo-dom [`Tree`](turbo_dom::Tree) directly
//! ([`crate::style::TreeView`]); this converter stays for the legacy [`Node`] input
//! ([`crate::html_layout::parse_html`]) and as the reference the Tree path is
//! checked against (equivalence tests). It lowers the document's `<body>` children
//! into [`Node`]s — elements and text only (comments, processing instructions and
//! doctypes are dropped; `<template>` content is not a child) — parsed with
//! html5ever's default options (scripting on, so `<noscript>` content is raw
//! text). An optional classifier turns elements into [`Tag::Marker`]s or rejects
//! them with an error.

use html5ever::tendril::TendrilSink;
use html5ever::{parse_document, ParseOpts};
use markup5ever_rcdom::{Handle, NodeData, RcDom};

use crate::error::RenderError;
use crate::node::{Attr, Element, Node, Tag};

/// Maps an element's local name to its [`Tag`] (a marker, a plain element, or an
/// error for a name the embedder rejects).
pub type TagClassifier<'a> = &'a dyn Fn(&str) -> Result<Tag, RenderError>;

/// Every element is a plain [`Tag::Html`] (no markers, no errors).
fn plain_tag(local: &str) -> Result<Tag, RenderError> {
    Ok(Tag::Html(local.to_string()))
}

/// Parse `markup` into the top-level flow nodes (the `<body>` children).
pub fn parse(markup: &str) -> Result<Vec<Node>, RenderError> {
    parse_with_roots(markup).map(|(nodes, _)| nodes)
}

/// Like [`parse`], but also returns the `<html>` and `<body>` ancestor shells
/// (their attributes only — no children). The cascade seeds these as match-only
/// ancestors so selectors gated on `<html>`/`<body>` classes (Wikipedia's
/// `vector-feature-*`, `client-js`, theme classes, …) match, even though those
/// elements are not themselves laid out as boxes.
pub fn parse_with_roots(markup: &str) -> Result<(Vec<Node>, Vec<Element>), RenderError> {
    parse_classified(markup, &plain_tag)
}

/// [`parse_with_roots`] with an element classifier: `classify(local_name)` decides
/// each element's [`Tag`] (pre-order; the first error wins and aborts the parse).
/// The `<html>`/`<body>` shells are always plain elements.
pub fn parse_classified(
    markup: &str,
    classify: TagClassifier,
) -> Result<(Vec<Node>, Vec<Element>), RenderError> {
    let dom = parse_document(RcDom::default(), ParseOpts::default())
        .from_utf8()
        .read_from(&mut markup.as_bytes())
        .expect("reading from an in-memory slice is infallible");
    let body =
        find_element(&dom.document, "body").expect("the HTML parser always produces a <body>");
    let mut out = Vec::new();
    convert_children(&body, classify, &mut out)?;
    let mut roots = Vec::new();
    if let Some(html) = find_element(&dom.document, "html") {
        roots.push(shell(&html));
    }
    roots.push(shell(&body));
    Ok((out, roots))
}

/// A childless [`Element`] carrying a DOM element's tag + attributes — used as a
/// match-only ancestor shell for `<html>`/`<body>`.
fn shell(handle: &Handle) -> Element {
    match &handle.data {
        NodeData::Element { name, attrs, .. } => Element {
            tag: Tag::Html(name.local.to_string()),
            attrs: collect_attrs(attrs),
            children: Vec::new(),
        },
        _ => unreachable!("find_element only returns element handles"),
    }
}

/// Depth-first search for the first element with the given local name.
fn find_element(handle: &Handle, name: &str) -> Option<Handle> {
    for child in handle.children.borrow().iter() {
        if let Some(found) = match_or_descend(child, name) {
            return Some(found);
        }
    }
    None
}

fn match_or_descend(child: &Handle, name: &str) -> Option<Handle> {
    if let NodeData::Element { name: qual, .. } = &child.data {
        if qual.local.as_ref() == name {
            return Some(child.clone());
        }
    }
    find_element(child, name)
}

fn convert_children(
    parent: &Handle,
    classify: TagClassifier,
    out: &mut Vec<Node>,
) -> Result<(), RenderError> {
    for child in parent.children.borrow().iter() {
        if let Some(node) = convert_node(child, classify)? {
            out.push(node);
        }
    }
    Ok(())
}

fn convert_node(handle: &Handle, classify: TagClassifier) -> Result<Option<Node>, RenderError> {
    match &handle.data {
        NodeData::Text { contents } => Ok(Some(Node::Text(contents.borrow().to_string()))),
        NodeData::Element { name, attrs, .. } => {
            let tag = classify(&name.local)?;
            let mut children = Vec::new();
            convert_children(handle, classify, &mut children)?;
            Ok(Some(Node::Element(Element {
                tag,
                attrs: collect_attrs(attrs),
                children,
            })))
        }
        _ => Ok(None),
    }
}

fn collect_attrs(attrs: &std::cell::RefCell<Vec<html5ever::Attribute>>) -> Vec<Attr> {
    attrs
        .borrow()
        .iter()
        .map(|a| Attr {
            name: a.name.local.to_string(),
            value: a.value.to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{ErrorCode, Span};
    use crate::node::MarkerKind;

    #[test]
    fn classifier_marks_rejects_and_shells_stay_plain() {
        let marker = MarkerKind {
            id: 7,
            inline: true,
        };
        let classify = |local: &str| match local {
            "x:mark" => Ok(Tag::Marker(marker)),
            "x:bad" => Err(RenderError {
                code: ErrorCode::UnknownElement,
                message: "bad".into(),
                span: Span::default(),
            }),
            other => Ok(Tag::Html(other.to_string())),
        };
        let (nodes, roots) =
            parse_classified("<p>a<x:mark></x:mark><!--c--></p>", &classify).expect("parse");
        let p = nodes[0].as_element().expect("p");
        assert_eq!(p.children.len(), 2, "comment dropped");
        assert_eq!(p.children[1].as_element().unwrap().tag, Tag::Marker(marker));
        assert_eq!(roots.len(), 2);
        assert!(roots.iter().all(|r| matches!(r.tag, Tag::Html(_))));
        let err = parse_classified("<div><x:bad></x:bad></div>", &classify).unwrap_err();
        assert_eq!(err.code, ErrorCode::UnknownElement);
        // The default parse never classifies.
        let plain = parse("<x:mark></x:mark>").expect("plain parse");
        assert_eq!(
            plain[0].as_element().unwrap().tag,
            Tag::Html("x:mark".into())
        );
    }
}
