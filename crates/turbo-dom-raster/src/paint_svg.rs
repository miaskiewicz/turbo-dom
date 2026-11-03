//! Vector painter: walk a [`Fragment`] galley into a standalone SVG document.
//! Same stacking-order walk as the raster painter, so the two agree. Glyphs are
//! emitted as filled `<path>` outlines (self-contained — no viewer font needed),
//! boxes/borders as `<rect>`, and images as base64 PNG `data:` URI `<image>`s
//! (any decoded source format; unknown formats fall back to a placeholder rect).

use std::fmt::Write;

use turbo_dom_layout::layout::value::{BorderEdges, BorderSide};
use turbo_dom_layout::text::FontFace;
use turbo_dom_layout::{
    BorderRadii, BoxShadow, ClipRect, Fragment, FragmentContent, LinearGradient as CssGradient,
    PositionedGlyph, Rgba,
};

use crate::glyph::{self, Pen, Tracer};
use crate::image_paint::{self, DecodedAssets};
use crate::rrect::{self, Seg};

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
    SHARED.with(|memo| memo.borrow_mut().take());
    paint_fragment(&mut svg, galley, images);
    svg.push_str("</svg>\n");
    svg
}

fn paint_fragment(svg: &mut String, f: &Fragment, images: &DecodedAssets) {
    // CSS `opacity` (miaskiewicz/turbo-dom#206) is a group opacity over the box and
    // its subtree; `opacity: 0` paints nothing.
    if f.opacity < 1.0 {
        if f.opacity > 0.0 {
            let _ = write!(svg, "<g opacity=\"{}\">", f.opacity);
            paint_opaque(svg, f, images);
            svg.push_str("</g>");
        }
        return;
    }
    paint_opaque(svg, f, images);
}

/// [`paint_fragment`] for a box painted at full opacity.
fn paint_opaque(svg: &mut String, f: &Fragment, images: &DecodedAssets) {
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
    let _ = write!(svg, "<clipPath id=\"clip{id}\">");
    let region = (f.x + clip.x, f.y + clip.y, clip.width, clip.height);
    shape(svg, region, &clip.radii, "");
    let _ = write!(svg, "</clipPath><g clip-path=\"url(#clip{id})\">");
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
                paint_box_shadow(svg, f, border_radius, &sh);
            }
            if let Some(bg) = background {
                rect_r(svg, (f.x, f.y, f.width, f.height), *bg, border_radius);
            }
            if let Some(g) = gradient {
                paint_gradient(svg, f, border_radius, g);
            }
            paint_inset_shadow(svg, f, border, border_radius, shadow.as_ref());
            if border_radius.is_rounded() {
                paint_round_border(svg, f, border, border_radius);
            } else {
                paint_border(svg, f, border);
            }
        }
        FragmentContent::TextLine {
            glyphs,
            face,
            font_size,
            color,
            vertical,
            upright,
            ..
        } => {
            if *vertical && *upright {
                paint_upright_text(svg, f, glyphs, face, (*font_size, *color));
            } else {
                paint_text(svg, (f, *vertical), glyphs, face, *font_size, *color);
            }
        }
        FragmentContent::Image(placement) => paint_image(svg, f, placement, images),
        FragmentContent::Marker(_) => {}
    }
    paint_children(svg, f, images);
}

/// Paint `f`'s children in stacking order inside a `<g clip-path>` of its `overflow`
/// region (miaskiewicz/turbo-dom#253) when it has one and they may paint past it (an empty
/// region paints none of them).
fn paint_children(svg: &mut String, f: &Fragment, images: &DecodedAssets) {
    let clip = f.overflow_clip.filter(|c| {
        !f.children.is_empty()
            && (c.radii.is_rounded()
                || !f
                    .children
                    .iter()
                    .all(|k| crate::paint_png::within(k, crate::paint_png::overflow_region(f, c))))
    });
    if let Some(c) = clip {
        if c.width <= 0.0 || c.height <= 0.0 {
            return;
        }
        let id = svg.len();
        let _ = write!(svg, "<clipPath id=\"overflow{id}\">");
        shape(svg, crate::paint_png::overflow_region(f, &c), &c.radii, "");
        let _ = write!(svg, "</clipPath><g clip-path=\"url(#overflow{id})\">");
    }
    // Paint children back-to-front in CSS stacking order (§9.9), not raw DOM
    // order, so `position`/`z-index` boxes (menus, modals) layer correctly.
    for &i in &f.paint_order() {
        paint_fragment(svg, &f.children[i], images);
    }
    if clip.is_some() {
        svg.push_str("</g>");
    }
}

/// SVG path data for outline steps.
fn path_data(segs: &[Seg]) -> String {
    let mut d = String::new();
    for seg in segs {
        let _ = match *seg {
            Seg::Move(x, y) => write!(d, "M{x:.2} {y:.2} "),
            Seg::Line(x, y) => write!(d, "L{x:.2} {y:.2} "),
            Seg::Cubic(a, b, c, e, x, y) => {
                write!(d, "C{a:.2} {b:.2} {c:.2} {e:.2} {x:.2} {y:.2} ")
            }
            Seg::Close => write!(d, "Z "),
        };
    }
    d.truncate(d.trim_end().len());
    d
}

