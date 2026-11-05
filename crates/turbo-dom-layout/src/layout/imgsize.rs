//! Raster-image box sizing (§7.4, Phase 9b). Turns an image's intrinsic pixel
//! size into a painted box size that fits the page: scaled to preserve aspect
//! ratio and clamped so it never overflows.
//!
//! **Overflow caps (user spec).** Every image is bounded by
//! `max-width = 100%` of the containing block and `max-height ≈ 60%` of the page
//! body height. Because images are never split across pages, the height cap is
//! the mitigation that keeps an oversized image on a single page. When the page
//! body height is unknown at layout time (a region measured without geometry),
//! only the width cap applies — the height clamp is then a no-op hook the caller
//! fills by threading `body_height` (see `ImageCtx`).

use crate::image::{ImageResolver, Intrinsic};
use crate::style::ComputedStyle;

use super::fragment::ImagePlacement;
use super::value::{BoxStyle, LengthPct};

/// The fraction of the page body height an image may occupy (user spec).
const MAX_HEIGHT_FRACTION: f32 = 0.6;

/// A resolved image box: its painted px size and the placement to emit.
pub struct SizedImage {
    pub width: f32,
    pub height: f32,
    pub placement: ImagePlacement,
}

/// Inputs the sizer needs beyond the intrinsic dimensions.
pub struct SizeCtx<'a> {
    /// The box's resolved style (explicit `width`/`height` override intrinsic).
    pub style: &'a BoxStyle,
    /// Containing-block width — the basis for a `%` `width`/`max-width` and the
    /// 100% width cap.
    pub cb_width: f32,
    /// Containing-block height, if a definite one is known — the basis for a `%`
    /// `height` (e.g. an `<img height:100%>` filling a sized hero card). `None`
    /// leaves a `%` height to fall back to the intrinsic aspect ratio.
    pub cb_height: Option<f32>,
    /// Page body height, if known (the 60% height cap basis).
    pub body_height: Option<f32>,
}

/// Size a replaced `<img>` box from its intrinsic pixel dimensions and the
/// overflow caps, returning the painted box plus the placement to emit.
#[must_use]
pub fn size_replaced(name: String, intrinsic: Intrinsic, ctx: &SizeCtx) -> SizedImage {
    let (iw, ih) = match intrinsic.natural_ratio {
        // No natural size: the containing block's width at the natural ratio (#320).
        Some(ratio) => (stretch_width(ctx), stretch_width(ctx) / ratio),
        None => (intrinsic.width as f32, intrinsic.height as f32),
    };
    let (base_w, base_h) = base_size(iw, ih, ctx);
    let (width, height) = apply_caps(base_w, base_h, ctx);
    SizedImage {
        width,
        height,
        placement: placement_of(name, intrinsic),
    }
}

/// Live layout has CSS constraints, but no pagination overflow caps. `layout_scale` is
/// the physical scale the layout units of a ratio-derived axis quantize at.
pub(crate) fn size_replaced_live(
    name: String,
    intrinsic: Intrinsic,
    ctx: &SizeCtx,
    layout_scale: f32,
) -> SizedImage {
    let (iw, ih) = (intrinsic.width as f32, intrinsic.height as f32);
    let frame = live_frame(ctx.style);
    let width = live_dimension(ctx.style.width, Some(ctx.cb_width), frame.0);
    let height = live_dimension(ctx.style.height, ctx.cb_height, frame.1);
    let (w, h) = match intrinsic.natural_ratio {
        Some(ratio) => ratio_base(ratio, (width, height), stretch_width(ctx), layout_scale),
        None => live_base(iw, ih, width, height),
    };
    let limits = live_limits(ctx, frame);
    let auto = width.is_none() && height.is_none();
    let (width, height) = constrain_live(w, h, width.is_none(), height.is_none(), limits);
    let (width, height) = match intrinsic.natural_ratio {
        Some(ratio) if auto => ratio_units(ratio, height, layout_scale),
        _ => (width, height),
    };
    SizedImage {
        width,
        height,
        placement: placement_of(name, intrinsic),
    }
}

