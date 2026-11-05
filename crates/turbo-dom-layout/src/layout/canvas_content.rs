//! A `<canvas>` box's drawn content (miaskiewicz/turbo-dom#291).
//!
//! Layout has no canvas bitmap of its own: an embedder that has one (turbo-dom-raster's
//! screenshots, from the page's op log or pixels) answers the canvas's
//! [`canvas_image_name`] through its [`ImageResolver`](crate::ImageResolver), and the box
//! then carries the bitmap as an [`Image`](super::fragment::FragmentContent::Image) child
//! placed in its content box like Chrome 156 paints a canvas: scaled per `object-fit`,
//! placed per `object-position`, clipped to the content box and its inner corner radii.
//! The box itself (its size, border, padding and background) is the same with or without
//! content.
use super::fragment::{Fragment, FragmentContent, ImagePlacement, OverflowClip};
use super::value::BoxStyle;
use super::writing::{layers_to_logical, physical_in, WritingMode};
use turbo_dom::Handle;

/// The [`ImageResolver`](crate::ImageResolver) name a `<canvas>` element's box loads its
/// drawn content by. An embedder resolves it to the canvas bitmap (sized like the canvas:
/// [`canvas_bitmap_size`]); unresolved, the canvas is blank. The name starts with U+0000,
/// which no HTML attribute value or CSS `url()` holds (the parsers replace it), so it never
/// names an `<img>` or background image.
#[must_use]
pub fn canvas_image_name(element: Handle) -> String {
    format!("\0canvas:{}", element.raw())
}

/// A canvas's bitmap size from its `width` / `height` attributes (HTML's rules: a valid
/// non-negative integer, else 300 x 150): the size its 2D context draws in and, like
/// Chrome's, its natural size.
#[must_use]
pub fn canvas_bitmap_size(width: Option<&str>, height: Option<&str>) -> (u32, u32) {
    (dimension(width, 300), dimension(height, 150))
}

/// A canvas `width` / `height` attribute: a valid non-negative integer (leading
/// whitespace, an optional `+`, the leading digits), else `default`.
fn dimension(value: Option<&str>, default: u32) -> u32 {
    let value = value
        .unwrap_or_default()
        .trim_start_matches([' ', '\t', '\n', '\x0c', '\r']);
    let value = value.strip_prefix('+').unwrap_or(value);
    let digits = value.len() - value.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    value[..digits].parse().unwrap_or(default)
}

/// Put a canvas's loaded content (`placement`, its physical intrinsic size) in its box
/// `frag` (laid out in the space `space`): the bitmap fitted into the content box, which
/// clips it.
pub(crate) fn attach(
    frag: &mut Fragment,
    style: &crate::ComputedStyle,
    bs: &BoxStyle,
    placement: ImagePlacement,
    space: WritingMode,
) {
    let border = bs.border.widths();
    let (left, top) = (border.left + bs.padding.left, border.top + bs.padding.top);
    let right = border.right + bs.padding.right;
    let bottom = border.bottom + bs.padding.bottom;
    let content = (
        frag.x + left,
        frag.y + top,
        (frag.width - left - right).max(0.0),
        (frag.height - top - bottom).max(0.0),
    );
    let outer = (frag.x, frag.y, frag.width, frag.height);
    // `object-fit` / `object-position` are physical: fit in the physical box, then map back.
    let physical = if space.vertical() {
        physical_in(space, outer, content)
    } else {
        content
    };
    let natural = (placement.intrinsic_w as f32, placement.intrinsic_h as f32);
    let fitted = fit(style, bs.font_size, physical, natural);
    let mut image = Fragment::new(
        frag.node_id,
        fitted.0,
        fitted.1,
        fitted.2,
        fitted.3,
        FragmentContent::Image(placement),
    );
    image.break_meta = frag.break_meta.clone();
    let mut images = vec![image];
    if space.vertical() {
        layers_to_logical(space, outer, &mut images);
    }
    frag.children.extend(images);
    let radii = super::value::resolve_radii(&bs.border_radius, frag.width, frag.height)
        .inset(top, right, bottom, left);
    frag.overflow_clip = Some(OverflowClip {
        x: left,
        y: top,
        width: content.2,
        height: content.3,
        radii,
    });
}

