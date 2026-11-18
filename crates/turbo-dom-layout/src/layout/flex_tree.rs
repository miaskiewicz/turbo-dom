//! Flat native-measured leaves over Taffy's low-level API. Its high-level tree
//! hardcodes calc resolution to zero, including in leaf sizing.
use std::cell::{Cell, RefCell};
use taffy::{
    AvailableSpace, Cache, CacheTree, Layout, LayoutFlexboxContainer, LayoutGridContainer,
    LayoutInput, LayoutOutput, LayoutPartialTree, NodeId, RoundTree, RunMode, Size, Style,
    TraversePartialTree, TraverseTree,
};

thread_local! { static CALCS: RefCell<Vec<(f32, f32, f32)>> = const { RefCell::new(Vec::new()) }; }

thread_local! { static SCALE: Cell<f32> = const { Cell::new(1.0) }; }

pub(super) struct CalcScope(usize, f32);
impl CalcScope {
    pub fn new(scale: Option<f32>) -> Self {
        let previous = SCALE.replace(scale.unwrap_or(1.0));
        Self(CALCS.with(|values| values.borrow().len()), previous)
    }
}
impl Drop for CalcScope {
    fn drop(&mut self) {
        CALCS.with(|values| values.borrow_mut().truncate(self.0));
        SCALE.set(self.1);
    }
}

pub(super) fn calc(pct: f32, px: f32) -> *const () {
    CALCS.with(|values| {
        let mut values = values.borrow_mut();
        let index = values.len();
        values.push((pct, px, SCALE.get()));
        // Opaque aligned token, never dereferenced. Taffy preserves this identity.
        std::ptr::without_provenance((index + 1) * 8)
    })
}

pub(super) fn calc_fixed(token: *const ()) -> f32 {
    CALCS.with(|values| values.borrow()[token.addr() / 8 - 1].1)
}

pub(super) fn offset_calc(token: *const (), offset: f32) -> *const () {
    let (pct, px, _) = CALCS.with(|values| values.borrow()[token.addr() / 8 - 1]);
    calc(pct, px + offset)
}

pub(super) fn resolve_calc(token: *const (), basis: f32) -> f32 {
    CALCS.with(|values| {
        let (pct, px, scale) = values.borrow()[token.addr() / 8 - 1];
        let value = f64::from(pct) / 100.0 * f64::from(basis) + f64::from(px);
        // Convert the resolved CSS length once to a LayoutUnit. Rounding each
        // coefficient in f32 can make three exact thirds exceed the container.
        let units = 64.0 * f64::from(scale);
        ((value * units).trunc() / units) as f32
    })
}

struct Node<C> {
    style: Style,
    context: Option<C>,
    children: Vec<NodeId>,
    layout: Layout,
    rounded: Layout,
    cache: Cache,
    details: taffy::DetailedLayoutInfo,
}

pub(super) struct Tree<C> {
    nodes: Vec<Node<C>>,
    rounding: bool,
    dirty: bool,
}
impl<C> Tree<C> {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            rounding: true,
            dirty: false,
        }
    }
    fn add(&mut self, style: Style, context: Option<C>, children: Vec<NodeId>) -> NodeId {
        let id = NodeId::from(self.nodes.len());
        self.nodes.push(Node {
            style,
            context,
            children,
            layout: Layout::default(),
            rounded: Layout::default(),
            cache: Cache::new(),
            details: taffy::DetailedLayoutInfo::None,
        });
        id
    }
    pub fn new_leaf_with_context(&mut self, style: Style, context: C) -> NodeId {
        self.add(style, Some(context), Vec::new())
    }
    pub fn new_with_children(&mut self, style: Style, children: &[NodeId]) -> NodeId {
        self.add(style, None, children.to_vec())
    }
    pub fn style(&self, id: NodeId) -> Option<&Style> {
        self.nodes.get(usize::from(id)).map(|node| &node.style)
    }
    pub fn set_style(&mut self, id: NodeId, style: Style) -> Option<()> {
        self.nodes.get_mut(usize::from(id))?.style = style;
        self.dirty = true;
        Some(())
    }
    pub fn detailed_layout_info(&self, id: NodeId) -> &taffy::DetailedLayoutInfo {
        &self.nodes[usize::from(id)].details
    }
    pub fn disable_rounding(&mut self) {
        self.rounding = false;
    }
    pub fn layout(&self, id: NodeId) -> Option<&Layout> {
        let node = self.nodes.get(usize::from(id))?;
        Some(if self.rounding {
            &node.rounded
        } else {
            &node.layout
        })
    }
    pub fn compute_layout_with_measure<F>(
        &mut self,
        root: NodeId,
        avail: Size<AvailableSpace>,
        mut measure: F,
    ) where
        F: FnMut(
            Size<Option<f32>>,
            Size<AvailableSpace>,
            NodeId,
            Option<&mut C>,
            &Style,
        ) -> Size<f32>,
    {
        // A batch of changed leaf styles invalidates the arena once, avoiding
        // a full cache walk for every item in grid's intrinsic/final phases.
        if self.dirty {
            for node in &mut self.nodes {
                node.cache.clear();
            }
            self.dirty = false;
        }
        let mut view = View {
            tree: self,
            measure: &mut measure,
        };
        taffy::compute_root_layout(&mut view, root, avail);
        if view.tree.rounding {
            taffy::round_layout(&mut view, root);
        }
    }
}

