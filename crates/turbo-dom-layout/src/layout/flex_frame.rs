//! Keep item frames typed until Taffy knows the containing block or grid area.
use super::{
    measure_item, native_tree, AvailableSpace, BoxSizing, BoxStyle, Ctx, Fill, LayoutBox,
    LengthPercentage, LengthPercentageAuto, Rect, ResolveCtx, Size, Style,
};
use crate::layout::value::{edge_lengths, Edges, LengthPct};
use crate::style::ComputedStyle;
use taffy::ResolveOrZero;

fn rect<T>([top, right, bottom, left]: [T; 4]) -> Rect<T> {
    Rect {
        top,
        right,
        bottom,
        left,
    }
}

fn padding_value(value: LengthPct) -> LengthPercentage {
    match value {
        LengthPct::Px(v) => LengthPercentage::length(v),
        LengthPct::Pct(v) => LengthPercentage::percent(v / 100.0),
        LengthPct::Calc { pct, px } => LengthPercentage::calc(native_tree::calc(pct, px)),
        LengthPct::Auto => LengthPercentage::length(0.0),
    }
}

pub(super) fn style(mut style: Style, item: &LayoutBox, fs: f32) -> Style {
    let bs = item.resolved(ResolveCtx {
        parent_font_size: fs,
        cb_width: 0.0,
    });
    style.box_sizing = match bs.box_sizing {
        BoxSizing::ContentBox => taffy::BoxSizing::ContentBox,
        BoxSizing::BorderBox => taffy::BoxSizing::BorderBox,
    };
    style.padding = rect(edge_lengths(&item.style, "padding", bs.font_size).map(padding_value));
    style.margin = rect(
        edge_lengths(&item.style, "margin", bs.font_size).map(|value| {
            if value == LengthPct::Auto {
                LengthPercentageAuto::auto()
            } else {
                padding_value(value).into()
            }
        }),
    );
    let edges = bs.border.widths();
    style.border =
        rect([edges.top, edges.right, edges.bottom, edges.left].map(LengthPercentage::length));
    // Preserve the existing zero automatic minimum policy; padding/borders are
    // now enforced by Taffy's frame floor rather than a folded pixel minimum.
    for minimum in [&mut style.min_size.width, &mut style.min_size.height] {
        if minimum.is_auto() {
            *minimum = taffy::Dimension::length(0.0);
        }
    }
    style
}

pub(super) fn edges(value: Rect<f32>) -> Edges {
    Edges {
        top: value.top,
        right: value.right,
        bottom: value.bottom,
        left: value.left,
    }
}

fn normal_alignment(value: Option<&str>, parent: Option<&str>) -> bool {
    match value.map(str::trim) {
        None | Some("auto") => parent.is_none_or(|value| value.trim() == "normal"),
        Some("normal") => true,
        Some(_) => false,
    }
}

pub(super) fn grid_alignment(style: &mut Style, item: &LayoutBox, parent: &ComputedStyle) {
    if item.svg_viewport.is_none() && !item.image.as_ref().is_some_and(|image| image.replaced) {
        return;
    }
    // CSS grid's normal alignment stretches ordinary boxes, but intrinsic
    // replaced items keep their natural size. Explicit stretch still wins.
    if normal_alignment(item.style.get("justify-self"), parent.get("justify-items")) {
        style.justify_self = Some(taffy::AlignItems::Start);
    } else if style.justify_self.is_none() {
        style.justify_self = super::justify_items(parent);
    }
    if normal_alignment(item.style.get("align-self"), parent.get("align-items")) {
        style.align_self = Some(taffy::AlignItems::Start);
    } else if style.align_self.is_none() {
        style.align_self = super::align_items(parent);
    }
}

fn viewport_known(
    known: Size<Option<f32>>,
    avail: Size<AvailableSpace>,
    style: &Style,
    fill: Fill,
) -> Size<Option<f32>> {
    if fill.row {
        return Size {
            width: known.width.or(avail.width.into_option()),
            ..known
        };
    }
    if fill.column_height.is_some() {
        return Size {
            height: known.height.or(avail.height.into_option()),
            ..known
        };
    }
    // PerformLayout callbacks omit known dimensions. An explicitly stretched
    // grid viewport still needs the assigned width to derive its aspect height.
    Size {
        width: known.width.or_else(|| {
            (style.justify_self == Some(taffy::AlignItems::Stretch))
                .then(|| avail.width.into_option())
                .flatten()
        }),
        height: known.height.or_else(|| {
            (style.align_self == Some(taffy::AlignItems::Stretch))
                .then(|| avail.height.into_option())
                .flatten()
        }),
    }
}

