//! One immutable selector view and escaped source edits over validated XML.
use super::super::{usvg::roxmltree, SvgResourceError};
use std::collections::HashMap;
use std::ops::Range;
use turbo_dom::{Handle, Tree};

type NodeId = roxmltree::NodeId;

pub(super) struct SelectorTree {
    pub tree: Tree,
    pub handles: HashMap<NodeId, Handle>,
}

impl SelectorTree {
    pub fn new(document: &roxmltree::Document<'_>) -> Self {
        let mut tree = Tree::parse("<!doctype html>");
        let root = tree.root();
        for child in tree.children(root) {
            tree.remove_child(root, child);
        }
        let mut handles = HashMap::from([(document.root().id(), root)]);
        for node in document.descendants().skip(1) {
            let handle = if node.is_element() {
                let name = node.tag_name();
                let handle = tree.create_element_namespace(name.namespace(), name.name());
                copy_attributes(&mut tree, handle, node);
                handle
            } else if node.is_text() {
                tree.create_text_node(node.text().unwrap_or_default())
            } else {
                continue;
            };
            let parent = node
                .parent()
                .and_then(|parent| handles.get(&parent.id()))
                .copied()
                .unwrap_or(root);
            tree.append_child(parent, handle);
            handles.insert(node.id(), handle);
        }
        Self { tree, handles }
    }
}

fn copy_attributes(tree: &mut Tree, handle: Handle, node: roxmltree::Node<'_, '_>) {
    for attr in node.attributes() {
        tree.set_attribute_ns(handle, attr.namespace(), attr.name(), attr.value());
    }
}

pub(super) struct Edit {
    range: Range<usize>,
    value: String,
}

pub(super) fn attribute_edit(
    node: roxmltree::Node<'_, '_>,
    name: &str,
    value: &str,
    source: &str,
    limit: usize,
) -> Result<Edit, SvgResourceError> {
    let escaped = escape(value, limit)?;
    if let Some(attr) = node
        .attributes()
        .find(|attr| attr.namespace().is_none() && attr.name() == name)
    {
        return Ok(Edit {
            range: attr.range_value(),
            value: escaped,
        });
    }
    let start = node.range().start + 1;
    let name_len = source[start..]
        .find(|c: char| c.is_ascii_whitespace() || c == '/' || c == '>')
        .ok_or(SvgResourceError::UnsupportedCustomProperties)?;
    let at = start + name_len;
    Ok(Edit {
        range: at..at,
        value: format!(" {name}=\"{escaped}\""),
    })
}

pub(super) fn empty_styles(document: &roxmltree::Document<'_>) -> Vec<Edit> {
    let source = document.input_text();
    document
        .descendants()
        .filter(|node| {
            node.has_tag_name((super::super::SVG_NS, "style")) && node.children().next().is_some()
        })
        .map(|node| {
            let range = node.range();
            let start = range.start + super::super::tag_end(&source.as_bytes()[range.start..]) + 1;
            let end = range.start
                + source[range.clone()]
                    .rfind("</")
                    .expect("validated style closing tag");
            Edit {
                range: start..end,
                value: String::new(),
            }
        })
        .collect()
}

fn replacement(c: char) -> Option<&'static str> {
    match c {
        '&' => Some("&amp;"),
        '<' => Some("&lt;"),
        '>' => Some("&gt;"),
        '"' => Some("&quot;"),
        '\'' => Some("&apos;"),
        _ => None,
    }
}

fn escape(style: &str, limit: usize) -> Result<String, SvgResourceError> {
    let units = style
        .chars()
        .try_fold(0usize, |total, c| {
            total.checked_add(replacement(c).map_or(c.len_utf16(), str::len))
        })
        .ok_or(SvgResourceError::CustomPropertyBytes)?;
    if units > limit {
        return Err(SvgResourceError::CustomPropertyBytes);
    }
    let mut escaped = String::new();
    for c in style.chars() {
        match replacement(c) {
            Some(value) => escaped.push_str(value),
            None => escaped.push(c),
        }
    }
    Ok(escaped)
}

pub(super) struct Edits<'a> {
    source: &'a str,
    limit: usize,
    growth: usize,
    values: Vec<Edit>,
}

impl<'a> Edits<'a> {
    pub fn new(source: &'a str, limit: usize) -> Self {
        Self {
            source,
            limit,
            growth: 0,
            values: Vec::new(),
        }
    }
    pub fn push(&mut self, edit: Edit) -> Result<(), SvgResourceError> {
        charge(self.source, &edit, &mut self.growth, self.limit)?;
        self.values.push(edit);
        Ok(())
    }
    pub fn finish(self) -> Vec<Edit> {
        self.values
    }
}

pub(super) fn apply(
    source: &str,
    mut edits: Vec<Edit>,
    limit: usize,
) -> Result<Option<String>, SvgResourceError> {
    if edits.is_empty() {
        return Ok(None);
    }
    edits.sort_by_key(|edit| edit.range.start);
    validate(source, &edits, limit)?;
    let mut output = source.to_owned();
    for edit in edits.into_iter().rev() {
        output.replace_range(edit.range, &edit.value);
    }
    Ok(Some(output))
}

fn validate(source: &str, edits: &[Edit], limit: usize) -> Result<(), SvgResourceError> {
    let mut growth = 0usize;
    for edit in edits {
        charge(source, edit, &mut growth, limit)?;
    }
    Ok(())
}

fn charge(
    source: &str,
    edit: &Edit,
    growth: &mut usize,
    limit: usize,
) -> Result<(), SvgResourceError> {
    // Validated XML ranges are disjoint; DTD rewriting is refused upstream.
    let old = source[edit.range.clone()].encode_utf16().count();
    let extra = edit.value.encode_utf16().count().saturating_sub(old);
    *growth = growth
        .checked_add(extra)
        .filter(|&units| units <= limit)
        .ok_or(SvgResourceError::CustomPropertyBytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escaped_attribute_budget_is_checked_before_materialization() {
        let source = "<svg xmlns='http://www.w3.org/2000/svg'/>";
        let document = roxmltree::Document::parse(source).unwrap();
        let root = document.root_element();
        assert!(matches!(
            attribute_edit(root, "style", "<&>", source, 12),
            Err(SvgResourceError::CustomPropertyBytes)
        ));
        let edit = attribute_edit(root, "style", "<&>", source, 13).unwrap();
        let output = apply(source, vec![edit], 32).unwrap().unwrap();
        assert_eq!(
            roxmltree::Document::parse(&output)
                .unwrap()
                .root_element()
                .attribute("style"),
            Some("<&>")
        );
    }

    #[test]
    fn no_edits_preserve_source_without_an_owned_replacement() {
        let source = "<svg xmlns='http://www.w3.org/2000/svg'/>";
        assert_eq!(apply(source, Vec::new(), 0).unwrap(), None);
    }
}
