//! Vector painter: walk a [`Fragment`] galley into a standalone SVG document.
//! Same stacking-order walk as the raster painter, so the two agree. Glyphs are
//! emitted as filled `<path>` outlines (self-contained — no viewer font needed),
//! boxes/borders as `<rect>`, and images as base64 PNG `data:` URI `<image>`s
//! (any decoded source format; unknown formats fall back to a placeholder rect).

use std::fmt::Write;

use turbo_dom_layout::layout::value::{BorderEdges, BorderSide};
use turbo_dom_layout::text::FontFace;
use turbo_dom_layout::{
    BoxShadow, ClipRect, Fragment, FragmentContent, LinearGradient as CssGradient, PositionedGlyph,
    Rgba,
};

use crate::glyph::{self, Pen, Tracer};
use crate::image_paint::{self, DecodedAssets};

const IMAGE_PLACEHOLDER: Rgba = Rgba {
    r: 220,
    g: 220,
    b: 220,
    a: 255,
};

/// Paint `galley` into a `width × height` SVG document string over a `bg` canvas
/// fill (the propagated root/body background).
pub fn paint(
    galley: &Fragment,
    width: u32,
    height: u32,
    bg: Rgba,
    images: &DecodedAssets,
) -> String {
    let mut svg = String::with_capacity(4096);
    let _ = writeln!(
        svg,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" \
         viewBox=\"0 0 {width} {height}\">\n<rect width=\"{width}\" height=\"{height}\" {}/>",
        fill_attrs(bg)
    );
    paint_fragment(&mut svg, galley, images);
    svg.push_str("</svg>\n");
    svg
}

fn paint_fragment(svg: &mut String, f: &Fragment, images: &DecodedAssets) {
    // A CSS `transform` wraps this box + subtree in a `<g transform>` (applied about
    // the box's absolute transform-origin), so the SVG viewer transforms them as a unit.
    let transformed = matches!(
        &f.content,
        FragmentContent::Box {
            transform: Some(_),
            ..
        }
    );
    if let FragmentContent::Box {
        transform: Some(t), ..
    } = &f.content
    {
        let (ox, oy) = (f.x + t.origin_x, f.y + t.origin_y);
        let [a, b, c, d, e, g] = t.matrix;
        let _ = write!(
            svg,
            "<g transform=\"translate({ox:.2} {oy:.2}) matrix({a} {b} {c} {d} {e} {g}) translate({:.2} {:.2})\">",
            -ox, -oy
        );
    }
    match f.clip {
        Some(clip) => paint_clipped(svg, f, clip, images),
        None => paint_subtree(svg, f, images),
    }
    if transformed {
        svg.push_str("</g>");
    }
}

/// Paint a fragment whose box has a `clip: rect()` (miaskiewicz/turbo-dom#52) inside a
/// `<g clip-path>` of that region (in the box's own, possibly transformed, user space;
/// the `clipPath` id is the document offset, unique); an empty region paints nothing.
fn paint_clipped(svg: &mut String, f: &Fragment, clip: ClipRect, images: &DecodedAssets) {
    if clip.width <= 0.0 || clip.height <= 0.0 {
        return;
    }
    let id = svg.len();
    let _ = write!(
        svg,
        "<clipPath id=\"clip{id}\"><rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\"/></clipPath><g clip-path=\"url(#clip{id})\">",
        f.x + clip.x,
        f.y + clip.y,
        clip.width,
        clip.height
    );
    paint_subtree(svg, f, images);
    svg.push_str("</g>");
}

/// Paint a fragment's own content, then its children in stacking order.
fn paint_subtree(svg: &mut String, f: &Fragment, images: &DecodedAssets) {
    match &f.content {
        FragmentContent::Box {
            background,
            border,
            border_radius,
            shadow,
            gradient,
            ..
        } => {
            if let Some(sh) = shadow.filter(|s| !s.inset && s.color.a > 0) {
                paint_box_shadow(svg, f, *border_radius, &sh);
            }
            if let Some(bg) = background {
                rect_r(svg, f.x, f.y, f.width, f.height, *bg, *border_radius);
            }
            if let Some(g) = gradient {
                paint_gradient(svg, f, *border_radius, g);
            }
            if *border_radius > 0.5 {
                paint_round_border(svg, f, border, *border_radius);
            } else {
                paint_border(svg, f, border);
            }
        }
        FragmentContent::TextLine {
            glyphs,
            face,
            font_size,
            color,
        } => paint_text(svg, f, glyphs, face, *font_size, *color),
        FragmentContent::Image(placement) => paint_image(svg, f, placement, images),
        FragmentContent::Marker(_) => {}
    }
    // Paint children back-to-front in CSS stacking order (§9.9), not raw DOM
    // order, so `position`/`z-index` boxes (menus, modals) layer correctly.
    for &i in &f.paint_order() {
        paint_fragment(svg, &f.children[i], images);
    }
}