type Measure<'a, C> = dyn FnMut(Size<Option<f32>>, Size<AvailableSpace>, NodeId, Option<&mut C>, &Style) -> Size<f32>
    + 'a;
struct View<'a, C> {
    tree: &'a mut Tree<C>,
    measure: &'a mut Measure<'a, C>,
}

impl<C> TraversePartialTree for View<'_, C> {
    type ChildIter<'a>
        = std::iter::Copied<std::slice::Iter<'a, NodeId>>
    where
        Self: 'a;
    fn child_ids(&self, id: NodeId) -> Self::ChildIter<'_> {
        self.tree.nodes[usize::from(id)].children.iter().copied()
    }
    fn child_count(&self, id: NodeId) -> usize {
        self.tree.nodes[usize::from(id)].children.len()
    }
    fn get_child_id(&self, id: NodeId, index: usize) -> NodeId {
        self.tree.nodes[usize::from(id)].children[index]
    }
}
impl<C> TraverseTree for View<'_, C> {}
impl<C> LayoutPartialTree for View<'_, C> {
    type CoreContainerStyle<'a>
        = &'a Style
    where
        Self: 'a;
    fn get_core_container_style(&self, id: NodeId) -> &Style {
        &self.tree.nodes[usize::from(id)].style
    }
    fn set_unrounded_layout(&mut self, id: NodeId, layout: &Layout) {
        self.tree.nodes[usize::from(id)].layout = *layout;
    }
    fn resolve_calc_value(&self, token: *const (), basis: f32) -> f32 {
        resolve_calc(token, basis)
    }
    fn compute_child_layout(&mut self, id: NodeId, inputs: LayoutInput) -> LayoutOutput {
        if inputs.run_mode == RunMode::PerformHiddenLayout {
            return taffy::compute_hidden_layout(self, id);
        }
        taffy::compute_cached_layout(self, id, inputs, |view, id, inputs| {
            view.compute(id, inputs)
        })
    }
}
impl<C> View<'_, C> {
    fn compute(&mut self, id: NodeId, inputs: LayoutInput) -> LayoutOutput {
        let node = &mut self.tree.nodes[usize::from(id)];
        if node.style.display == taffy::Display::None {
            return taffy::compute_hidden_layout(self, id);
        }
        if !node.children.is_empty() {
            return match node.style.display {
                taffy::Display::Grid => taffy::compute_grid_layout(self, id, inputs),
                _ => taffy::compute_flexbox_layout(self, id, inputs),
            };
        }
        taffy::compute_leaf_layout(inputs, &node.style, resolve_calc, |known, avail| {
            (self.measure)(known, avail, id, node.context.as_mut(), &node.style)
        })
    }
}
impl<C> CacheTree for View<'_, C> {
    fn cache_get(
        &self,
        id: NodeId,
        known: Size<Option<f32>>,
        avail: Size<AvailableSpace>,
        mode: RunMode,
    ) -> Option<LayoutOutput> {
        self.tree.nodes[usize::from(id)]
            .cache
            .get(known, avail, mode)
    }
    fn cache_store(
        &mut self,
        id: NodeId,
        known: Size<Option<f32>>,
        avail: Size<AvailableSpace>,
        mode: RunMode,
        output: LayoutOutput,
    ) {
        self.tree.nodes[usize::from(id)]
            .cache
            .store(known, avail, mode, output);
    }
    fn cache_clear(&mut self, id: NodeId) {
        self.tree.nodes[usize::from(id)].cache.clear();
    }
}
impl<C> RoundTree for View<'_, C> {
    fn get_unrounded_layout(&self, id: NodeId) -> &Layout {
        &self.tree.nodes[usize::from(id)].layout
    }
    fn set_final_layout(&mut self, id: NodeId, layout: &Layout) {
        self.tree.nodes[usize::from(id)].rounded = *layout;
    }
}
impl<C> LayoutFlexboxContainer for View<'_, C> {
    type FlexboxContainerStyle<'a>
        = &'a Style
    where
        Self: 'a;
    type FlexboxItemStyle<'a>
        = &'a Style
    where
        Self: 'a;
    fn get_flexbox_container_style(&self, id: NodeId) -> &Style {
        &self.tree.nodes[usize::from(id)].style
    }
    fn get_flexbox_child_style(&self, id: NodeId) -> &Style {
        &self.tree.nodes[usize::from(id)].style
    }
}
impl<C> LayoutGridContainer for View<'_, C> {
    type GridContainerStyle<'a>
        = &'a Style
    where
        Self: 'a;
    type GridItemStyle<'a>
        = &'a Style
    where
        Self: 'a;
    fn set_detailed_grid_info(&mut self, id: NodeId, info: taffy::DetailedGridInfo) {
        self.tree.nodes[usize::from(id)].details = taffy::DetailedLayoutInfo::Grid(Box::new(info));
    }
    fn get_grid_container_style(&self, id: NodeId) -> &Style {
        &self.tree.nodes[usize::from(id)].style
    }
    fn get_grid_child_style(&self, id: NodeId) -> &Style {
        &self.tree.nodes[usize::from(id)].style
    }
}

