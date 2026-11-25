//! Context-owned paint order, separate from layout/containing-block ancestry.
use super::value::Position;
use crate::{ComputedStyle, Fragment};

/// A fragment's own paint plus its scheduled children. Promoted positioned boxes
/// retain the clip-only ancestors they crossed; real contexts are never crossed.
pub struct PaintNode<'a> {
    pub fragment: &'a Fragment,
    pub children: Vec<PaintNode<'a>>,
    pub clips: Vec<&'a Fragment>,
    order: usize,
}

impl<'a> PaintNode<'a> {
    #[must_use]
    pub fn new(fragment: &'a Fragment) -> Self {
        context(fragment)
    }

    /// Own-content paint order; storage/geometry parent indices stay unchanged.
    pub fn visit(&self, callback: &mut impl FnMut(&'a Fragment)) {
        callback(self.fragment);
        for child in &self.children {
            child.visit(callback);
        }
    }
}

fn context(fragment: &Fragment) -> PaintNode<'_> {
    let mut promoted = Vec::new();
    let mut order = 0;
    let mut root = ordinary(fragment, &[], &mut promoted, &mut order, true);
    root.children.extend(promoted);
    root.children.sort_by_key(|node| {
        (
            node.fragment.paint_z(),
            node.fragment.stacks_as_positioned(),
            node.order,
        )
    });
    root
}

fn ordinary<'a>(
    fragment: &'a Fragment,
    ancestors: &[&'a Fragment],
    promoted: &mut Vec<PaintNode<'a>>,
    order: &mut usize,
    root: bool,
) -> PaintNode<'a> {
    let own_order = *order;
    *order += 1;
    let mut node = PaintNode {
        fragment,
        children: Vec::new(),
        clips: Vec::new(),
        order: own_order,
    };
    for child in &fragment.children {
        child_node(child, ancestors, &mut node, promoted, order, root);
    }
    node
}

fn child_node<'a>(
    child: &'a Fragment,
    ancestors: &[&'a Fragment],
    parent: &mut PaintNode<'a>,
    promoted: &mut Vec<PaintNode<'a>>,
    order: &mut usize,
    root: bool,
) {
    let child_order = *order;
    let mut lineage = ancestors.to_vec();
    // The context root is already active. Its own frame must not be replayed.
    if !root && (parent.fragment.clip.is_some() || parent.fragment.overflow_clip.is_some()) {
        lineage.push(parent.fragment);
    }
    let mut node = if child.creates_stacking_context() {
        *order += 1;
        context(child)
    } else {
        ordinary(child, &lineage, promoted, order, false)
    };
    node.order = child_order;
    if child.stacks_as_positioned() {
        node.clips = lineage;
        promoted.push(node);
    } else {
        parent.children.push(node);
    }
}

pub(crate) fn style_context(style: &ComputedStyle, position: Position) -> bool {
    style.has_stacking_context()
        || matches!(position, Position::Fixed | Position::Sticky)
        || style.get("isolation") == Some("isolate")
        || containment_context(style.get("contain").unwrap_or(""))
        || [
            "filter",
            "backdrop-filter",
            "perspective",
            "clip-path",
            "mask-image",
        ]
        .iter()
        .any(|name| style.get(name).is_some_and(|value| value != "none"))
        || style
            .get("mix-blend-mode")
            .is_some_and(|value| value != "normal")
        || style.get("will-change").is_some_and(will_change_context)
}

fn containment_context(value: &str) -> bool {
    value
        .split_ascii_whitespace()
        .any(|token| matches!(token, "layout" | "paint" | "strict" | "content"))
}

fn will_change_context(value: &str) -> bool {
    value.split(',').any(|name| {
        matches!(
            name.trim(),
            "opacity"
                | "transform"
                | "filter"
                | "perspective"
                | "isolation"
                | "clip-path"
                | "mask"
                | "mask-image"
                | "mix-blend-mode"
                | "contain"
        )
    })
}

pub(crate) fn context_property(name: &str) -> bool {
    matches!(
        name,
        "opacity"
            | "transform"
            | "translate"
            | "rotate"
            | "scale"
            | "filter"
            | "backdrop-filter"
            | "perspective"
            | "clip-path"
            | "mask"
            | "mask-image"
            | "mix-blend-mode"
            | "isolation"
            | "contain"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FragmentContent, NodeId};

    fn fragment(id: u32, z: Option<i32>) -> Fragment {
        let mut fragment = Fragment::new(
            NodeId(id),
            0.0,
            0.0,
            10.0,
            10.0,
            FragmentContent::Box {
                background: None,
                border: crate::layout::value::BorderEdges::default(),
                border_radius: crate::BorderRadii::ZERO,
                shadow: None,
                gradient: None,
                transform: None,
            },
        );
        fragment.is_positioned = true;
        fragment.z_index = z;
        fragment
    }

    fn ids(root: &Fragment) -> Vec<u32> {
        let mut ids = Vec::new();
        PaintNode::new(root).visit(&mut |fragment| ids.push(fragment.node_id.0));
        ids
    }

    #[test]
    fn contexts_keep_nested_z_order_atomic_but_auto_ancestors_do_not() {
        let mut root = fragment(0, None);
        let mut auto = fragment(1, None);
        auto.children = vec![fragment(2, Some(100)), fragment(3, Some(-1))];
        root.children = vec![auto, fragment(4, Some(1))];
        assert_eq!(ids(&root), [0, 3, 1, 4, 2]);
        root.children[0].establishes_stacking_context = true;
        assert_eq!(ids(&root), [0, 1, 3, 2, 4]);
        assert_eq!(root.children[0].children[0].node_id, NodeId(2));
    }

    #[test]
    fn equal_z_uses_original_source_order_across_auto_ancestors() {
        let mut root = fragment(0, None);
        let mut auto = fragment(1, None);
        auto.children = vec![fragment(2, Some(1))];
        root.children = vec![auto, fragment(3, Some(1))];
        assert_eq!(ids(&root), [0, 1, 2, 3]);
    }

    #[test]
    fn independent_context_style_evidence_is_not_a_positioning_sentinel() {
        for css in [
            "isolation:isolate",
            "contain:layout",
            "filter:blur(0px)",
            "mix-blend-mode:multiply",
            "will-change:transform",
            "will-change:opacity,width",
        ] {
            let style = ComputedStyle::from_pairs(
                crate::style::parse_declarations(css)
                    .into_iter()
                    .map(|declaration| (declaration.property, declaration.value)),
            );
            assert!(style_context(&style, Position::Static), "{css}");
        }
        for css in [
            "isolation:auto",
            "contain:size",
            "filter:none",
            "mix-blend-mode:normal",
            "will-change:width",
        ] {
            let style = ComputedStyle::from_pairs(
                crate::style::parse_declarations(css)
                    .into_iter()
                    .map(|declaration| (declaration.property, declaration.value)),
            );
            assert!(!style_context(&style, Position::Static), "{css}");
        }
        assert!(style_context(
            &ComputedStyle::default().with_stacking_context(),
            Position::Static
        ));
        assert!(style_context(&ComputedStyle::default(), Position::Fixed));
        assert!(style_context(&ComputedStyle::default(), Position::Sticky));
    }
}