/// Embed an image as a base64 `data:` URI `<image>` filling its layout box; fall
/// back to a neutral placeholder rect when the bytes are absent or unrecognized.
/// `preserveAspectRatio="none"` because layout already sized the box to the
/// image's aspect ratio.
fn paint_image(
    svg: &mut String,
    f: &Fragment,
    placement: &turbo_dom_layout::ImagePlacement,
    images: &DecodedAssets,
) {
    let uri = images
        .get(&placement.name)
        .and_then(|img| image_paint::data_uri(&img.oriented(placement.orientation)));
    match uri {
        Some(uri) => {
            let _ = writeln!(
                svg,
                "<image x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" \
                 preserveAspectRatio=\"none\" href=\"{uri}\"/>",
                f.x, f.y, f.width, f.height
            );
        }
        None => rect(svg, f.x, f.y, f.width, f.height, IMAGE_PLACEHOLDER),
    }
}

/// `rgba(...)`-free fill: an opaque `#rrggbb` plus a separate `fill-opacity`
/// when the color carries alpha (keeps output ASCII + widely compatible).
fn fill_attrs(c: Rgba) -> String {
    if c.a == 255 {
        format!("fill=\"#{:02x}{:02x}{:02x}\"", c.r, c.g, c.b)
    } else {
        format!(
            "fill=\"#{:02x}{:02x}{:02x}\" fill-opacity=\"{:.3}\"",
            c.r,
            c.g,
            c.b,
            f32::from(c.a) / 255.0
        )
    }
}

fn rect(svg: &mut String, x: f32, y: f32, w: f32, h: f32, c: Rgba) {
    rect_r(svg, x, y, w, h, c, 0.0);
}

/// Paint an outer `box-shadow` as a blurred rounded rect behind the box, using an
/// inline SVG `feGaussianBlur` filter (native, so any viewer renders the soft
/// edge). `blur` 0 emits an unfiltered offset rect. The filter id is derived from
/// the box geometry to stay unique within the document.
fn paint_box_shadow(svg: &mut String, f: &Fragment, border_radius: f32, sh: &BoxShadow) {
    let spread = sh.spread as f32;
    let x = f.x + sh.offset_x as f32 - spread;
    let y = f.y + sh.offset_y as f32 - spread;
    let w = f.width + 2.0 * spread;
    let h = f.height + 2.0 * spread;
    if w < 0.5 || h < 0.5 {
        return;
    }
    let radius = (border_radius + spread).max(0.0);
    if sh.blur < 1 {
        rect_r(svg, x, y, w, h, sh.color, radius);
        return;
    }
    // Pad the filter region by the blur so the feathered edge isn't clipped.
    let id = format!(
        "sh{}_{}",
        (f.x as i64) + 1_000_000,
        (f.y as i64) + 1_000_000
    );
    // Capped like Chrome's (miaskiewicz/turbo-dom#95).
    let std = crate::paint_png::shadow_blur(sh) / 2.0;
    let _ = writeln!(
        svg,
        "<filter id=\"{id}\" x=\"-50%\" y=\"-50%\" width=\"200%\" height=\"200%\">\
         <feGaussianBlur stdDeviation=\"{std:.2}\"/></filter>",
    );
    let rr = if radius > 0.5 {
        format!(" rx=\"{radius:.2}\" ry=\"{radius:.2}\"")
    } else {
        String::new()
    };
    let _ = writeln!(
        svg,
        "<rect x=\"{x:.2}\" y=\"{y:.2}\" width=\"{w:.2}\" height=\"{h:.2}\"{rr} {} filter=\"url(#{id})\"/>",
        fill_attrs(sh.color)
    );
}