pub(crate) fn size_viewport_live(
    intrinsic_width: Option<f32>,
    intrinsic_height: Option<f32>,
    ratio: Option<f32>,
    layout_scale: f32,
    ctx: &SizeCtx,
) -> (f32, f32) {
    let frame = live_frame(ctx.style);
    let width = live_dimension(ctx.style.width, Some(ctx.cb_width), frame.0)
        .map(|value| viewport_layout_unit(value, layout_scale));
    let height = live_dimension(ctx.style.height, ctx.cb_height, frame.1)
        .map(|value| viewport_layout_unit(value, layout_scale));
    let intrinsic_width = intrinsic_width.map(|value| viewport_layout_unit(value, layout_scale));
    let intrinsic_height = intrinsic_height.map(|value| viewport_layout_unit(value, layout_scale));
    let base = viewport_base(
        width,
        height,
        intrinsic_width,
        intrinsic_height,
        ratio,
        ctx.cb_width,
    );
    let limits = live_limits(ctx, frame);
    let used = if ratio.is_some() {
        constrain_live(base.0, base.1, width.is_none(), height.is_none(), limits)
    } else {
        (
            base.0.clamp(limits.min_w, limits.max_w),
            base.1.clamp(limits.min_h, limits.max_h),
        )
    };
    (
        viewport_layout_unit(used.0, layout_scale),
        viewport_layout_unit(used.1, layout_scale),
    )
}

// Blink155 LayoutUnit stores six fractional bits, saturates and truncates toward
// zero. Convert actual used dimensions before producing a native fragment; the
// caller's measured physical layout scale is separate from JavaScript DPR.
fn viewport_layout_unit(value: f32, scale: f32) -> f32 {
    ((value * scale * 64.0) as i32) as f32 / (scale * 64.0)
}

fn viewport_base(
    width: Option<f32>,
    height: Option<f32>,
    intrinsic_width: Option<f32>,
    intrinsic_height: Option<f32>,
    ratio: Option<f32>,
    containing_width: f32,
) -> (f32, f32) {
    match (width, height) {
        (Some(w), Some(h)) => (w, h),
        (Some(w), None) => (
            w,
            ratio.map_or(intrinsic_height.unwrap_or(150.0), |r| w / r),
        ),
        (None, Some(h)) => (ratio.map_or(intrinsic_width.unwrap_or(300.0), |r| h * r), h),
        (None, None) => {
            viewport_intrinsic(intrinsic_width, intrinsic_height, ratio, containing_width)
        }
    }
}

fn viewport_intrinsic(
    width: Option<f32>,
    height: Option<f32>,
    ratio: Option<f32>,
    containing_width: f32,
) -> (f32, f32) {
    match (width, height, ratio) {
        (Some(w), Some(h), _) => (w, h),
        (Some(w), None, Some(r)) => (w, w / r),
        (None, Some(h), Some(r)) => (h * r, h),
        (None, None, Some(r)) => (containing_width, containing_width / r),
        _ => (width.unwrap_or(300.0), height.unwrap_or(150.0)),
    }
}

/// The content width an auto-sized box stretches to: the containing block's width less
/// its own margins, padding and border.
fn stretch_width(ctx: &SizeCtx) -> f32 {
    let s = ctx.style;
    (ctx.cb_width - s.margin.horizontal() - s.padding.horizontal() - s.border.widths().horizontal())
        .max(0.0)
}

