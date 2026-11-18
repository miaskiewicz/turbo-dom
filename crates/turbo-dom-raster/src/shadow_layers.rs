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