/// Paint a `linear-gradient(...)` background as a native SVG `<linearGradient>`
/// (userSpaceOnUse, start/end points across the box per the CSS angle) filling the
/// box rect (rounded by `border_radius`). The gradient id is derived from the box
/// geometry to stay unique within the document.
fn paint_gradient(svg: &mut String, f: &Fragment, border_radius: f32, g: &CssGradient) {
    if f.width < 0.5 || f.height < 0.5 || g.stops.len() < 2 {
        return;
    }
    let (w, h) = (f.width, f.height);
    let theta = g.angle_deg.to_radians();
    let (dx, dy) = (theta.sin(), -theta.cos());
    let half = (w * theta.sin().abs() + h * theta.cos().abs()) / 2.0;
    let (cx, cy) = (f.x + w / 2.0, f.y + h / 2.0);
    let (x1, y1) = (cx - dx * half, cy - dy * half);
    let (x2, y2) = (cx + dx * half, cy + dy * half);
    let id = format!("g{}_{}", (f.x as i64) + 1_000_000, (f.y as i64) + 1_000_000);
    let _ = write!(
        svg,
        "<linearGradient id=\"{id}\" gradientUnits=\"userSpaceOnUse\" \
         x1=\"{x1:.2}\" y1=\"{y1:.2}\" x2=\"{x2:.2}\" y2=\"{y2:.2}\">"
    );
    for s in &g.stops {
        let _ = write!(
            svg,
            "<stop offset=\"{:.3}\" stop-color=\"#{:02x}{:02x}{:02x}\" stop-opacity=\"{:.3}\"/>",
            s.pos.clamp(0.0, 1.0),
            s.color.r,
            s.color.g,
            s.color.b,
            f32::from(s.color.a) / 255.0
        );
    }
    let rr = if border_radius > 0.5 {
        format!(" rx=\"{border_radius:.2}\" ry=\"{border_radius:.2}\"")
    } else {
        String::new()
    };
    let _ = writeln!(
        svg,
        "</linearGradient>\n<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{w:.2}\" height=\"{h:.2}\"{rr} \
         fill=\"url(#{id})\"/>",
        f.x, f.y
    );
}

/// A filled `<rect>`, rounded by `r` px (`rx`/`ry`) when `r > 0`.
fn rect_r(svg: &mut String, x: f32, y: f32, w: f32, h: f32, c: Rgba, r: f32) {
    if w <= 0.0 || h <= 0.0 || c.a == 0 {
        return;
    }
    let radius = if r > 0.5 {
        format!(" rx=\"{r:.2}\" ry=\"{r:.2}\"")
    } else {
        String::new()
    };
    let _ = writeln!(
        svg,
        "<rect x=\"{x:.2}\" y=\"{y:.2}\" width=\"{w:.2}\" height=\"{h:.2}\"{radius} {}/>",
        fill_attrs(c)
    );
}

fn paint_border(svg: &mut String, f: &Fragment, b: &BorderEdges) {
    let side = |s: &BorderSide| {
        s.color
            .filter(|_| s.width > 0)
            .map(|c| (f32::from(s.width), c))
    };
    if let Some((w, c)) = side(&b.top) {
        rect(svg, f.x, f.y, f.width, w, c);
    }
    if let Some((w, c)) = side(&b.bottom) {
        rect(svg, f.x, f.y + f.height - w, f.width, w, c);
    }
    if let Some((w, c)) = side(&b.left) {
        rect(svg, f.x, f.y, w, f.height, c);
    }
    if let Some((w, c)) = side(&b.right) {
        rect(svg, f.x + f.width - w, f.y, w, f.height, c);
    }
}