/// The pre-constraint content size of an image with only a natural `ratio` (CSS Images 3
/// default sizing, miaskiewicz/turbo-dom#320): a missing axis follows the given one at
/// the ratio; with neither given, the box stretches to `stretch` wide. Like Blink, the
/// derived height is a layout unit first, and a width derived from it is too (a 400px
/// wide 3:2 image is 399.984375 x 266.65625 at scale 1).
fn ratio_base(
    ratio: f32,
    (width, height): (Option<f32>, Option<f32>),
    stretch: f32,
    layout_scale: f32,
) -> (f32, f32) {
    let unit = |value: f32| viewport_layout_unit(value, layout_scale);
    match (width, height) {
        (Some(w), Some(h)) => (w, h),
        (Some(w), None) => (w, unit(w / ratio)),
        (None, Some(h)) => (unit(h * ratio), h),
        (None, None) => {
            let h = unit(stretch / ratio);
            (unit(h * ratio), h)
        }
    }
}

/// A constrained auto-sized ratio-only box in layout units, like Blink: the height
/// first, then the width derived from it (a `max-width: 286px` 3:2 image is
/// 285.984375 x 190.65625 at scale 1; miaskiewicz/turbo-dom#326).
fn ratio_units(ratio: f32, height: f32, layout_scale: f32) -> (f32, f32) {
    let unit = |value: f32| viewport_layout_unit(value, layout_scale);
    let h = unit(height);
    (unit(h * ratio), h)
}

fn live_frame(style: &BoxStyle) -> (f32, f32) {
    if style.box_sizing == super::value::BoxSizing::ContentBox {
        return (0.0, 0.0);
    }
    (
        style.padding.horizontal() + style.border.widths().horizontal(),
        style.padding.vertical() + style.border.widths().vertical(),
    )
}

fn live_dimension(value: LengthPct, basis: Option<f32>, frame: f32) -> Option<f32> {
    resolve_dim(value, basis).map(|v| (v - frame).max(0.0))
}

fn live_base(iw: f32, ih: f32, width: Option<f32>, height: Option<f32>) -> (f32, f32) {
    match (width, height) {
        (Some(w), Some(h)) => (w, h),
        (Some(w), None) => (w, scale_other_live(w, iw, ih)),
        (None, Some(h)) => (scale_other_live(h, ih, iw), h),
        (None, None) => (iw, ih),
    }
}

struct Limits {
    min_w: f32,
    max_w: f32,
    min_h: f32,
    max_h: f32,
}

fn live_limits(ctx: &SizeCtx, frame: (f32, f32)) -> Limits {
    let s = ctx.style;
    let min_w = live_dimension(s.min_width, Some(ctx.cb_width), frame.0).unwrap_or(0.0);
    let min_h = live_dimension(s.min_height, ctx.cb_height, frame.1).unwrap_or(0.0);
    Limits {
        min_w,
        min_h,
        max_w: live_dimension(s.max_width, Some(ctx.cb_width), frame.0)
            .unwrap_or(f32::INFINITY)
            .max(min_w),
        max_h: live_dimension(s.max_height, ctx.cb_height, frame.1)
            .unwrap_or(f32::INFINITY)
            .max(min_h),
    }
}

#[allow(clippy::needless_pass_by_value)] // signature kept as relocated; borrowing instead changes nothing observable
fn constrain_live(w: f32, h: f32, auto_w: bool, auto_h: bool, limits: Limits) -> (f32, f32) {
    if auto_w && auto_h {
        return constrain_ratio(w, h, &limits);
    }
    let width = w.clamp(limits.min_w, limits.max_w);
    let height = h.clamp(limits.min_h, limits.max_h);
    match (auto_w, auto_h) {
        (true, false) => (
            scale_other_live(height, h, w).clamp(limits.min_w, limits.max_w),
            height,
        ),
        (false, true) => (
            width,
            scale_other_live(width, w, h).clamp(limits.min_h, limits.max_h),
        ),
        _ => (width, height),
    }
}