/// Emit `rect` with corner radii `radii` and the extra attributes `attrs`: a `<rect>`
/// (with `rx`/`ry` for one circular radius over half a pixel) or, for different or
/// elliptical corners, a `<path>` of its outline (miaskiewicz/turbo-dom#253).
fn shape(svg: &mut String, (x, y, w, h): rrect::Rect, radii: &BorderRadii, attrs: &str) {
    let sep = if attrs.is_empty() { "" } else { " " };
    let Some(r) = radii.as_uniform() else {
        let d = path_data(&rrect::outline((x, y, w, h), radii));
        let _ = write!(svg, "<path d=\"{d}\"{sep}{attrs}/>");
        return;
    };
    let rr = if r > 0.5 {
        format!(" rx=\"{r:.2}\" ry=\"{r:.2}\"")
    } else {
        String::new()
    };
    let _ = write!(
        svg,
        "<rect x=\"{x:.2}\" y=\"{y:.2}\" width=\"{w:.2}\" height=\"{h:.2}\"{rr}{sep}{attrs}/>"
    );
}

/// Embed an image as a base64 `data:` URI `<image>` filling its layout box. An image
/// laid out at its size whose pixels did not decode paints nothing, as in Chrome
/// (miaskiewicz/turbo-dom#107); a neutral placeholder rect stands for an image the caller
/// did not supply or this build cannot decode.
/// `preserveAspectRatio="none"` because layout already sized the box to the
/// image's aspect ratio.
fn paint_image(
    svg: &mut String,
    f: &Fragment,
    placement: &turbo_dom_layout::ImagePlacement,
    images: &DecodedAssets,
) {
    let decoded = images.get(&placement.name);
    let key = decoded.map(|img| {
        let size = (f.width.to_bits(), f.height.to_bits());
        (
            std::ptr::from_ref(img) as usize,
            placement.orientation,
            size,
        )
    });
    // The next tile of the same image at the same size reuses the first one's `<image>`
    // through a `<use>` (miaskiewicz/turbo-dom#248: a background layer draws one image
    // many times); the id is given to that `<image>` only once it is reused.
    let reused = SHARED.with(|memo| {
        let mut memo = memo.borrow_mut();
        let shared = memo.as_mut().filter(|shared| Some(shared.key) == key)?;
        if !shared.named {
            svg.insert_str(
                shared.at + "<image".len(),
                &format!(" id=\"img{}\"", shared.at),
            );
            shared.named = true;
        }
        Some((shared.at, f.x - shared.x, f.y - shared.y))
    });
    if let Some((id, dx, dy)) = reused {
        let _ = writeln!(svg, "<use href=\"#img{id}\" x=\"{dx:.2}\" y=\"{dy:.2}\"/>");
        return;
    }
    let uri = decoded.and_then(|img| image_paint::data_uri(&img.oriented(placement.orientation)));
    match (uri, key) {
        (Some(uri), Some(key)) => {
            let at = svg.len();
            let _ = writeln!(
                svg,
                "<image x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" \
                 preserveAspectRatio=\"none\" href=\"{uri}\"/>",
                f.x, f.y, f.width, f.height
            );
            let shared = Shared {
                key,
                at,
                x: f.x,
                y: f.y,
                named: false,
            };
            SHARED.with(|memo| *memo.borrow_mut() = Some(shared));
        }
        _ if decoded.is_some_and(|d| !d.placeholder) => {}
        _ => rect(svg, f.x, f.y, f.width, f.height, IMAGE_PLACEHOLDER),
    }
}

/// The last `<image>` painted: its key, offset in the document (its id once reused)
/// and position.
struct Shared {
    key: (usize, turbo_dom_layout::image::Orientation, (u32, u32)),
    at: usize,
    x: f32,
    y: f32,
    named: bool,
}
thread_local! {
    static SHARED: std::cell::RefCell<Option<Shared>> = const { std::cell::RefCell::new(None) };
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
    rect_r(svg, (x, y, w, h), c, &BorderRadii::ZERO);
}