fn viewport_measure(
    item: &LayoutBox,
    mut style: BoxStyle,
    known: Size<Option<f32>>,
    parent: Size<Option<f32>>,
    (fs, fill): (f32, Fill),
    body_height: Option<f32>,
) -> Size<f32> {
    if super::content_column_basis(item, fs, fill) && known.height.is_none() {
        style.height = LengthPct::Auto;
    }
    if let Some(width) = known.width {
        super::used_width(&mut style, width);
    }
    let frame = Size {
        width: style.padding.horizontal() + style.border.widths().horizontal(),
        height: style.padding.vertical() + style.border.widths().vertical(),
    };
    if let Some(height) = known.height {
        let inset = if style.box_sizing == BoxSizing::ContentBox {
            frame.height
        } else {
            0.0
        };
        style.height = LengthPct::Px((height - inset).max(0.0));
        style.min_height = LengthPct::Auto;
        style.max_height = LengthPct::Auto;
    }
    let viewport = item.svg_viewport.as_ref().expect("viewport item");
    let (width, height) = viewport.size(&super::super::imgsize::SizeCtx {
        style: &style,
        cb_width: parent.width.unwrap_or(fill.width),
        cb_height: parent.height,
        body_height,
    });
    Size {
        width: width + frame.width,
        height: height + frame.height,
    }
}

pub(super) fn measure(
    known: Size<Option<f32>>,
    avail: Size<AvailableSpace>,
    parent: Size<Option<f32>>,
    style: &Style,
    (item, fs): (&LayoutBox, f32),
    ctx: &Ctx,
    fill: Fill,
) -> Size<f32> {
    let padding = style
        .padding
        .resolve_or_zero(parent.width, native_tree::resolve_calc);
    let border = style
        .border
        .resolve_or_zero(parent.width, native_tree::resolve_calc);
    let edges = edges(padding + border);
    let frame = Size {
        width: edges.horizontal(),
        height: edges.vertical(),
    };
    let mut bs = item.resolved(ResolveCtx {
        parent_font_size: fs,
        cb_width: parent.width.unwrap_or(0.0),
    });
    // The intrinsic grid pass can defer cyclic edges. Use the solver's current
    // style, not the original percentage declaration, for that measurement.
    bs.padding = self::edges(padding);
    let border_avail = Size {
        width: avail.width.map_definite_value(|v| v + frame.width),
        height: avail.height.map_definite_value(|v| v + frame.height),
    };
    let measured = if item.svg_viewport.is_some() {
        let minimum = super::specified_content_minimum(
            &bs,
            avail.height,
            super::content_column_basis(item, fs, fill),
        );
        let viewport_known = viewport_known(known, border_avail, style, fill);
        let measured = viewport_measure(
            item,
            bs.clone(),
            viewport_known,
            parent,
            (fs, fill),
            ctx.images.body_height,
        );
        if known.height.is_none() && measured.height > minimum {
            // The automatic minimum changes a ratio-derived cross size too.
            // Returning a capped height with the old width would poison Taffy's
            // otherwise valid cache reuse for the later known-height probe.
            viewport_measure(
                item,
                bs,
                Size {
                    height: Some(minimum),
                    ..viewport_known
                },
                parent,
                (fs, fill),
                ctx.images.body_height,
            )
        } else {
            measured
        }
    } else {
        measure_item(
            known,
            border_avail,
            item,
            fs,
            ctx.fonts,
            ctx.images,
            (
                ctx.live_geometry,
                ctx.html_font_metrics,
                ctx.html_font_scale,
                ctx.original_units,
                fill,
                &bs,
            ),
        )
    };
    // Taffy's leaf callback takes border-box known dimensions, offers content
    // space, and expects content measurements; native layout returns a border box.
    Size {
        width: (measured.width - frame.width).max(0.0),
        height: (measured.height - frame.height).max(0.0),
    }
}