// CSS 2.2 §10.4: preserve intrinsic ratio for both-auto axes where constraints
// permit it; conflicting width/height constraints can necessarily break it.
fn constrain_ratio(w: f32, h: f32, limits: &Limits) -> (f32, f32) {
    if w <= 0.0 || h <= 0.0 {
        return (
            w.clamp(limits.min_w, limits.max_w),
            h.clamp(limits.min_h, limits.max_h),
        );
    }
    let lower = (limits.min_w / w).max(limits.min_h / h);
    let upper = (limits.max_w / w).min(limits.max_h / h);
    let scale = 1.0_f32.min(upper).max(lower);
    (
        (w * scale).clamp(limits.min_w, limits.max_w),
        (h * scale).clamp(limits.min_h, limits.max_h),
    )
}

/// The intrinsic size an image box lays out with: the resolver's probe of `name`, as
/// displayed under the box's `image-orientation` (`from-image`, the default, applies the
/// image's EXIF orientation — width/height swap for orientations 5–8; `none` keeps the
/// stored pixels). Chrome 156-controlled (turbo-dom
/// `scripts/image-orientation-chrome-control.mjs`).
pub(crate) fn box_intrinsic(
    resolver: &dyn ImageResolver,
    name: &str,
    style: &ComputedStyle,
) -> Option<Intrinsic> {
    let from_image = style.get("image-orientation").map(str::trim) != Some("none");
    Some(resolver.intrinsic(name)?.displayed(from_image))
}

/// A replaced `<img>`'s intrinsic size and whether its image loaded: the resolver's,
/// else its [`ImageSource::fallback`](super::boxgen::ImageSource::fallback) (Chrome's
/// broken-image size, miaskiewicz/turbo-dom#243). `None` when neither applies (the
/// box lays out its alt text instead).
pub(crate) fn replaced_intrinsic(
    resolver: &dyn ImageResolver,
    src: &super::boxgen::ImageSource,
    style: &ComputedStyle,
) -> Option<(Intrinsic, bool)> {
    let loaded = (!src.name.is_empty())
        .then(|| box_intrinsic(resolver, &src.name, style))
        .flatten();
    if let Some(intrinsic) = loaded {
        return Some((intrinsic, true));
    }
    let (width, height) = src.fallback?;
    Some((Intrinsic::new(width, height, true), false))
}

/// The placement an intrinsic-sized image emits (the resolver name plus its
/// displayed size, alpha flag and the orientation to paint with).
#[must_use]
pub fn placement_of(name: String, intrinsic: Intrinsic) -> ImagePlacement {
    ImagePlacement {
        name,
        intrinsic_w: intrinsic.width,
        intrinsic_h: intrinsic.height,
        has_alpha: intrinsic.has_alpha,
        tint: None,
        orientation: intrinsic.orientation,
    }
}

/// The pre-cap box size: explicit `width`/`height` when set (filling the missing
/// axis from the intrinsic aspect ratio), else the intrinsic pixel size.
fn base_size(iw: f32, ih: f32, ctx: &SizeCtx) -> (f32, f32) {
    // Resolve an explicit `width`/`height` against the containing block (a `%`
    // `width` against `cb_width`, a `%` `height` against `cb_height` when a
    // definite one is known). A `%` with no basis, or `auto`, stays unresolved so
    // the intrinsic aspect ratio fills it in. Previously `%` resolved against 0,
    // collapsing every `width:100%` responsive image to a 0-size box.
    let w = resolve_dim(ctx.style.width, Some(ctx.cb_width));
    let h = resolve_dim(ctx.style.height, ctx.cb_height);
    match (w, h) {
        (Some(w), Some(h)) => (w, h),
        (Some(w), None) => (w, scale_other(w, iw, ih)),
        (None, Some(h)) => (scale_other(h, ih, iw), h),
        (None, None) => (iw, ih),
    }
}