/// Stroke a rounded box's border as one rounded outline, as the raster painter does: the
/// widest side's width and colour, inset by half the stroke so the outline stays inside the
/// border box, its radius the box's less that half (miaskiewicz/turbo-dom#95).
fn paint_round_border(svg: &mut String, f: &Fragment, b: &BorderEdges, r: f32) {
    let widest = [&b.top, &b.right, &b.bottom, &b.left]
        .into_iter()
        .filter(|s| s.width > 0)
        .max_by_key(|s| s.width);
    let Some((w, c)) = widest.and_then(|s| s.color.map(|c| (f32::from(s.width), c))) else {
        return;
    };
    if c.a == 0 {
        return;
    }
    let hw = w / 2.0;
    let (iw, ih) = ((f.width - w).max(0.0), (f.height - w).max(0.0));
    let ir = (r - hw).min(iw / 2.0).min(ih / 2.0).max(0.0);
    let opacity = if c.a == 255 {
        String::new()
    } else {
        format!(" stroke-opacity=\"{:.3}\"", f32::from(c.a) / 255.0)
    };
    let _ = writeln!(
        svg,
        "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{iw:.2}\" height=\"{ih:.2}\" rx=\"{ir:.2}\" ry=\"{ir:.2}\" \
         fill=\"none\" stroke=\"#{:02x}{:02x}{:02x}\"{opacity} stroke-width=\"{w:.2}\"/>",
        f.x + hw,
        f.y + hw,
        c.r,
        c.g,
        c.b,
    );
}

/// An SVG path-data sink for [`glyph::trace_glyph`].
#[derive(Default)]
struct PathSink(String);

impl Tracer for PathSink {
    fn move_to(&mut self, x: f32, y: f32) {
        let _ = write!(self.0, "M{x:.2} {y:.2} ");
    }
    fn line_to(&mut self, x: f32, y: f32) {
        let _ = write!(self.0, "L{x:.2} {y:.2} ");
    }
    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        let _ = write!(self.0, "Q{cx:.2} {cy:.2} {x:.2} {y:.2} ");
    }
    fn cubic_to(&mut self, c1x: f32, c1y: f32, c2x: f32, c2y: f32, x: f32, y: f32) {
        let _ = write!(
            self.0,
            "C{c1x:.2} {c1y:.2} {c2x:.2} {c2y:.2} {x:.2} {y:.2} "
        );
    }
    fn close(&mut self) {
        self.0.push_str("Z ");
    }
}