/// Paint an outer `box-shadow` as a blurred rounded rect behind the box, using an
/// inline SVG `feGaussianBlur` filter (native, so any viewer renders the soft
/// edge). `blur` 0 emits an unfiltered offset rect. The filter id is derived from
/// the box geometry to stay unique within the document.
fn paint_box_shadow(svg: &mut String, f: &Fragment, border_radius: &BorderRadii, sh: &BoxShadow) {
    let spread = sh.spread as f32;
    let x = f.x + sh.offset_x as f32 - spread;
    let y = f.y + sh.offset_y as f32 - spread;
    let w = f.width + 2.0 * spread;
    let h = f.height + 2.0 * spread;
    if w < 0.5 || h < 0.5 {
        return;
    }
    let radii = rrect::shadow_radii(border_radius, spread);
    if sh.blur < 1 {
        rect_r(svg, (x, y, w, h), sh.color, &radii);
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
    let attrs = format!("{} filter=\"url(#{id})\"", fill_attrs(sh.color));
    shape(svg, (x, y, w, h), &radii, &attrs);
    svg.push('\n');
}

/// Paint an inset `box-shadow` (miaskiewicz/turbo-dom#192) as the padding box in the
/// shadow colour through a `<mask>` that cuts out the hole: the padding box offset, shrunk
/// by the spread (radius with it) and blurred (`feGaussianBlur` inside the mask). The mask
/// id is the document offset, unique.
fn paint_inset_shadow(
    svg: &mut String,
    f: &Fragment,
    border: &BorderEdges,
    border_radius: &BorderRadii,
    shadow: Option<&BoxShadow>,
) {
    let Some(sh) = shadow.filter(|s| s.inset && s.color.a > 0) else {
        return;
    };
    let ((px, py, pw, ph), pr) = rrect::padding_box(f, border, border_radius);
    if pw <= 0.0 || ph <= 0.0 {
        return;
    }
    let spread = sh.spread as f32;
    let (hx, hy) = (
        px + sh.offset_x as f32 + spread,
        py + sh.offset_y as f32 + spread,
    );
    let (hw, hh) = (pw - 2.0 * spread, ph - 2.0 * spread);
    let id = svg.len();
    let _ = write!(
        svg,
        "<mask id=\"inset{id}\" maskUnits=\"userSpaceOnUse\" x=\"{px:.2}\" y=\"{py:.2}\" width=\"{pw:.2}\" height=\"{ph:.2}\">\
         <rect x=\"{px:.2}\" y=\"{py:.2}\" width=\"{pw:.2}\" height=\"{ph:.2}\" fill=\"#ffffff\"/>"
    );
    if hw >= 0.5 && hh >= 0.5 {
        let blur = crate::paint_png::shadow_blur(sh);
        let filter = if blur < 0.5 {
            String::new()
        } else {
            let _ = write!(
                svg,
                "<filter id=\"insetblur{id}\" filterUnits=\"userSpaceOnUse\" x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\">\
                 <feGaussianBlur stdDeviation=\"{:.2}\"/></filter>",
                hx - 2.0 * blur,
                hy - 2.0 * blur,
                hw + 4.0 * blur,
                hh + 4.0 * blur,
                blur / 2.0
            );
            format!(" filter=\"url(#insetblur{id})\"")
        };
        let hole = pr.inset(spread, spread, spread, spread).fit(hw, hh);
        shape(
            svg,
            (hx, hy, hw, hh),
            &hole,
            &format!("fill=\"#000000\"{filter}"),
        );
    }
    let _ = write!(svg, "</mask><g mask=\"url(#inset{id})\">");
    rect_r(svg, (px, py, pw, ph), sh.color, &pr);
    svg.push_str("</g>\n");
}

/// Paint a `linear-gradient(...)` background as a native SVG `<linearGradient>`
/// (userSpaceOnUse, start/end points across the box per the CSS angle) filling the
/// box rect (rounded by `border_radius`). The gradient id is derived from the box
/// geometry to stay unique within the document.
fn paint_gradient(svg: &mut String, f: &Fragment, border_radius: &BorderRadii, g: &CssGradient) {
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
    svg.push_str("</linearGradient>\n");
    let radii = if border_radius.is_rounded() {
        *border_radius
    } else {
        BorderRadii::ZERO
    };
    shape(
        svg,
        (f.x, f.y, w, h),
        &radii,
        &format!("fill=\"url(#{id})\""),
    );
    svg.push('\n');
}

/// A filled box `rect` with corner radii `radii` (square unless rounded).
fn rect_r(svg: &mut String, rect: rrect::Rect, c: Rgba, radii: &BorderRadii) {
    if rect.2 <= 0.0 || rect.3 <= 0.0 || c.a == 0 {
        return;
    }
    let radii = if radii.is_rounded() {
        *radii
    } else {
        BorderRadii::ZERO
    };
    shape(svg, rect, &radii, &fill_attrs(c));
    svg.push('\n');
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

/// Paint a rounded box's border as the raster painter does (miaskiewicz/turbo-dom#253):
/// the even-odd ring between the border-box and padding-box outlines, in one `<path>`
/// when every painted side has one colour, else each side's part of it through a
/// `<clipPath>` of its region ([`rrect::side_region`]). Clip ids are document offsets.
fn paint_round_border(svg: &mut String, f: &Fragment, b: &BorderEdges, radii: &BorderRadii) {
    let rect = (f.x, f.y, f.width, f.height);
    let (inner, inner_radii) = rrect::padding_box(f, b, radii);
    let mut d = path_data(&rrect::outline(rect, radii));
    if inner.2 > 0.0 && inner.3 > 0.0 {
        d.push(' ');
        d.push_str(&path_data(&rrect::outline(inner, &inner_radii)));
    }
    let sides = [&b.top, &b.right, &b.bottom, &b.left];
    let painted: Vec<(usize, Rgba)> = sides
        .iter()
        .enumerate()
        .filter(|(_, s)| s.width > 0)
        .filter_map(|(i, s)| s.color.filter(|c| c.a > 0).map(|c| (i, c)))
        .collect();
    let widths = rrect::widths(b);
    let all = widths.iter().filter(|w| **w > 0.0).count();
    if painted.len() == all && painted.windows(2).all(|p| p[0].1 == p[1].1) {
        if let Some(&(_, c)) = painted.first() {
            let _ = writeln!(
                svg,
                "<path d=\"{d}\" fill-rule=\"evenodd\" {}/>",
                fill_attrs(c)
            );
        }
        return;
    }
    for (side, c) in painted {
        let region = path_data(&rrect::polygon(&rrect::side_region(rect, widths, side)));
        let id = svg.len();
        let _ = writeln!(
            svg,
            "<clipPath id=\"side{id}\"><path d=\"{region}\"/></clipPath>\
             <path d=\"{d}\" fill-rule=\"evenodd\" clip-path=\"url(#side{id})\" {}/>",
            fill_attrs(c)
        );
    }
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
/// A turned (vertical) run of upright glyphs: each glyph its own path, turned back a
/// quarter about its pen inside the run's turn (`paint_png::upright_turn`).
fn paint_upright_text(
    svg: &mut String,
    f: &Fragment,
    glyphs: &[PositionedGlyph],
    face: &FontFace,
    (font_size, color): (f32, Rgba),
) {
    if color.a == 0 {
        return;
    }
    let scale = font_size / f32::from(face.units_per_em());
    let outline = glyph::outline_face(face);
    let frame = crate::paint_png::glyph_frame(f, true);
    for g in glyphs {
        let pen = Pen {
            origin_x: f.x + g.x,
            baseline_y: f.y + g.y,
            scale,
        };
        let mut sink = PathSink::default();
        glyph::trace_glyph(&outline, g.glyph_id, pen, &mut sink);
        if sink.0.is_empty() {
            continue;
        }
        let t = frame.pre_concat(crate::paint_png::upright_turn(
            true,
            (pen.origin_x, pen.baseline_y),
        ));
        let _ = writeln!(
            svg,
            "<path d=\"{}\" {} transform=\"matrix({:.4} {:.4} {:.4} {:.4} {:.2} {:.2})\"/>",
            sink.0.trim_end(),
            fill_attrs(color),
            t.sx,
            t.ky,
            t.kx,
            t.sy,
            t.tx,
            t.ty
        );
    }
}

fn paint_text(
    svg: &mut String,
    (f, vertical): (&Fragment, bool),
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
        // A vertical line's run turns a quarter clockwise about the fragment
        // (`crate::paint_png::glyph_frame`).
        let frame = if vertical {
            format!(
                " transform=\"matrix(0 1 -1 0 {:.2} {:.2})\"",
                f.x + f.width + f.y,
                f.y - f.x
            )
        } else {
            String::new()
        };
        let _ = writeln!(
            svg,
            "<path d=\"{}\" {}{frame}/>",
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
        Transform2D,
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
                border_radius: BorderRadii::ZERO,
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
    fn images_embed_as_png_data_uris_else_nothing_or_a_placeholder_rect() {
        let mut images = HashMap::new();
        images.insert(
            "red.png".to_string(),
            DecodedImage {
                width: 2,
                height: 1,
                rgba: vec![255, 0, 0, 255, 255, 0, 0, 255],
                intrinsic: Intrinsic::new(2, 1, true),
                placeholder: false,
            },
        );
        let embedded = body(&image("red.png"), &images);
        assert!(
            embedded.starts_with(
                r#"<image x="1.00" y="2.00" width="4.00" height="3.00" preserveAspectRatio="none" href="data:image/png;base64,iVBORw0KGgo"#
            ),
            "{embedded}"
        );
        // The next tile of the same image and size reuses it through a `<use>`, naming
        // it (miaskiewicz/turbo-dom#248); another size draws its own.
        let mut tiles = image("red.png");
        let mut second = image("red.png");
        second.x = 5.0;
        let mut third = image("red.png");
        third.y = 7.0;
        let mut wider = image("red.png");
        wider.width = 8.0;
        tiles.children = vec![second, third, wider];
        let painted = body(&tiles, &images);
        assert_eq!(painted.matches("<image").count(), 2, "{painted}");
        assert_eq!(painted.matches(" id=\"img").count(), 1, "{painted}");
        assert!(painted.contains(r#"" x="4.00" y="0.00"/>"#), "{painted}");
        assert!(painted.contains(r#"" x="0.00" y="5.00"/>"#), "{painted}");
        // A size-only entry whose pixels did not decode paints nothing, as in Chrome
        // (turbo-dom#107); absent bytes, or a size-only entry this build has no decoder
        // for, are the placeholder.
        for (name, placeholder) in [("undecodable", false), ("no-decoder", true)] {
            images.insert(
                name.to_string(),
                DecodedImage {
                    width: 0,
                    height: 0,
                    rgba: Vec::new(),
                    intrinsic: Intrinsic::new(2, 1, true),
                    placeholder,
                },
            );
        }
        let placeholder =
            "<rect x=\"1.00\" y=\"2.00\" width=\"4.00\" height=\"3.00\" fill=\"#dcdcdc\"/>\n";
        assert_eq!(body(&image("absent.png"), &images), placeholder);
        assert_eq!(body(&image("no-decoder"), &images), placeholder);
        assert_eq!(body(&image("undecodable"), &images), "");
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
            *border_radius = radius.into();
        }
        f
    }

    #[test]
    fn rounded_borders_fill_the_ring_between_the_outer_and_inner_outlines() {
        let side = |width: u16, color: Option<Rgba>| BorderSide { width, color };
        let blue = Some(Rgba::new(0, 0, 255, 255));
        let thin = side(1, Some(RED));
        // One colour on every side: one even-odd ring, the outer outline (radius 40 fitted
        // to the 20x10 box: 5) and the inner one (the padding box, radius 5 less 2: 3); a
        // translucent colour carries a fill opacity (miaskiewicz/turbo-dom#253).
        let half = Some(Rgba::new(255, 0, 0, 128));
        assert_eq!(
            body(&bordered([side(2, half); 4], 40.0), &HashMap::new()),
            "<path d=\"M6.00 2.00 L16.00 2.00 C18.76 2.00 21.00 4.24 21.00 7.00 L21.00 7.00 \
             C21.00 9.76 18.76 12.00 16.00 12.00 L6.00 12.00 C3.24 12.00 1.00 9.76 1.00 7.00 \
             L1.00 7.00 C1.00 4.24 3.24 2.00 6.00 2.00 Z M6.00 4.00 L16.00 4.00 C17.66 4.00 \
             19.00 5.34 19.00 7.00 L19.00 7.00 C19.00 8.66 17.66 10.00 16.00 10.00 L6.00 10.00 \
             C4.34 10.00 3.00 8.66 3.00 7.00 L3.00 7.00 C3.00 5.34 4.34 4.00 6.00 4.00 Z\" \
             fill-rule=\"evenodd\" fill=\"#ff0000\" fill-opacity=\"0.502\"/>\n"
        );
        // Different sides: the ring once per painted side, clipped to the side's region
        // (cut at the corner diagonals), in its own colour.
        let mixed = body(
            &bordered([thin, side(4, blue), thin, side(0, blue)], 6.0),
            &HashMap::new(),
        );
        assert_eq!(mixed.matches("<clipPath").count(), 3, "{mixed}");
        let top = mixed.find("M1.00 2.00 L21.00 2.00 L1.00 7.00 Z").unwrap();
        let right = mixed.find("M21.00 2.00 L21.00 12.00 L1.00 7.00 Z").unwrap();
        let bottom = mixed.find("M21.00 12.00 L1.00 12.00 L1.00 7.00 Z").unwrap();
        assert!(top < right && right < bottom, "{mixed}");
        assert_eq!(mixed.matches("fill=\"#ff0000\"").count(), 2);
        assert_eq!(mixed.matches("fill=\"#0000ff\"").count(), 1);
        // A border wider than the box leaves no inner outline: the whole outline fills.
        let wide = body(&bordered([side(30, blue); 4], 6.0), &HashMap::new());
        assert!(
            wide.starts_with("<path d=\"M6.00 2.00 L16.00 2.00"),
            "{wide}"
        );
        assert_eq!(wide.matches(" M").count(), 0, "{wide}");
        // No visible side or only transparent ones: nothing; a colourless side is left
        // out while the others paint.
        let none = side(0, blue);
        assert_eq!(body(&bordered([none; 4], 6.0), &HashMap::new()), "");
        let clear = side(3, Some(Rgba::new(0, 0, 255, 0)));
        assert_eq!(body(&bordered([clear; 4], 6.0), &HashMap::new()), "");
        let colourless = body(
            &bordered([side(3, None), thin, thin, thin], 6.0),
            &HashMap::new(),
        );
        assert_eq!(colourless.matches("<clipPath").count(), 3, "{colourless}");
        // At a radius of 0.5 or less the four square edge bands paint instead.
        assert_eq!(
            body(
                &bordered([side(2, blue), none, none, none], 0.5),
                &HashMap::new()
            ),
            "<rect x=\"1.00\" y=\"2.00\" width=\"20.00\" height=\"2.00\" fill=\"#0000ff\"/>\n"
        );
    }

    fn decorated(
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        radius: f32,
        edit: impl FnOnce(&mut Option<BoxShadow>, &mut Option<CssGradient>, &mut Option<Transform2D>),
    ) -> Fragment {
        let mut f = plain(x, y, w, h, None);
        if let FragmentContent::Box {
            border_radius,
            shadow,
            gradient,
            transform,
            ..
        } = &mut f.content
        {
            *border_radius = radius.into();
            edit(shadow, gradient, transform);
        }
        f
    }

    fn shadow(
        offset_x: i32,
        offset_y: i32,
        blur: u32,
        spread: i32,
        color: Rgba,
        inset: bool,
    ) -> BoxShadow {
        BoxShadow {
            offset_x,
            offset_y,
            blur,
            spread,
            color,
            inset,
        }
    }

    #[test]
    fn transforms_wrap_the_box_about_its_absolute_origin() {
        let f = decorated(10.0, 20.0, 40.0, 30.0, 0.0, |_, _, t| {
            *t = Some(Transform2D {
                matrix: [0.0, 1.0, -1.0, 0.5, 5.0, 6.0],
                origin_x: 20.0,
                origin_y: 15.0,
            });
        });
        assert_eq!(
            body(&f, &HashMap::new()),
            "<g transform=\"translate(30.00 35.00) matrix(0 1 -1 0.5 5 6) translate(-30.00 -35.00)\"></g>"
        );
    }

    #[test]
    fn shadows_are_the_offset_spread_silhouette_blurred_by_a_filter() {
        let blue = Rgba::new(0, 0, 255, 255);
        let svg = |radius: f32, sh: Option<BoxShadow>| {
            body(
                &decorated(10.0, 10.0, 20.0, 10.0, radius, |s, _, _| *s = sh),
                &HashMap::new(),
            )
        };
        assert_eq!(
            svg(0.0, Some(shadow(5, 7, 0, 3, blue, false))),
            "<rect x=\"12.00\" y=\"14.00\" width=\"26.00\" height=\"16.00\" fill=\"#0000ff\"/>\n"
        );
        assert_eq!(
            svg(4.0, Some(shadow(2, 3, 6, 0, blue, false))),
            "<filter id=\"sh1000010_1000010\" x=\"-50%\" y=\"-50%\" width=\"200%\" height=\"200%\"><feGaussianBlur stdDeviation=\"3.00\"/></filter>\n\
             <rect x=\"12.00\" y=\"13.00\" width=\"20.00\" height=\"10.00\" rx=\"4.00\" ry=\"4.00\" fill=\"#0000ff\" filter=\"url(#sh1000010_1000010)\"/>\n"
        );
        // A 0.5px radius is square; the spread scales below the radius (CSS); a 400px blur
        // is capped at 256 (stdDeviation 128).
        assert!(!svg(0.5, Some(shadow(0, 0, 4, 0, blue, false))).contains("rx="));
        assert!(svg(2.0, Some(shadow(0, 0, 0, 8, blue, false))).contains("rx=\"6.62\""));
        assert!(
            svg(0.0, Some(shadow(0, 0, 400, 0, blue, false))).contains("stdDeviation=\"128.00\"")
        );
        // An inset shadow is a masked padding box (below); a transparent one emits nothing;
        // a half-pixel one still does.
        assert_eq!(svg(0.0, Some(shadow(0, 5, 0, 0, blue, true))), "<mask id=\"inset130\" maskUnits=\"userSpaceOnUse\" x=\"10.00\" y=\"10.00\" width=\"20.00\" height=\"10.00\">\
             <rect x=\"10.00\" y=\"10.00\" width=\"20.00\" height=\"10.00\" fill=\"#ffffff\"/>\
             <rect x=\"10.00\" y=\"15.00\" width=\"20.00\" height=\"10.00\" fill=\"#000000\"/></mask>\
             <g mask=\"url(#inset130)\"><rect x=\"10.00\" y=\"10.00\" width=\"20.00\" height=\"10.00\" fill=\"#0000ff\"/>\n</g>\n"
        );
        // A blurred inset hole is blurred inside the mask; its radius is the padding-box
        // radius less the spread, clamped to half the hole.
        let blurred = svg(9.0, Some(shadow(0, 0, 4, 2, blue, true)));
        assert!(
            blurred.contains(
                "<filter id=\"insetblur130\" filterUnits=\"userSpaceOnUse\" x=\"4.00\" y=\"4.00\" width=\"32.00\" height=\"22.00\"><feGaussianBlur stdDeviation=\"2.00\"/></filter>\
                 <rect x=\"12.00\" y=\"12.00\" width=\"16.00\" height=\"6.00\" rx=\"3.00\" ry=\"3.00\" fill=\"#000000\" filter=\"url(#insetblur130)\"/>"
            ),
            "{blurred}"
        );
        // A spread past half the box leaves no hole: the whole padding box is the colour.
        let full = svg(0.0, Some(shadow(0, 0, 0, 6, blue, true)));
        assert!(
            full.contains("fill=\"#ffffff\"/></mask>") && !full.contains("#000000"),
            "{full}"
        );
        // A box with no padding box (its borders fill it) paints no inset shadow.
        assert_eq!(
            body(
                &decorated(1.0, 2.0, 0.0, 0.0, 0.0, |s, _, _| {
                    *s = Some(shadow(0, 0, 0, 0, blue, true));
                }),
                &HashMap::new(),
            ),
            ""
        );
        assert_eq!(
            svg(0.0, Some(shadow(0, 5, 0, 0, Rgba::new(0, 0, 255, 0), true))),
            ""
        );
        assert_eq!(
            svg(
                0.0,
                Some(shadow(0, 5, 0, 0, Rgba::new(0, 0, 255, 0), false))
            ),
            ""
        );
        for (w, h) in [(0.5, 10.0), (10.0, 0.5)] {
            let half = body(
                &decorated(1.0, 2.0, w, h, 0.0, |s, _, _| {
                    *s = Some(shadow(0, 0, 0, 0, blue, false));
                }),
                &HashMap::new(),
            );
            assert_eq!(half, format!("<rect x=\"1.00\" y=\"2.00\" width=\"{w:.2}\" height=\"{h:.2}\" fill=\"#0000ff\"/>\n"));
        }
    }

    #[test]
    fn gradient_lines_reach_the_box_corners_along_the_css_angle() {
        let grad = |angle_deg: f32| {
            Some(CssGradient {
                angle_deg,
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
            })
        };
        for angle in [30.0_f32, 135.0, 200.0, 290.0] {
            let (x, y, w, h) = (10.0_f32, 20.0_f32, 60.0_f32, 30.0_f32);
            let out = body(
                &decorated(x, y, w, h, 0.0, |_, g, _| *g = grad(angle)),
                &HashMap::new(),
            );
            let num = |k: &str| -> f32 {
                let at = out.find(&format!(" {k}=\"")).unwrap() + k.len() + 3;
                out[at..at + out[at..].find('"').unwrap()].parse().unwrap()
            };
            let (x1, y1, x2, y2) = (num("x1"), num("y1"), num("x2"), num("y2"));
            // The line points along (sin, -cos) of the angle, through the box centre...
            let len = (x2 - x1).hypot(y2 - y1);
            let t = angle.to_radians();
            assert!(
                ((x2 - x1) / len - t.sin()).abs() < 1e-3
                    && ((y2 - y1) / len + t.cos()).abs() < 1e-3,
                "{angle}"
            );
            assert!(
                ((x1 + x2) / 2.0 - (x + w / 2.0)).abs() < 0.01
                    && ((y1 + y2) / 2.0 - (y + h / 2.0)).abs() < 0.01
            );
            // ...and spans the box exactly: its corners project onto 0 and 1.
            let proj =
                |px: f32, py: f32| ((px - x1) * (x2 - x1) + (py - y1) * (y2 - y1)) / (len * len);
            let corners = [
                proj(x, y),
                proj(x + w, y),
                proj(x, y + h),
                proj(x + w, y + h),
            ];
            let (lo, hi) = corners
                .iter()
                .fold((f32::MAX, f32::MIN), |(a, b), &c| (a.min(c), b.max(c)));
            assert!(
                lo.abs() < 1e-3 && (hi - 1.0).abs() < 1e-3,
                "{angle}: {lo} {hi}"
            );
            assert!(out.ends_with(
                "<rect x=\"10.00\" y=\"20.00\" width=\"60.00\" height=\"30.00\" fill=\"url(#g1000010_1000020)\"/>\n"
            ), "{out}");
        }
        // Half a pixel wide or high still emits the gradient; rounded boxes round it.
        for (w, h) in [(0.5, 10.0), (10.0, 0.5)] {
            assert!(body(
                &decorated(1.0, 1.0, w, h, 0.0, |_, g, _| *g = grad(90.0)),
                &HashMap::new()
            )
            .contains("<linearGradient"));
        }
        let round = body(
            &decorated(0.0, 0.0, 40.0, 40.0, 6.0, |_, g, _| *g = grad(90.0)),
            &HashMap::new(),
        );
        assert!(
            round.contains("rx=\"6.00\" ry=\"6.00\" fill=\"url(#g"),
            "{round}"
        );
        let square = body(
            &decorated(0.0, 0.0, 40.0, 40.0, 0.5, |_, g, _| *g = grad(90.0)),
            &HashMap::new(),
        );
        assert!(!square.contains("rx="), "{square}");
    }

    /// SVG path data of a glyph straight from ttf-parser, mapped by hand.
    struct Data(String, f32, f32, f32);

    impl ttf_parser::OutlineBuilder for Data {
        fn move_to(&mut self, x: f32, y: f32) {
            let (px, py) = (self.1 + x * self.3, self.2 - y * self.3);
            let _ = write!(self.0, "M{px:.2} {py:.2} ");
        }
        fn line_to(&mut self, x: f32, y: f32) {
            let (px, py) = (self.1 + x * self.3, self.2 - y * self.3);
            let _ = write!(self.0, "L{px:.2} {py:.2} ");
        }
        fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
            let m = |x: f32, y: f32| (self.1 + x * self.3, self.2 - y * self.3);
            let ((a, b), (c, d)) = (m(cx, cy), m(x, y));
            let _ = write!(self.0, "Q{a:.2} {b:.2} {c:.2} {d:.2} ");
        }
        fn curve_to(&mut self, ax: f32, ay: f32, bx: f32, by: f32, x: f32, y: f32) {
            let m = |x: f32, y: f32| (self.1 + x * self.3, self.2 - y * self.3);
            let ((a, b), (c, d), (e, f)) = (m(ax, ay), m(bx, by), m(x, y));
            let _ = write!(self.0, "C{a:.2} {b:.2} {c:.2} {d:.2} {e:.2} {f:.2} ");
        }
        fn close(&mut self) {
            self.0 += "Z ";
        }
    }

    #[test]
    fn text_paths_are_every_glyph_outline_at_its_pen_position() {
        let reg = FontRegistry::new();
        for family in ["Inter", "Liberation Serif"] {
            let face = reg.select(&[family], 400, false).unwrap().clone();
            let ttf = glyph::outline_face(&face);
            let scale = 24.0 / f32::from(face.units_per_em());
            let mut x = 0.0;
            let glyphs: Vec<PositionedGlyph> = face
                .shape("Ag%")
                .iter()
                .map(|g| {
                    let p = PositionedGlyph {
                        glyph_id: g.glyph_id,
                        x,
                        y: 20.0,
                        text_start: 0,
                        text_end: 0,
                    };
                    x += g.x_advance as f32 * scale;
                    p
                })
                .collect();
            let line = Fragment::new(
                NodeId(0),
                7.0,
                4.0,
                x,
                30.0,
                FragmentContent::TextLine {
                    glyphs: glyphs.clone(),
                    text: String::new(),
                    face: face.clone(),
                    font_size: 24.0,
                    color: RED,
                    vertical: false,
                    upright: false,
                },
            );
            let mut d = Data(String::new(), 0.0, 0.0, scale);
            for g in &glyphs {
                (d.1, d.2) = (7.0 + g.x, 4.0 + g.y);
                ttf.outline_glyph(ttf_parser::GlyphId(g.glyph_id), &mut d);
            }
            assert_eq!(
                body(&line, &HashMap::new()),
                format!("<path d=\"{}\" fill=\"#ff0000\"/>\n", d.0.trim_end()),
                "{family}"
            );
        }
    }

    #[test]
    fn square_borders_are_four_edge_bands_of_their_own_widths() {
        let side = |width: u16, color: Option<Rgba>| BorderSide { width, color };
        let blue = Some(Rgba::new(0, 0, 255, 255));
        let mut f = plain(10.0, 20.0, 40.0, 30.0, None);
        if let FragmentContent::Box { border, .. } = &mut f.content {
            *border = BorderEdges {
                top: side(1, blue),
                right: side(2, blue),
                bottom: side(3, blue),
                left: side(4, blue),
            };
        }
        let r = |x: f32, y: f32, w: f32, h: f32| {
            format!("<rect x=\"{x:.2}\" y=\"{y:.2}\" width=\"{w:.2}\" height=\"{h:.2}\" fill=\"#0000ff\"/>\n")
        };
        assert_eq!(
            body(&f, &HashMap::new()),
            [
                r(10.0, 20.0, 40.0, 1.0),
                r(10.0, 47.0, 40.0, 3.0),
                r(10.0, 20.0, 4.0, 30.0),
                r(48.0, 20.0, 2.0, 30.0)
            ]
            .concat()
        );
        // Zero-width or colourless sides are no band.
        if let FragmentContent::Box { border, .. } = &mut f.content {
            *border = BorderEdges {
                top: side(0, blue),
                right: side(3, None),
                bottom: side(0, blue),
                left: side(0, blue),
            };
        }
        assert_eq!(body(&f, &HashMap::new()), "");
    }

    #[test]
    fn backgrounds_need_a_size_a_colour_and_round_past_half_a_pixel() {
        let bg = |w: f32, h: f32, c: Rgba, radius: f32| {
            let mut f = plain(1.0, 2.0, w, h, Some(c));
            if let FragmentContent::Box { border_radius, .. } = &mut f.content {
                *border_radius = radius.into();
            }
            body(&f, &HashMap::new())
        };
        assert_eq!(bg(0.0, 5.0, RED, 0.0), "");
        assert_eq!(bg(5.0, 0.0, RED, 0.0), "");
        assert_eq!(bg(5.0, -1.0, RED, 0.0), "");
        assert_eq!(bg(5.0, 5.0, Rgba::new(255, 0, 0, 0), 0.0), "");
        assert_eq!(
            bg(5.0, 5.0, RED, 0.5),
            "<rect x=\"1.00\" y=\"2.00\" width=\"5.00\" height=\"5.00\" fill=\"#ff0000\"/>\n"
        );
        assert_eq!(
            bg(5.0, 5.0, RED, 0.75),
            "<rect x=\"1.00\" y=\"2.00\" width=\"5.00\" height=\"5.00\" rx=\"0.75\" ry=\"0.75\" fill=\"#ff0000\"/>\n"
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
                        text_start: 0,
                        text_end: 0,
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
                    text: String::new(),
                    face: face.clone(),
                    font_size: 20.0,
                    color,
                    vertical: false,
                    upright: false,
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

    #[test]
    fn opacity_wraps_the_box_and_its_subtree_in_one_group() {
        // miaskiewicz/turbo-dom#206: `opacity` is a group opacity over the whole subtree.
        let red = Rgba::new(255, 0, 0, 255);
        let mut half = plain(0.0, 0.0, 8.0, 8.0, Some(red));
        half.children.push(plain(0.0, 0.0, 4.0, 4.0, Some(red)));
        half.opacity = 0.5;
        let got = body(&half, &HashMap::new());
        assert!(got.starts_with("<g opacity=\"0.5\">"), "{got}");
        assert!(got.ends_with("</g>"), "{got}");
        assert_eq!(got.matches("<rect").count(), 2, "{got}");
        let mut gone = plain(0.0, 0.0, 8.0, 8.0, Some(red));
        gone.opacity = 0.0;
        assert_eq!(body(&gone, &HashMap::new()), "");
        assert!(!body(&plain(0.0, 0.0, 8.0, 8.0, Some(red)), &HashMap::new()).contains("opacity"));
    }

    #[test]
    fn overflow_clips_children_with_a_clip_path_of_the_padding_box() {
        let clipped = |w: f32, r: f32, child: Fragment| {
            let mut f = plain(1.0, 1.0, 8.0, 8.0, None);
            f.overflow_clip = Some(turbo_dom_layout::OverflowClip {
                x: 1.0,
                y: 1.0,
                width: w,
                height: 6.0,
                radii: r.into(),
            });
            f.children = vec![child];
            body(&f, &HashMap::new())
        };
        let big = plain(0.0, 0.0, 20.0, 20.0, Some(RED));
        // miaskiewicz/turbo-dom#253: square and rounded regions, an empty one (no child),
        // and children inside a square region (no clip path).
        assert_eq!(
            clipped(6.0, 0.0, big.clone()),
            format!(
                "<clipPath id=\"overflow130\"><rect x=\"2.00\" y=\"2.00\" width=\"6.00\" height=\"6.00\"/></clipPath><g clip-path=\"url(#overflow130)\">{}</g>",
                body(&big, &HashMap::new())
            )
        );
        assert!(clipped(6.0, 2.0, big.clone()).contains("rx=\"2.00\" ry=\"2.00\"/></clipPath>"));
        assert_eq!(clipped(0.0, 0.0, big), "");
        let small = plain(3.0, 3.0, 2.0, 2.0, Some(RED));
        assert_eq!(
            clipped(6.0, 0.0, small.clone()),
            body(&small, &HashMap::new())
        );
        // Elliptical corners clip with a path.
        let mut f = plain(1.0, 1.0, 8.0, 8.0, Some(RED));
        if let FragmentContent::Box { border_radius, .. } = &mut f.content {
            border_radius.top_left = turbo_dom_layout::CornerRadii { h: 4.0, v: 2.0 };
        }
        assert!(body(&f, &HashMap::new()).starts_with("<path d=\"M5.00 1.00 L9.00 1.00"));
    }
}