/// A `<length-percentage>`/`auto` image dimension resolved against a basis: a px
/// length is itself, a `%` needs a (definite) basis, and `auto`/an unresolvable
/// `%` yields `None` (the caller falls back to the intrinsic aspect ratio).
fn resolve_dim(v: LengthPct, basis: Option<f32>) -> Option<f32> {
    match v {
        LengthPct::Px(px) => Some(px),
        LengthPct::Pct(p) => basis.map(|b| p / 100.0 * b),
        LengthPct::Calc { pct, px } => basis.map(|b| pct / 100.0 * b + px),
        LengthPct::Auto => None,
    }
}

/// The dependent axis when one axis is fixed at `given`: `given * (other /
/// base)`, preserving aspect ratio. Falls back to `given` for a degenerate
/// (zero) base so the result stays finite, except that a 0x0 image (an `<img>` that
/// represents nothing, miaskiewicz/turbo-dom#243) has no ratio and keeps its 0.
fn scale_other(given: f32, base: f32, other: f32) -> f32 {
    if base > 0.0 {
        given * other / base
    } else if other > 0.0 {
        given
    } else {
        0.0
    }
}

/// [`scale_other`] with Chrome 156's live semantics: a degenerate (zero) base has no
/// aspect ratio, so the dependent axis keeps `other`, its intrinsic size (a
/// `height=0` canvas sized `height: 30px` keeps its intrinsic width,
/// miaskiewicz/turbo-dom#252; a 0x0 `<img>` keeps its 0, #243).
fn scale_other_live(given: f32, base: f32, other: f32) -> f32 {
    if base > 0.0 {
        given * other / base
    } else {
        other
    }
}

// Whether the running layout sizes images without the overflow caps
// ([`crate::LayoutOptions::uncapped_images`]): installed for the entry's duration as
// a scoped thread-local, like the `cmyk(...)` switch; off everywhere else.
thread_local!(static UNCAPPED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) });

/// Restores the previous cap policy when dropped.
pub(crate) struct UncappedScope(bool);

impl Drop for UncappedScope {
    fn drop(&mut self) {
        UNCAPPED.with(|flag| flag.set(self.0));
    }
}

/// Size images without (`true`) or with the overflow caps until the guard drops.
pub(crate) fn uncapped_scope(uncapped: bool) -> UncappedScope {
    UncappedScope(UNCAPPED.with(|flag| flag.replace(uncapped)))
}

/// Clamp a box to the width and (when known) height caps, preserving aspect
/// ratio by scaling both axes by the tighter of the two fit ratios; uncapped, the box
/// keeps its CSS size.
fn apply_caps(w: f32, h: f32, ctx: &SizeCtx) -> (f32, f32) {
    if UNCAPPED.with(std::cell::Cell::get) {
        return (w, h);
    }
    let max_w = ctx.cb_width;
    let max_h = ctx.body_height.map(|bh| bh * MAX_HEIGHT_FRACTION);
    let scale = fit_scale(w, h, max_w, max_h);
    (w * scale, h * scale)
}

/// The largest uniform scale ≤ 1 that fits `(w, h)` inside the caps. A zero or
/// missing cap dimension imposes no limit on that axis.
fn fit_scale(w: f32, h: f32, max_w: f32, max_h: Option<f32>) -> f32 {
    let sw = axis_scale(w, max_w);
    let sh = max_h.map_or(1.0, |m| axis_scale(h, m));
    sw.min(sh).min(1.0)
}

/// The fit ratio for one axis: `cap / value`, or `1.0` when the value already
/// fits or the cap is non-positive (no limit).
fn axis_scale(value: f32, cap: f32) -> f32 {
    if cap > 0.0 && value > cap {
        cap / value
    } else {
        1.0
    }
}

#[cfg(test)]
mod coverage_tests {
    use super::*;

