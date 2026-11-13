//! The unquantized frame facts needed when walking an already laid-out flow.
use super::{
    length_prop, position_of, resolve_borders, resolve_edges, resolve_font_size, ComputedStyle,
    Edges, LengthPct, Position, ResolveCtx,
};

/// Typed frame facts for pagination and flow inspection, without text or paint values.
///
/// These have the same semantics as the corresponding [`super::BoxStyle`] fields:
/// percentages in margins and padding use the containing-block width; insets and
/// height retain their length-percentage form. HTML used-value quantization is
/// applied by layout after resolution and is not part of these facts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoxFlowStyle {
    /// The box's positioning scheme.
    pub position: Position,
    /// Vertical positioned offsets, retaining percentages and `auto`.
    pub inset_top: LengthPct,
    /// The bottom positioned offset, retaining percentages and `auto`.
    pub inset_bottom: LengthPct,
    /// Resolved physical margins.
    pub margin: Edges,
    /// Resolved physical padding.
    pub padding: Edges,
    /// Physical border widths, without their paint colours.
    pub border_widths: Edges,
    /// The declared height, retaining percentages and `auto`.
    pub height: LengthPct,
    /// The resolved font size used for relative lengths.
    pub font_size: f32,
}

/// Resolve flow-frame facts using the same parsers as [`super::resolve_box_style`].
/// This avoids resolving unused text, background, shadow and transform properties.
#[must_use]
pub fn resolve_box_flow_style(style: &ComputedStyle, ctx: ResolveCtx) -> BoxFlowStyle {
    crate::hot!("layout.resolve_box_flow_style");
    let fs = resolve_font_size(style, ctx.parent_font_size);
    BoxFlowStyle {
        position: position_of(style),
        inset_top: length_prop(style, "top", fs, LengthPct::Auto),
        inset_bottom: length_prop(style, "bottom", fs, LengthPct::Auto),
        margin: resolve_edges(style, "margin", fs, ctx.cb_width),
        padding: resolve_edges(style, "padding", fs, ctx.cb_width),
        border_widths: resolve_borders(style, fs).widths(),
        height: length_prop(style, "height", fs, LengthPct::Auto),
        font_size: fs,
    }
}