/// The rectangle a `natural`-sized object takes in the box `area` under the element's
/// `object-fit` (CSS Images 3 §5.5; `fill` when unset or unknown) and `object-position`
/// (§5.6; `50% 50%`).
fn fit(
    style: &crate::ComputedStyle,
    font_size: f32,
    (x, y, w, h): (f32, f32, f32, f32),
    (iw, ih): (f32, f32),
) -> (f32, f32, f32, f32) {
    let contain = (w / iw).min(h / ih);
    let scale = match style.get("object-fit").map(str::trim) {
        Some("contain") => Some(contain),
        Some("cover") => Some((w / iw).max(h / ih)),
        Some("none") => Some(1.0),
        Some("scale-down") => Some(contain.min(1.0)),
        _ => None,
    };
    let (ow, oh) = scale.map_or((w, h), |s| (iw * s, ih * s));
    let (dx, dy) = super::background::object_position(
        style.get("object-position"),
        font_size,
        (w, h),
        (ow, oh),
    );
    (x + dx, y + dy, ow, oh)
}

#[cfg(test)]
mod tests {
    use super::{canvas_bitmap_size, canvas_image_name, fit};
    use crate::ComputedStyle;
    use turbo_dom::Handle;

    #[test]
    fn a_canvas_names_its_content_by_element() {
        assert_eq!(canvas_image_name(Handle::from_raw(7)), "\0canvas:7");
        assert_ne!(
            canvas_image_name(Handle::from_raw(7)),
            canvas_image_name(Handle::from_raw(8))
        );
    }

    #[test]
    fn the_bitmap_size_follows_the_attributes_html_rules() {
        assert_eq!(canvas_bitmap_size(None, None), (300, 150));
        assert_eq!(canvas_bitmap_size(Some(" +64px"), Some("40.9")), (64, 40));
        assert_eq!(canvas_bitmap_size(Some("-1"), Some("x")), (300, 150));
        assert_eq!(canvas_bitmap_size(Some("0"), Some("\t\n2")), (0, 2));
    }

    fn style(pairs: &[(&str, &str)]) -> ComputedStyle {
        let mut style = ComputedStyle::default();
        for (property, value) in pairs {
            style.set(property, (*value).to_string());
        }
        style
    }

    #[test]
    fn object_fit_scales_and_object_position_places() {
        let area = (10.0, 20.0, 128.0, 128.0);
        let fitted = |pairs: &[(&str, &str)]| fit(&style(pairs), 16.0, area, (64.0, 40.0));
        assert_eq!(fitted(&[]), area);
        assert_eq!(fitted(&[("object-fit", "fill")]), area);
        assert_eq!(fitted(&[("object-fit", "bogus")]), area);
        assert_eq!(
            fitted(&[("object-fit", "contain")]),
            (10.0, 44.0, 128.0, 80.0)
        );
        let cover = fit(&style(&[("object-fit", "cover")]), 16.0, area, (64.0, 32.0));
        assert_eq!(cover, (-54.0, 20.0, 256.0, 128.0));
        assert_eq!(fitted(&[("object-fit", "none")]), (42.0, 64.0, 64.0, 40.0));
        assert_eq!(
            fitted(&[("object-fit", "scale-down")]),
            (42.0, 64.0, 64.0, 40.0)
        );
        assert_eq!(
            fitted(&[("object-fit", "none"), ("object-position", "right 4px top")]),
            (70.0, 20.0, 64.0, 40.0)
        );
        assert_eq!(
            fitted(&[("object-fit", "none"), ("object-position", "1em 50%")]),
            (26.0, 64.0, 64.0, 40.0)
        );
        // `scale-down` of a larger object is `contain`.
        let large = fit(
            &style(&[("object-fit", "scale-down")]),
            16.0,
            (0.0, 0.0, 32.0, 32.0),
            (64.0, 40.0),
        );
        assert_eq!(large, (0.0, 6.0, 32.0, 20.0));
    }
}
