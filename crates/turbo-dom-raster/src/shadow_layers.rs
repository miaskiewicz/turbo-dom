//! Snapshot-owned paint layers, indexed only by this layout's recorded source ids.
use crate::image_paint::DecodedAssets;
use std::collections::HashMap;
use turbo_dom_layout::layout::value::{box_shadow_layers, parse_px, ResolveCtx};
use turbo_dom_layout::turbo_dom::Handle;
use turbo_dom_layout::{BoxShadow, Fragment, FragmentContent, NodeId, NodeSources, StyledNode};

pub(crate) type ShadowLayers = HashMap<NodeId, Vec<BoxShadow>>;
pub(crate) type SourceLayers = HashMap<Handle, Vec<BoxShadow>>;

pub(crate) fn collect(nodes: &[StyledNode], parent_font: f32, width: f32, out: &mut SourceLayers) {
    for node in nodes {
        let StyledNode::Element(element) = node else {
            continue;
        };
        let style = &element.style;
        let font = style
            .get("font-size")
            .and_then(|v| parse_px(v, parent_font))
            .unwrap_or(parent_font);
        let layers = box_shadow_layers(
            style,
            ResolveCtx {
                parent_font_size: parent_font,
                cb_width: width,
            },
        );
        if layers.len() > 1 {
            if let Some(source) = element.source {
                out.insert(source, layers);
            }
        }
        collect(&element.children, font, width, out);
    }
}

pub(crate) fn remap(
    sources: Option<&NodeSources>,
    layers: &SourceLayers,
    galley: &Fragment,
) -> ShadowLayers {
    let mut output = ShadowLayers::new();
    if let Some(sources) = sources {
        remap_fragment(galley, None, sources, layers, &mut output);
    }
    output
}

fn remap_fragment(
    f: &Fragment,
    inherited: Option<Handle>,
    sources: &NodeSources,
    layers: &SourceLayers,
    output: &mut ShadowLayers,
) {
    let source = sources.elements.get(&f.node_id).copied();
    let owner = if sources.table_grids.contains(&f.node_id) {
        inherited
    } else {
        source
    };
    if matches!(
        f.content,
        FragmentContent::Box {
            shadow: Some(_),
            ..
        }
    ) {
        if let Some(layers) = owner.and_then(|h| layers.get(&h)) {
            output.insert(f.node_id, layers.clone());
        }
    }
    for child in &f.children {
        remap_fragment(child, source.or(inherited), sources, layers, output);
    }
}

pub(crate) fn for_fragment<'a>(
    f: &'a Fragment,
    layers: Option<&'a ShadowLayers>,
) -> &'a [BoxShadow] {
    if let Some(layers) = layers.and_then(|m| m.get(&f.node_id)) {
        return layers;
    }
    match &f.content {
        FragmentContent::Box {
            shadow: Some(shadow),
            ..
        } => std::slice::from_ref(shadow),
        _ => &[],
    }
}

pub(crate) fn outer_bounds(f: &Fragment, shadows: &[BoxShadow]) -> Option<(f32, f32, f32, f32)> {
    if !crate::paint_png::fragment_finite_positive(f) {
        return None;
    }
    let mut bounds: Option<(f32, f32, f32, f32)> = None;
    for sh in shadows.iter().filter(|s| !s.inset && s.color.a > 0) {
        let reach = sh.spread.max(0) as f32 + 3.0 * crate::paint_png::shadow_blur(sh) + 1.0;
        let (x, y) = (f.x + sh.offset_x as f32, f.y + sh.offset_y as f32);
        let rect = (
            x - reach,
            y - reach,
            x + f.width + reach,
            y + f.height + reach,
        );
        bounds = Some(bounds.map_or(rect, |(l, t, r, b)| {
            (l.min(rect.0), t.min(rect.1), r.max(rect.2), b.max(rect.3))
        }));
    }
    bounds
        .and_then(|(l, t, r, b)| tiny_skia::Rect::from_ltrb(l, t, r, b))
        .map(|r| (r.x(), r.y(), r.width(), r.height()))
}

pub(crate) trait PaintInput {
    fn images(&self) -> &DecodedAssets;
    fn layers(&self) -> Option<&ShadowLayers> {
        None
    }
    fn multiple(&self, fragment: &Fragment) -> bool {
        self.layers()
            .is_some_and(|layers| layers.contains_key(&fragment.node_id))
    }
    fn shadows<'a>(&'a self, fragment: &'a Fragment) -> &'a [BoxShadow] {
        for_fragment(fragment, self.layers())
    }
}

impl PaintInput for DecodedAssets {
    fn images(&self) -> &DecodedAssets {
        self
    }
}

pub(crate) struct PaintContext<'a> {
    pub(crate) images: &'a DecodedAssets,
    pub(crate) layers: &'a ShadowLayers,
}

