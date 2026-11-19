//! Flex intrinsic probes omit their main-axis percentage basis; Taffy 0.8 also
//! resolves column flex-basis padding against height. Keep the definite inline
//! frame through both paths for this solve, then restore its typed declaration.
use super::{resolve_calc, LayoutInput, NodeId, View};
use taffy::{LengthPercentage, MaybeResolve, Rect, ResolveOrZero};

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
        if cyclic(node.style.padding) {
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
    use taffy::prelude::TaffyMaxContent;
    use taffy::{
        AlignItems, AvailableSpace, BoxSizing, Dimension, Display, FlexDirection, Size, Style,
    };

    #[test]
    fn indefinite_padding_preserves_taffy_intrinsic_geometry() {
        let padding = Rect {
            left: LengthPercentage::percent(0.1),
            right: LengthPercentage::percent(0.1),
            top: LengthPercentage::percent(0.1),
            bottom: LengthPercentage::percent(0.1),
        };
        let style = Style {
            padding,
            ..Style::default()
        };
        let mut native: Tree<usize> = Tree::new();
        let leaf = native.new_leaf_with_context(style.clone(), 0);
        let root = native.new_with_children(Style::default(), &[leaf]);
        native.compute_layout_with_measure(root, Size::MAX_CONTENT, |_, _, _, _, _, _| Size {
            width: 80.0,
            height: 20.0,
        });

        let mut reference: taffy::TaffyTree<usize> = taffy::TaffyTree::new();
        let reference_leaf = reference.new_leaf_with_context(style, 0).unwrap();
        let reference_root = reference
            .new_with_children(Style::default(), &[reference_leaf])
            .unwrap();
        reference
            .compute_layout_with_measure(reference_root, Size::MAX_CONTENT, |_, _, _, _, _| Size {
                width: 80.0,
                height: 20.0,
            })
            .unwrap();

        assert_eq!(
            native.layout(root).unwrap(),
            reference.layout(reference_root).unwrap()
        );
        assert_eq!(
            native.layout(leaf).unwrap(),
            reference.layout(reference_leaf).unwrap()
        );
        assert_eq!(native.style(leaf).unwrap().padding, padding);
        assert_eq!(
            native.layout(root).unwrap().size,
            Size {
                width: 80.0,
                height: 20.0
            }
        );

        // Resolving an intrinsic pass must not turn the original percentages
        // into permanent zero edges when a later pass has a definite basis.
        native
            .set_style(
                root,
                Style {
                    size: Size {
                        width: Dimension::length(200.0),
                        height: Dimension::auto(),
                    },
                    ..Style::default()
                },
            )
            .unwrap();
        native.compute_layout_with_measure(root, Size::MAX_CONTENT, |_, _, _, _, _, _| Size {
            width: 80.0,
            height: 20.0,
        });
        assert_eq!(native.layout(leaf).unwrap().padding.left, 20.0);
        assert_eq!(
            native.layout(leaf).unwrap().size,
            Size {
                width: 120.0,
                height: 60.0
            }
        );
        assert_eq!(native.style(leaf).unwrap().padding, padding);
    }

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