/// Emit a line's glyph outlines as one filled `<path>` (a transparent colour, or a run
/// with no outlined glyph, emits nothing). The face's `units_per_em` is non-zero:
/// `ttf-parser` rejects a `head` outside 16..=16384.
fn paint_text(
    svg: &mut String,
    f: &Fragment,
    glyphs: &[PositionedGlyph],
    face: &FontFace,
    font_size: f32,
    color: Rgba,
) {
    if color.a == 0 {
        return;
    }
    let scale = font_size / f32::from(face.units_per_em());
    let face = glyph::outline_face(face);
    let mut sink = PathSink::default();
    for g in glyphs {
        let pen = Pen {
            origin_x: f.x + g.x,
            baseline_y: f.y + g.y,
            scale,
        };
        glyph::trace_glyph(&face, g.glyph_id, pen, &mut sink);
    }
    if !sink.0.is_empty() {
        let _ = writeln!(
            svg,
            "<path d=\"{}\" {}/>",
            sink.0.trim_end(),
            fill_attrs(color)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use turbo_dom_layout::image::{Intrinsic, Orientation};
    use turbo_dom_layout::{
        FontRegistry, GradientStop, ImagePlacement, MarkerKind, NodeId, PositionedGlyph,
    };

    use crate::image_paint::DecodedImage;

    const RED: Rgba = Rgba {
        r: 255,
        g: 0,
        b: 0,
        a: 255,
    };
    const WHITE: Rgba = Rgba {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };

    fn plain(x: f32, y: f32, w: f32, h: f32, background: Option<Rgba>) -> Fragment {
        Fragment::new(
            NodeId(0),
            x,
            y,
            w,
            h,
            FragmentContent::Box {
                background,
                border: BorderEdges::default(),
                border_radius: 0.0,
                shadow: None,
                gradient: None,
                transform: None,
            },
        )
    }

    /// The document body between the canvas background rect and `</svg>`.
    fn body(galley: &Fragment, images: &DecodedAssets) -> String {
        let svg = paint(galley, 10, 10, WHITE, images);
        let start = svg.find("/>\n").unwrap() + 3;
        svg[start..svg.len() - "</svg>\n".len()].to_string()
    }

    #[test]
    fn the_document_is_sized_and_filled_with_the_canvas_background() {
        let svg = paint(
            &plain(0.0, 0.0, 1.0, 1.0, None),
            30,
            20,
            Rgba::new(0, 0, 255, 128),
            &HashMap::new(),
        );
        assert!(svg.starts_with(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="30" height="20" viewBox="0 0 30 20">
<rect width="30" height="20" fill="#0000ff" fill-opacity="0.502"/>"##
        ));
        assert!(svg.ends_with("</svg>\n"));
    }

    #[test]
    fn a_marker_emits_nothing_but_its_children_do() {
        let mut marker = Fragment::new(
            NodeId(0),
            0.0,
            0.0,
            10.0,
            10.0,
            FragmentContent::Marker(MarkerKind {
                id: 1,
                inline: true,
            }),
        );
        marker.children.push(plain(5.0, 0.0, 5.0, 10.0, Some(RED)));
        assert_eq!(
            body(&marker, &HashMap::new()),
            "<rect x=\"5.00\" y=\"0.00\" width=\"5.00\" height=\"10.00\" fill=\"#ff0000\"/>\n"
        );
    }

    fn image(name: &str) -> Fragment {
        Fragment::new(
            NodeId(0),
            1.0,
            2.0,
            4.0,
            3.0,
            FragmentContent::Image(ImagePlacement {
                name: name.to_string(),
                intrinsic_w: 2,
                intrinsic_h: 1,
                has_alpha: true,
                tint: None,
                orientation: Orientation::IDENTITY,
            }),
        )
    }

    #[test]
    fn images_embed_as_png_data_uris_else_a_placeholder_rect() {
        let mut images = HashMap::new();
        images.insert(
            "red.png".to_string(),
            DecodedImage {
                width: 2,
                height: 1,
                rgba: vec![255, 0, 0, 255, 255, 0, 0, 255],
                intrinsic: Intrinsic::new(2, 1, true),
            },
        );
        let embedded = body(&image("red.png"), &images);
        assert!(
            embedded.starts_with(
                r#"<image x="1.00" y="2.00" width="4.00" height="3.00" preserveAspectRatio="none" href="data:image/png;base64,iVBORw0KGgo"#
            ),
            "{embedded}"
        );
        // Absent bytes, or pixels that cannot form an image (a size-only entry), are the
        // placeholder.
        images.insert(
            "size-only".to_string(),
            DecodedImage {
                width: 0,
                height: 0,
                rgba: Vec::new(),
                intrinsic: Intrinsic::new(2, 1, true),
            },
        );
        let placeholder =
            "<rect x=\"1.00\" y=\"2.00\" width=\"4.00\" height=\"3.00\" fill=\"#dcdcdc\"/>\n";
        assert_eq!(body(&image("absent.png"), &images), placeholder);
        assert_eq!(body(&image("size-only"), &images), placeholder);
    }

    #[test]
    fn empty_boxes_shadows_and_gradients_emit_nothing() {
        let mut f = plain(0.0, 0.0, 0.0, 10.0, Some(RED));
        if let FragmentContent::Box {
            shadow, gradient, ..
        } = &mut f.content
        {
            *shadow = Some(BoxShadow {
                offset_x: 2,
                offset_y: 2,
                blur: 0,
                spread: -1,
                color: RED,
                inset: false,
            });
            *gradient = Some(CssGradient {
                angle_deg: 90.0,
                stops: vec![
                    GradientStop {
                        color: RED,
                        pos: 0.0,
                    },
                    GradientStop {
                        color: WHITE,
                        pos: 1.0,
                    },
                ],
            });
        }
        assert_eq!(body(&f, &HashMap::new()), "");
        // One stop is no gradient line either.
        let mut one = plain(0.0, 0.0, 10.0, 10.0, None);
        if let FragmentContent::Box { gradient, .. } = &mut one.content {
            *gradient = Some(CssGradient {
                angle_deg: 0.0,
                stops: vec![GradientStop {
                    color: RED,
                    pos: 0.0,
                }],
            });
        }
        assert_eq!(body(&one, &HashMap::new()), "");
    }

    /// A rounded box with `border` sides on a 20x10 box at (1, 2) and the given radius.
    fn bordered(sides: [BorderSide; 4], radius: f32) -> Fragment {
        let mut f = plain(1.0, 2.0, 20.0, 10.0, None);
        if let FragmentContent::Box {
            border,
            border_radius,
            ..
        } = &mut f.content
        {
            let [top, right, bottom, left] = sides;
            *border = BorderEdges {
                top,
                right,
                bottom,
                left,
            };
            *border_radius = radius;
        }
        f
    }

    #[test]
    fn rounded_borders_stroke_one_inset_outline_of_the_widest_side() {
        let side = |width: u16, color: Option<Rgba>| BorderSide { width, color };
        let blue = Some(Rgba::new(0, 0, 255, 255));
        let thin = side(1, Some(RED));
        // The widest side (4px blue) strokes the whole outline, inset by 2px; the radius
        // shrinks by the same half width.
        assert_eq!(
            body(
                &bordered([thin, side(4, blue), thin, side(0, blue)], 6.0),
                &HashMap::new()
            ),
            "<rect x=\"3.00\" y=\"4.00\" width=\"16.00\" height=\"6.00\" rx=\"3.00\" ry=\"3.00\" \
             fill=\"none\" stroke=\"#0000ff\" stroke-width=\"4.00\"/>\n"
        );
        // A radius larger than the inset box is clamped to half its shorter side; a
        // translucent colour carries a stroke opacity.
        let half = Some(Rgba::new(255, 0, 0, 128));
        assert_eq!(
            body(&bordered([side(2, half); 4], 40.0), &HashMap::new()),
            "<rect x=\"2.00\" y=\"3.00\" width=\"18.00\" height=\"8.00\" rx=\"4.00\" ry=\"4.00\" \
             fill=\"none\" stroke=\"#ff0000\" stroke-opacity=\"0.502\" stroke-width=\"2.00\"/>\n"
        );
        // A border wider than the box strokes a zero-size inset rect.
        assert!(
            body(&bordered([side(30, blue); 4], 6.0), &HashMap::new()).starts_with(
                "<rect x=\"16.00\" y=\"17.00\" width=\"0.00\" height=\"0.00\" rx=\"0.00\""
            ),
        );
        // No visible side, a colourless widest side, or a transparent one: nothing.
        let none = side(0, blue);
        assert_eq!(body(&bordered([none; 4], 6.0), &HashMap::new()), "");
        assert_eq!(
            body(
                &bordered([side(3, None), thin, thin, thin], 6.0),
                &HashMap::new()
            ),
            ""
        );
        let clear = side(3, Some(Rgba::new(0, 0, 255, 0)));
        assert_eq!(body(&bordered([clear; 4], 6.0), &HashMap::new()), "");
        // At a radius of 0.5 or less the four square edge bands paint instead.
        assert_eq!(
            body(
                &bordered([side(2, blue), none, none, none], 0.5),
                &HashMap::new()
            ),
            "<rect x=\"1.00\" y=\"2.00\" width=\"20.00\" height=\"2.00\" fill=\"#0000ff\"/>\n"
        );
    }

    #[test]
    fn text_emits_one_outline_path_unless_nothing_has_an_outline() {
        let face = FontRegistry::new()
            .select(&["serif"], 400, false)
            .unwrap()
            .clone();
        let line = |text: &str, color: Rgba| {
            let glyphs = face
                .shape(text)
                .iter()
                .scan(0.0, |pen, g| {
                    let x = *pen;
                    *pen += g.x_advance as f32 * 20.0 / f32::from(face.units_per_em());
                    Some(PositionedGlyph {
                        glyph_id: g.glyph_id,
                        x,
                        y: 15.0,
                    })
                })
                .collect();
            Fragment::new(
                NodeId(0),
                0.0,
                0.0,
                10.0,
                20.0,
                FragmentContent::TextLine {
                    glyphs,
                    face: face.clone(),
                    font_size: 20.0,
                    color,
                },
            )
        };
        let ink = body(&line("Ag", RED), &HashMap::new());
        assert!(ink.starts_with("<path d=\"M"), "{ink}");
        // A TrueType (quadratic) face traces `Q` segments.
        assert!(
            ink.contains(" Q") || ink.starts_with("<path d=\"Q"),
            "{ink}"
        );
        assert!(ink.ends_with(" fill=\"#ff0000\"/>\n"), "{ink}");
        assert_eq!(
            body(&line(" ", RED), &HashMap::new()),
            "",
            "a space has no outline"
        );
        assert_eq!(
            body(&line("Ag", Rgba::new(255, 0, 0, 0)), &HashMap::new()),
            ""
        );
    }
}
