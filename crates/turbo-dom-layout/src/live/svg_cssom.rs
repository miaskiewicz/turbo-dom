//! CSSOM no-box certification requires the original native namespace, not a tag guess.
use std::collections::{BTreeSet, HashSet};

use super::{source_document, source_handle, LiveCapture, LiveTree};
use crate::layout_snapshot::SourceId;
use crate::StyledNode;

pub(super) fn classify(
    live: &LiveTree,
    capture: &LiveCapture,
    styled: &[StyledNode],
) -> BTreeSet<SourceId> {
    if capture.document != live.document || capture.tree_version != live.tree.version {
        return BTreeSet::new();
    }
    let viewports: HashSet<_> = capture.svg_viewports.iter().copied().collect();
    capture
        .identities
        .iter()
        .filter(|identity| {
            source_document(identity.id) == live.document && !viewports.contains(&identity.id)
        })
        .filter(|identity| {
            let node = source_handle(identity.id);
            live.tree.node_type_id(node) == 1
                && live.tree.namespace_uri(node) == Some("http://www.w3.org/2000/svg")
                && live.tree.local_name(node) != Some("foreignObject")
                && element_at(styled, &identity.path) == Some(node)
        })
        .map(|identity| identity.id)
        .collect()
}

fn element_at(mut styled: &[StyledNode], path: &[usize]) -> Option<turbo_dom::Handle> {
    let (&last, parents) = path.split_last()?;
    for &index in parents {
        let StyledNode::Element(element) = styled.get(index)? else {
            return None;
        };
        styled = &element.children;
    }
    match styled.get(last)? {
        StyledNode::Element(element) => element.source,
        StyledNode::Text(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::live::{source_id, testing};
    use turbo_dom::Tree;

    fn captured(tree: &Tree) -> LiveCapture {
        testing::capture_light(tree, &testing::light_children(tree))
    }

    fn classify_capture(tree: &Tree, capture: &LiveCapture) -> BTreeSet<SourceId> {
        let children = testing::light_children(tree);
        let provider = |node| children.get(&node).map_or(&[][..], Vec::as_slice);
        let live = LiveTree {
            tree,
            document: testing::DOCUMENT,
            children: &provider,
        };
        let style = crate::live::LiveStyle {
            tree: live,
            origins: &[],
            scope_depth: &|_| Ok(0),
            inline_override: &|_| None,
            inline_declarations: &|text| Ok(crate::style::parse_declarations(text)),
        };
        let styled = crate::live::style::style(&style, capture).unwrap();
        classify(&live, capture, &styled)
    }

    #[test]
    fn svg_cssom_certification_requires_native_namespace_document_and_version() {
        let mut tree = Tree::parse("<!doctype html><svg id=root><circle id=shape /><foreignObject id=foreign><div id=html>text</div></foreignObject></svg>");
        let capture = captured(&tree);
        let shape = tree.query_selector("#shape").unwrap();
        let expected = BTreeSet::from([source_id(testing::DOCUMENT, shape)]);
        assert_eq!(classify_capture(&tree, &capture), expected);
        let mut wrong = capture.clone();
        wrong.document += 1;
        assert!(classify_capture(&tree, &wrong).is_empty());
        let mut wrong = capture.clone();
        wrong.tree_version -= 1;
        assert!(classify_capture(&tree, &wrong).is_empty());
        let mut wrong = capture.clone();
        for identity in &mut wrong.identities {
            if identity.id == source_id(testing::DOCUMENT, shape) {
                identity.id = source_id(testing::DOCUMENT + 1, shape);
            }
        }
        assert!(classify_capture(&tree, &wrong).is_empty());
        let html = source_id(testing::DOCUMENT, tree.query_selector("#html").unwrap());
        let html_path = capture
            .identities
            .iter()
            .find(|identity| identity.id == html)
            .unwrap()
            .path
            .clone();
        let mut wrong = capture.clone();
        wrong
            .identities
            .iter_mut()
            .find(|identity| identity.id == source_id(testing::DOCUMENT, shape))
            .unwrap()
            .path = html_path;
        assert!(classify_capture(&tree, &wrong).is_empty());
        let root = tree.query_selector("#root").unwrap();
        tree.remove_child(root, shape);
        assert!(classify_capture(&tree, &capture).is_empty());
        assert!(classify_capture(&tree, &captured(&tree)).is_empty());
    }

    #[test]
    fn svg_cssom_imported_local_names_and_detached_storage_are_not_namespace_proof() {
        let mut tree = Tree::parse("<!doctype html><svg id=root></svg>");
        let root = tree.query_selector("#root").unwrap();
        let svg = tree.create_element_namespace(Some("http://www.w3.org/2000/svg"), "circle");
        let html = tree.create_element_namespace(Some("http://www.w3.org/1999/xhtml"), "circle");
        let detached = tree.create_element_namespace(Some("http://www.w3.org/2000/svg"), "circle");
        tree.append_child(root, svg);
        tree.append_child(root, html);
        let classified = classify_capture(&tree, &captured(&tree));
        assert_eq!(
            classified,
            BTreeSet::from([source_id(testing::DOCUMENT, svg)])
        );
        assert!(!classified.contains(&source_id(testing::DOCUMENT, html)));
        assert!(!classified.contains(&source_id(testing::DOCUMENT, detached)));
    }
}