#[cfg(test)]
pub(super) fn resolve_for_test(token: *const (), basis: f32) -> f32 {
    resolve_calc(token, basis)
}

#[cfg(test)]
mod tests {
    use super::*;
    use taffy::prelude::TaffyMaxContent;

    #[test]
    fn nested_calc_scopes_preserve_tokens_and_zoom_policy() {
        let outer = CalcScope::new(Some(2.0));
        let token = calc(50.0, -0.003);
        let expected = resolve_calc(token, 10.0);
        assert_eq!(expected, 4.992_187_5);
        {
            let _inner = CalcScope::new(None);
            let nested = calc(25.0, 0.003);
            assert_eq!(resolve_calc(nested, 10.0), 2.5);
            assert_eq!(resolve_calc(token, 10.0), expected);
        }
        let offset = offset_calc(token, 2.0);
        assert_eq!(resolve_calc(offset, 10.0), 6.992_187_5);
        drop(outer);
    }

    #[test]
    fn hidden_nodes_and_rounding_follow_taffy_layout_contract() {
        let mut tree: Tree<usize> = Tree::new();
        let hidden = tree.new_leaf_with_context(
            Style {
                display: taffy::Display::None,
                ..Style::default()
            },
            0,
        );
        let root = tree.new_with_children(
            Style {
                size: Size {
                    width: taffy::Dimension::length(20.4),
                    height: taffy::Dimension::auto(),
                },
                ..Style::default()
            },
            &[hidden],
        );
        tree.compute_layout_with_measure(root, Size::MAX_CONTENT, |_, _, _, _, _| {
            panic!("hidden leaf measured")
        });
        assert_eq!(tree.layout(hidden).unwrap().size, Size::ZERO);
        assert_eq!(tree.layout(root).unwrap().size.width, 20.0);
        assert!(tree.layout(NodeId::from(99_usize)).is_none());
        tree.disable_rounding();
        assert_eq!(tree.layout(root).unwrap().size.width, 20.4);
    }
    #[test]
    fn hiding_an_ancestor_clears_and_restores_descendant_geometry() {
        let mut tree: Tree<usize> = Tree::new();
        let leaf = tree.new_leaf_with_context(
            Style {
                size: Size {
                    width: taffy::Dimension::length(20.0),
                    height: taffy::Dimension::length(10.0),
                },
                ..Style::default()
            },
            0,
        );
        let inner = tree.new_with_children(
            Style {
                size: Size {
                    width: taffy::Dimension::length(60.0),
                    height: taffy::Dimension::length(40.0),
                },
                ..Style::default()
            },
            &[leaf],
        );
        let visible = Style {
            justify_content: Some(taffy::JustifyContent::End),
            size: Size {
                width: taffy::Dimension::length(100.0),
                height: taffy::Dimension::length(80.0),
            },
            ..Style::default()
        };
        let root = tree.new_with_children(visible.clone(), &[inner]);
        tree.compute_layout_with_measure(root, Size::MAX_CONTENT, |_, _, _, _, _| Size::ZERO);
        let before = *tree.layout(leaf).unwrap();
        assert_eq!(
            before.size,
            Size {
                width: 20.0,
                height: 10.0
            }
        );
        assert_eq!(tree.layout(inner).unwrap().location.x, 40.0);
        tree.set_style(
            root,
            Style {
                display: taffy::Display::None,
                ..visible.clone()
            },
        )
        .unwrap();
        tree.compute_layout_with_measure(root, Size::MAX_CONTENT, |_, _, _, _, _| {
            panic!("a hidden ancestor must not measure its visible descendants")
        });
        for id in [root, inner, leaf] {
            let layout = tree.layout(id).unwrap();
            assert_eq!(layout.size, Size::ZERO);
            assert_eq!(layout.location, taffy::Point::ZERO);
        }
        tree.set_style(root, visible).unwrap();
        tree.compute_layout_with_measure(root, Size::MAX_CONTENT, |_, _, _, _, _| Size::ZERO);
        assert_eq!(*tree.layout(leaf).unwrap(), before);
        assert_eq!(tree.layout(inner).unwrap().location.x, 40.0);
    }
}