    #[test]
    fn live_degenerate_intrinsic_dimensions_stay_finite_and_honor_css_minimums() {
        use crate::layout::value::{resolve_box_style, ResolveCtx};
        use crate::ComputedStyle;
        let style = resolve_box_style(
            &ComputedStyle::from_pairs([("min-width", "12px"), ("min-height", "6px")]),
            ResolveCtx {
                parent_font_size: 16.0,
                cb_width: 200.0,
            },
        );
        let ctx = SizeCtx {
            style: &style,
            cb_width: 200.0,
            cb_height: None,
            body_height: None,
        };
        for (width, height, expected) in [
            (0, 0, (12.0, 6.0)),
            (0, 20, (12.0, 20.0)),
            (20, 0, (20.0, 6.0)),
        ] {
            let sized = size_replaced_live(
                "degenerate".into(),
                Intrinsic::new(width, height, false),
                &ctx,
                1.0,
            );
            assert_eq!((sized.width, sized.height), expected);
            assert!(sized.width.is_finite() && sized.height.is_finite());
        }
    }

    #[test]
    fn ratio_only_images_stretch_to_the_containing_block() {
        // miaskiewicz/turbo-dom#320: no natural size, so the reported 225x150 is ignored.
        use crate::layout::value::{resolve_box_style, ResolveCtx};
        use crate::ComputedStyle;
        let sized = |pairs: &[(&str, &str)], live: Option<f32>| {
            let style = resolve_box_style(
                &ComputedStyle::from_pairs(pairs.iter().copied()),
                ResolveCtx {
                    parent_font_size: 16.0,
                    cb_width: 400.0,
                },
            );
            let ctx = SizeCtx {
                style: &style,
                cb_width: 400.0,
                cb_height: None,
                body_height: None,
            };
            let intrinsic = Intrinsic::ratio_only(225, 150, 1.5, true);
            let out = match live {
                Some(scale) => size_replaced_live("svg".into(), intrinsic, &ctx, scale),
                None => size_replaced("svg".into(), intrinsic, &ctx),
            };
            assert_eq!(
                (out.placement.intrinsic_w, out.placement.intrinsic_h),
                (225, 150)
            );
            (out.width, out.height)
        };
        // Static (capped) drive: the containing block's width less the margins and frame.
        assert_eq!(sized(&[], None), (400.0, 400.0 / 1.5));
        assert_eq!(
            sized(&[("margin", "0 10px"), ("padding", "5px")], None),
            (370.0, 370.0 / 1.5)
        );
        assert_eq!(
            sized(&[("width", "120px")], None),
            (120.0, 120.0 * (400.0 / 1.5) / 400.0)
        );
        // Live drive: the height in layout units, then the width from it.
        assert_eq!(sized(&[], Some(1.0)), (399.984_38, 266.656_25));
        assert_eq!(sized(&[], Some(2.0)), (399.992_2, 266.664_06));
        assert_eq!(sized(&[("width", "50%")], Some(1.0)), (200.0, 133.328_13));
        assert_eq!(sized(&[("height", "60px")], Some(1.0)), (90.0, 60.0));
        assert_eq!(
            sized(&[("width", "30px"), ("height", "7px")], Some(1.0)),
            (30.0, 7.0)
        );
        assert_eq!(
            sized(&[("margin", "0 300px 0 200px")], Some(1.0)),
            (0.0, 0.0)
        );
    }

    #[test]
    fn resolve_dim_handles_px_percent_calc_and_auto() {
        assert_eq!(resolve_dim(LengthPct::Px(40.0), None), Some(40.0));
        assert_eq!(resolve_dim(LengthPct::Pct(50.0), Some(200.0)), Some(100.0));
        // calc(50% + 10px) against a 200px basis = 110.
        assert_eq!(
            resolve_dim(
                LengthPct::Calc {
                    pct: 50.0,
                    px: 10.0
                },
                Some(200.0)
            ),
            Some(110.0)
        );
        // A calc with no basis (indefinite CB) can't resolve → None.
        assert_eq!(
            resolve_dim(
                LengthPct::Calc {
                    pct: 50.0,
                    px: 10.0
                },
                None
            ),
            None
        );
        assert_eq!(resolve_dim(LengthPct::Auto, Some(200.0)), None);
    }
}
