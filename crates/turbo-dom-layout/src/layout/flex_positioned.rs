//! Keep normal-flow percentage definiteness in the content solve. Only absolute
//! children use a second root with the finalized positioned padding-box frame.
use super::{
    container_style, frames, item_style, place_one, solve, AlignItems, BoxStyle, Ctx, Dimension,
    Fill, Fragment, LayoutBox, LengthPct, LengthPercentageAuto, ResolveCtx, Size, TaffyId,
    TaffyTree,
};

pub(super) struct Root<'a> {
    pub container: &'a LayoutBox,
    pub style: &'a BoxStyle,
    pub origin: (f32, f32),
    pub size: Size<f32>,
}

fn absolute_tree(
    root: &Root<'_>,
    items: &[LayoutBox],
    column: bool,
    ctx: &Ctx,
) -> Option<(TaffyTree<usize>, Vec<Option<TaffyId>>)> {
    if !items.iter().any(LayoutBox::out_of_flow) {
        return None;
    }
    let mut tree = TaffyTree::new();
    let fs = root.style.font_size;
    let mut leaves = vec![None; items.len()];
    let absolute: Vec<_> = items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            if !item.out_of_flow() {
                return None;
            }
            let (bs, edges) = item.resolved_with_edges(ResolveCtx {
                parent_font_size: fs,
                cb_width: 0.0,
            });
            let mut style = frames::style(item_style(item, &bs, fs, column), &bs, edges);
            // abs_item_inset's auto-axis start is a static content-edge anchor,
            // not an authored zero inset against the positioned padding box.
            if bs.inset_left == LengthPct::Auto && bs.inset_right == LengthPct::Auto {
                style.inset.left = LengthPercentageAuto::length(root.style.padding.left);
            }
            let leaf = tree.new_leaf_with_context(style, index);
            leaves[index] = Some(leaf);
            Some(leaf)
        })
        .collect();
    let mut style = container_style(root.container, root.size.width, fs, Some(root.size.height));
    // The first solve already applied min/max sizes. Freezing its used content
    // height here must not make in-flow percentage heights definite.
    style.min_size.height = Dimension::auto();
    style.max_size.height = Dimension::auto();
    let id = tree.new_with_children(frames::container(style, root.style), &absolute);
    solve(
        &mut tree,
        id,
        items,
        (fs, root.size.width),
        Fill {
            width: root.size.width,
            row: !column,
            column_height: column.then_some(true),
        },
        ctx,
    );
    Some((tree, leaves))
}

pub(super) fn place(
    tree: &TaffyTree<usize>,
    leaves: &[TaffyId],
    items: &[LayoutBox],
    root: &Root<'_>,
    axis: (bool, AlignItems, bool),
    ctx: &mut Ctx,
) -> Vec<Fragment> {
    let absolute = absolute_tree(root, items, axis.0, ctx);
    let border = root.style.border.widths();
    let absolute_origin = (
        root.origin.0 - root.style.padding.left - border.left,
        root.origin.1 - root.style.padding.top - border.top,
    );
    leaves
        .iter()
        .enumerate()
        .map(|(index, leaf)| {
            let (tree, leaf, origin) =
                match absolute.as_ref().filter(|_| items[index].out_of_flow()) {
                    Some((tree, leaves)) => {
                        (tree, leaves[index].expect("absolute leaf"), absolute_origin)
                    }
                    None => (tree, *leaf, root.origin),
                };
            place_one(
                &items[index],
                tree.layout(leaf).expect("item layout"),
                tree.parent_size(leaf).width.unwrap_or(0.0),
                (origin.0, origin.1, root.style.font_size),
                Some(axis),
                ctx,
            )
        })
        .collect()
}