impl PaintInput for PaintContext<'_> {
    fn images(&self) -> &DecodedAssets {
        self.images
    }
    fn layers(&self) -> Option<&ShadowLayers> {
        Some(self.layers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use turbo_dom_layout::layout::value::BorderEdges;
    use turbo_dom_layout::{BorderRadii, ComputedStyle, Rgba, StyledElement, Tag};

    fn shadow(offset: i32) -> BoxShadow {
        BoxShadow {
            offset_x: offset,
            offset_y: offset,
            blur: 0,
            spread: 0,
            color: Rgba::BLACK,
            inset: false,
        }
    }

    fn frame(id: u32, painted: bool) -> Fragment {
        Fragment::new(
            NodeId(id),
            10.0,
            20.0,
            40.0,
            30.0,
            FragmentContent::Box {
                background: None,
                border: BorderEdges::default(),
                border_radius: BorderRadii::default(),
                shadow: painted.then(|| shadow(1)),
                gradient: None,
                transform: None,
            },
        )
    }

    fn styled(source: Option<Handle>, value: &str, children: Vec<StyledNode>) -> StyledNode {
        StyledNode::Element(StyledElement {
            tag: Tag::Html("div".into()),
            attrs: vec![],
            style: ComputedStyle::from_pairs([("box-shadow", value)]),
            children,
            source,
        })
    }

    #[test]
    fn synthetic_style_ancestors_do_not_hide_owned_descendants() {
        let tree = turbo_dom_layout::turbo_dom::Tree::parse("<div id=a></div><div id=b></div>");
        let a = tree.query_selector("#a").unwrap();
        let b = tree.query_selector("#b").unwrap();
        let nodes = [styled(
            None,
            "1px 1px red,2px 2px blue",
            vec![
                StyledNode::Text("text".into()),
                styled(Some(a), "1em 2em red,3px 4px blue", vec![]),
                styled(Some(b), "1px 1px red", vec![]),
            ],
        )];
        let mut found = SourceLayers::new();
        collect(&nodes, 20.0, 200.0, &mut found);
        assert_eq!(found.len(), 1);
        assert_eq!((found[&a][0].offset_x, found[&a][0].offset_y), (20, 40));
        assert!(!found.contains_key(&b));
    }

    #[test]
    fn only_table_grids_inherit_the_nearest_real_source() {
        let tree =
            turbo_dom_layout::turbo_dom::Tree::parse("<table id=a></table><table id=b></table>");
        let a = tree.query_selector("#a").unwrap();
        let b = tree.query_selector("#b").unwrap();
        let layers = SourceLayers::from([
            (a, vec![shadow(1), shadow(2)]),
            (b, vec![shadow(3), shadow(4)]),
        ]);
        let mut root = frame(0, false);
        let mut nested = frame(3, false);
        let mut anonymous = frame(6, false);
        anonymous.children.push(frame(7, true));
        nested.children = vec![frame(4, true), frame(5, true), anonymous];
        root.children = vec![frame(1, true), frame(2, true), nested];
        let mut sources = NodeSources::default();
        sources
            .elements
            .extend([(NodeId(0), a), (NodeId(3), b), (NodeId(5), a)]);
        sources
            .table_grids
            .extend([NodeId(1), NodeId(4), NodeId(7)]);
        let mapped = remap(Some(&sources), &layers, &root);
        assert_eq!(mapped.len(), 4);
        assert_eq!(mapped[&NodeId(1)], layers[&a]);
        assert_eq!(mapped[&NodeId(5)], layers[&a]);
        assert_eq!(mapped[&NodeId(4)], layers[&b]);
        assert_eq!(mapped[&NodeId(7)], layers[&b]);
        assert!(
            !mapped.contains_key(&NodeId(2)),
            "ordinary anonymous boxes must not duplicate a parent's shadows"
        );
        assert!(remap(None, &layers, &root).is_empty());
        assert!(remap(Some(&sources), &SourceLayers::new(), &root).is_empty());
    }

    #[test]
    fn invisible_layers_do_not_expand_opacity_bounds() {
        let f = frame(0, true);
        let visible = shadow(3);
        let mut transparent = shadow(10000);
        transparent.color.a = 0;
        let mut inset = shadow(-10000);
        inset.inset = true;
        assert_eq!(
            outer_bounds(&f, &[visible, transparent, inset]),
            outer_bounds(&f, &[visible])
        );
        assert_eq!(outer_bounds(&f, &[transparent, inset]), None);
        for width in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            let mut invalid = f.clone();
            invalid.width = width;
            assert_eq!(outer_bounds(&invalid, &[visible]), None);
        }
    }

    #[test]
    fn legacy_paint_input_and_unrelated_metadata_keep_the_fragment_shadow() {
        let images = DecodedAssets::new();
        let f = frame(10, true);
        let layers = ShadowLayers::from([(NodeId(11), vec![shadow(5), shadow(6)])]);
        let context = PaintContext {
            images: &images,
            layers: &layers,
        };
        assert!(!images.multiple(&f));
        assert!(!context.multiple(&f));
        assert_eq!(images.shadows(&f), &[shadow(1)]);
        assert_eq!(context.shadows(&f), images.shadows(&f));
        assert!(images.shadows(&frame(12, false)).is_empty());
        assert!(context.images().is_empty());
    }
}
