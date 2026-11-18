//! Taffy 0.8 adds column flex-basis padding using the main-axis percentage basis.
//! Resolve it against the definite inline size for this solve, then restore it.
use super::{resolve_calc, LayoutInput, NodeId, View};
use taffy::{BoxSizing, FlexDirection, LengthPercentage, MaybeResolve, Rect, ResolveOrZero};

pub(super) type Saved = Vec<(NodeId, Rect<LengthPercentage>)>;

fn cyclic(padding: Rect<LengthPercentage>) -> bool {
    [padding.top, padding.right, padding.bottom, padding.left]
        .into_iter()
        .any(|value| {
            let raw = value.into_raw();
            raw.is_calc() || raw.tag() == taffy::CompactLength::PERCENT_TAG
        })
}

pub(super) fn resolve<C>(view: &mut View<'_, C>, id: NodeId, input: LayoutInput) -> Saved {
    let style = &view.tree.nodes[usize::from(id)].style;
    if !matches!(
        style.flex_direction,
        FlexDirection::Column | FlexDirection::ColumnReverse
    ) {
        return Vec::new();
    }
    let width = input.known_dimensions.width.or_else(|| {
        style
            .size
            .width
            .maybe_resolve(input.parent_size.width, resolve_calc)
    });
    let Some(width) = width else {
        return Vec::new();
    };
    let mut saved = Vec::new();
    for index in 0..view.tree.nodes[usize::from(id)].children.len() {
        let child = view.tree.nodes[usize::from(id)].children[index];
        let node = &mut view.tree.nodes[usize::from(child)];
        if node.style.box_sizing == BoxSizing::ContentBox && cyclic(node.style.padding) {
            saved.push((child, node.style.padding));
            node.style.padding = node
                .style
                .padding
                .resolve_or_zero(Some(width), resolve_calc)
                .map(LengthPercentage::length);
        }
    }
    saved
}

pub(super) fn restore<C>(view: &mut View<'_, C>, saved: Saved) {
    for (id, padding) in saved {
        view.tree.nodes[usize::from(id)].style.padding = padding;
    }
}

#[cfg(test)]
mod tests {
    use super::super::Tree;
    use super::*;
    use taffy::{AlignItems, AvailableSpace, Dimension, Display, Size, Style};

    #[test]
    fn column_padding_tracks_inline_size_without_losing_the_percentage() {
        let padding = Rect {
            top: LengthPercentage::percent(0.05),
            right: LengthPercentage::percent(0.05),
            bottom: LengthPercentage::percent(0.05),
            left: LengthPercentage::percent(0.05),
        };
        let mut tree: Tree<usize> = Tree::new();
        let leaf = tree.new_leaf_with_context(
            Style {
                size: Size {
                    width: Dimension::length(80.0),
                    height: Dimension::length(40.0),
                },
                box_sizing: BoxSizing::ContentBox,
                flex_basis: Dimension::length(40.0),
                padding,
                ..Style::default()
            },
            0,
        );
        let mut style = Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: Some(AlignItems::FlexStart),
            size: Size {
                width: Dimension::length(400.0),
                height: Dimension::length(240.0),
            },
            ..Style::default()
        };
        let root = tree.new_with_children(style.clone(), &[leaf]);
        for (width, height) in [(400.0, 80.0), (200.0, 60.0)] {
            style.size.width = Dimension::length(width);
            tree.set_style(root, style.clone()).unwrap();
            tree.compute_layout_with_measure(
                root,
                Size {
                    width: AvailableSpace::Definite(width),
                    height: AvailableSpace::Definite(240.0),
                },
                |_, _, _, _, _, _| Size {
                    width: 80.0,
                    height: 40.0,
                },
            );
            assert_eq!(tree.layout(leaf).unwrap().size.height, height);
            assert_eq!(tree.parent_size(leaf).width, Some(width));
            assert_eq!(tree.style(leaf).unwrap().padding, padding);
        }
    }
}
