//! Glyph coverage the way Chrome 156 on macOS paints text (miaskiewicz/turbo-dom#266,
//! #331).
//!
//! Chrome rasterizes glyphs through CoreText with font smoothing, which dilates each
//! outline, and the edge coverage then depends on the text's luminance. Measured against
//! the frozen Chrome 156 shell (`scripts/glyph-coverage-chrome-control.mjs`, fixture
//! Liberation Serif, 12–64px), the model here is:
//! - exact-area coverage (no supersampling steps);
//! - the outline grown by a box ([`embolden_contour`]): each edge moves out [`dilation`]
//!   (`0.0045 * size`) times the sum of its normal's |x| and |y|, so a 64px stem grows
//!   0.29 px a side and a 45° stroke 1.41 times that (a disc — the plain normal offset —
//!   leaves `W`, `x` and `y` 0.14 px off), and each concave vertex a little further
//!   ([`CONCAVE_FILL`]: Chrome fills joins);
//! - the whole run lifted by the growth plus [`LIFT`] px, so the baseline edge stays put
//!   and the top grows (CoreText's smoothing grows the glyph upward);
//! - horizontal pens at quarter pixels and baselines at whole pixels, Skia's subpixel
//!   glyph positions;
//! - the coverage taken through [`edge_transfer`]: linear for dark text from 28px up (a
//!   64px stem's edge pixels step 4, 61, 123, 189 over the four pen phases), a little
//!   darker below, and lighter for light text, by Skia's 3-bit text luminance.
//!
//! What stays different is measured in `tests/glyph_coverage_chrome.rs`: below about
//! 20px Chrome's vertical geometry depends on the size's pixel grid (a bar's height grows
//! 0.02–0.22 px from one size to the next; 16px glyphs stand about 4% taller), and some
//! features sit a fifth of a pixel off vertically (the 40px `g`'s bar), which no outline
//! offset reproduces (#331).

use tiny_skia::{Pixmap, PremultipliedColorU8, Transform};
use turbo_dom_layout::text::FontFace;
use turbo_dom_layout::{Fragment, PositionedGlyph, Rgba};

use crate::glyph::Pen;

/// Paint outline `o` of `size` px text in `color` over `pm` (exact coverage through the
/// colour's [`edge_transfer`] table, then source-over).
pub(crate) fn fill(pm: &mut Pixmap, o: &Outline, color: Rgba, size: f32) {
    let Some(window) = o.bounds(pm.width(), pm.height()) else {
        return;
    };
    let mut cov = Coverage::new(window);
    cov.add(o);
    let lut = edge_transfer(luminance_class(color.r, color.g, color.b), size);
    let mut layer =
        Pixmap::new(window.2 as u32, window.3 as u32).expect("bounds() is a non-empty window");
    let alpha = f32::from(color.a) / 255.0;
    for (px, c) in layer.pixels_mut().iter_mut().zip(cov.finish()) {
        let a = lut[(c * 255.0).round() as usize] * alpha;
        let ch = |v: u8| (f32::from(v) * a).round() as u8;
        *px = PremultipliedColorU8::from_rgba(ch(color.r), ch(color.g), ch(color.b), ch(255))
            .expect("channels scale by the same alpha");
    }
    pm.draw_pixmap(
        window.0,
        window.1,
        layer.as_ref(),
        &tiny_skia::PixmapPaint::default(),
        Transform::identity(),
        None,
    );
}

/// Outline growth per side per px of font size.
const DILATION_PER_PX: f32 = 0.0045;
/// The run's lift beyond its growth, in px.
const LIFT: f32 = 0.03;
/// A concave vertex's extra outward move, in growths per unit of the sine of its turn.
const CONCAVE_FILL: f32 = 0.1;
/// Dark text's edge darkening at 0px, tapering linearly to none at
/// [`DARK_CONTRAST_UNTIL`] px: the extra exponent of `1 - (1 - c)^e`.
const DARK_CONTRAST: f32 = 0.15;
/// The font size from which dark text's edge transfer is linear.
const DARK_CONTRAST_UNTIL: f32 = 28.0;
/// White text's extra edge gamma (`c^(1 / 1.4)`) and exponent (`1 - (1 - c)^1.3`).
const LIGHT_GAMMA: f32 = 0.4;
const LIGHT_BOOST: f32 = 0.3;

/// How far CoreText's smoothing grows a `size` px glyph's horizontal and vertical
/// edges, in px (the half-size of the box the outline is grown by).
pub(crate) fn dilation(size: f32) -> f32 {
    DILATION_PER_PX * size
}

/// A text line's glyph run as Chrome rasterizes it: each glyph traced at its pen through
/// `xf` (an upright glyph of a vertical run turned back about its pen), pens snapped and
/// lifted where the glyph lands unscaled and unturned, and the whole run emboldened.
pub(crate) fn run_outline(
    (f, upright): (&Fragment, bool),
    glyphs: &[PositionedGlyph],
    face: &FontFace,
    font_size: f32,
    xf: Transform,
) -> Outline {
    let scale = font_size / f32::from(face.units_per_em());
    let face = crate::glyph::outline_face(face);
    let d = dilation(font_size);
    let mut o = Outline::new();
    for g in glyphs {
        let pen = Pen {
            origin_x: f.x + g.x,
            baseline_y: f.y + g.y,
            scale,
        };
        let turn = crate::paint_png::upright_turn(upright, (pen.origin_x, pen.baseline_y));
        o.set_transform(snapped(
            xf.pre_concat(turn),
            (pen.origin_x, pen.baseline_y),
            d + LIFT,
        ));
        crate::glyph::trace_glyph(&face, g.glyph_id, pen, &mut o);
    }
    o.embolden(d);
    o
}

/// `xf` with the pen `(x, y)` moved to Skia's subpixel position (x to the nearest
/// quarter pixel, y to the whole pixel) and lifted `lift`, when `xf` only translates.
fn snapped(xf: Transform, (x, y): (f32, f32), lift: f32) -> Transform {
    if (xf.sx, xf.sy, xf.kx, xf.ky) != (1.0, 1.0, 0.0, 0.0) {
        return xf;
    }
    let (dx, dy) = (x + xf.tx, y + xf.ty);
    xf.post_translate(
        ((dx + 0.125) * 4.0).floor() / 4.0 - dx,
        (dy + 0.5).floor() - dy - lift,
    )
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Pt {
    x: f32,
    y: f32,
}

/// A segment kind: the tag of each point (a quad's control and end point both carry
/// `Quad`, a cubic's three points `Cubic`).
#[derive(Clone, Copy, Debug, PartialEq)]
enum Seg {
    Move,
    Line,
    Quad,
    Cubic,
}

/// A glyph run's outline as contours of tagged points, mapped through a transform.
pub(crate) struct Outline {
    pts: Vec<Pt>,
    tags: Vec<Seg>,
    /// The end (exclusive) of each contour in `pts`.
    ends: Vec<usize>,
    xf: Transform,
}

impl Outline {
    pub(crate) fn new() -> Self {
        Outline {
            pts: Vec::new(),
            tags: Vec::new(),
            ends: Vec::new(),
            xf: Transform::identity(),
        }
    }

    /// Map the points traced from now on through `xf`.
    pub(crate) fn set_transform(&mut self, xf: Transform) {
        self.end_contour();
        self.xf = xf;
    }

    fn push(&mut self, x: f32, y: f32, tag: Seg) {
        let mut p = tiny_skia::Point::from_xy(x, y);
        self.xf.map_points(std::slice::from_mut(&mut p));
        self.pts.push(Pt { x: p.x, y: p.y });
        self.tags.push(tag);
    }

    fn end_contour(&mut self) {
        let start = self.ends.last().copied().unwrap_or(0);
        if self.pts.len() > start {
            self.ends.push(self.pts.len());
        }
    }

    /// Grow every contour by a `2d` px box ([`embolden_contour`]; controls move with
    /// their polygon; holes shrink).
    pub(crate) fn embolden(&mut self, d: f32) {
        self.end_contour();
        let sign = if self.signed_area() >= 0.0 { d } else { -d };
        let mut start = 0;
        for &end in &self.ends {
            let grown = embolden_contour(&self.pts[start..end], sign);
            self.pts[start..end].copy_from_slice(&grown);
            start = end;
        }
    }

    fn contours(&self) -> impl Iterator<Item = (&[Pt], &[Seg])> {
        let starts = std::iter::once(0).chain(self.ends.iter().copied());
        starts
            .zip(self.ends.iter().copied())
            .map(|(s, e)| (&self.pts[s..e], &self.tags[s..e]))
    }

    fn signed_area(&self) -> f32 {
        self.contours()
            .flat_map(|(c, _)| c.iter().zip(c.iter().cycle().skip(1)))
            .map(|(a, b)| a.x * b.y - b.x * a.y)
            .sum()
    }

    /// The whole-pixel window, clipped to `w`x`h`, holding every point.
    pub(crate) fn bounds(&self, w: u32, h: u32) -> Option<(i32, i32, usize, usize)> {
        let mut b = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for p in &self.pts {
            b = (b.0.min(p.x), b.1.min(p.y), b.2.max(p.x), b.3.max(p.y));
        }
        let x0 = (b.0.floor() as i32).max(0);
        let y0 = (b.1.floor() as i32).max(0);
        let x1 = (b.2.ceil() as i32 + 1).min(w as i32);
        let y1 = (b.3.ceil() as i32 + 1).min(h as i32);
        (x0 < x1 && y0 < y1).then(|| (x0, y0, (x1 - x0) as usize, (y1 - y0) as usize))
    }

    /// The outline as SVG path data.
    pub(crate) fn svg_path(&self) -> String {
        use std::fmt::Write;
        let mut d = String::new();
        for (c, t) in self.contours() {
            let mut i = 0;
            while i < c.len() {
                let p = c[i];
                let n = match t[i] {
                    Seg::Move => {
                        let _ = write!(d, "M{:.2} {:.2} ", p.x, p.y);
                        1
                    }
                    Seg::Line => {
                        let _ = write!(d, "L{:.2} {:.2} ", p.x, p.y);
                        1
                    }
                    Seg::Quad => {
                        let e = c[i + 1];
                        let _ = write!(d, "Q{:.2} {:.2} {:.2} {:.2} ", p.x, p.y, e.x, e.y);
                        2
                    }
                    Seg::Cubic => {
                        let (b, e) = (c[i + 1], c[i + 2]);
                        let _ = write!(
                            d,
                            "C{:.2} {:.2} {:.2} {:.2} {:.2} {:.2} ",
                            p.x, p.y, b.x, b.y, e.x, e.y
                        );
                        3
                    }
                };
                i += n;
            }
            d.push_str("Z ");
        }
        d.truncate(d.trim_end().len());
        d
    }
}

impl crate::glyph::Tracer for Outline {
    fn move_to(&mut self, x: f32, y: f32) {
        self.end_contour();
        self.push(x, y, Seg::Move);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.push(x, y, Seg::Line);
    }
    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.push(cx, cy, Seg::Quad);
        self.push(x, y, Seg::Quad);
    }
    fn cubic_to(&mut self, c1x: f32, c1y: f32, c2x: f32, c2y: f32, x: f32, y: f32) {
        self.push(c1x, c1y, Seg::Cubic);
        self.push(c2x, c2y, Seg::Cubic);
        self.push(x, y, Seg::Cubic);
    }
    fn close(&mut self) {
        self.end_contour();
    }
}

/// The unit direction from `a` to `b`, or `None` for coincident points.
fn unit(a: Pt, b: Pt) -> Option<(f32, f32)> {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let l = dx.hypot(dy);
    (l > 1e-6).then(|| (dx / l, dy / l))
}

/// The direction of the edge into (`back`) or out of point `i` of closed contour `c`,
/// skipping coincident neighbours; `None` when every point coincides.
fn edge(c: &[Pt], i: usize, back: bool) -> Option<(f32, f32)> {
    let n = c.len();
    (1..n).find_map(|k| {
        if back {
            unit(c[(i + n - k) % n], c[i])
        } else {
            unit(c[i], c[(i + k) % n])
        }
    })
}

/// Each point of `c` moved to where its two edges meet once each has moved out
/// `d * (|nx| + |ny|)` along its normal `n` (a box's support), a concave point a further
/// `CONCAVE_FILL * d * |sin turn|` along its bisector; a point at a spike sharper than
/// `FreeType`'s limit (or of a degenerate contour) stays. `d`'s sign is the contour
/// orientation's, so `n * d` points out.
fn embolden_contour(c: &[Pt], d: f32) -> Vec<Pt> {
    (0..c.len())
        .map(|i| {
            let (Some(a), Some(b)) = (edge(c, i, true), edge(c, i, false)) else {
                return c[i];
            };
            let (na, nb) = ((a.1, -a.0), (b.1, -b.0));
            let k = 1.0 + na.0 * nb.0 + na.1 * nb.1;
            if k < 0.0625 {
                return c[i];
            }
            let turn = a.0 * b.1 - a.1 * b.0;
            let concave = if turn * d < 0.0 {
                CONCAVE_FILL * d * turn.abs()
            } else {
                0.0
            };
            let (dx, dy) = box_shift(na, nb, d);
            Pt {
                x: c[i].x + dx + concave * (na.0 + nb.0) / k,
                y: c[i].y + dy + concave * (na.1 + nb.1) / k,
            }
        })
        .collect()
}

/// The shift that moves both lines of normals `na` and `nb` (unit, `k = 1 + na·nb` above
/// the spike limit) out `d * (|nx| + |ny|)`: their meeting point, or the one shift of
/// collinear lines.
fn box_shift(na: (f32, f32), nb: (f32, f32), d: f32) -> (f32, f32) {
    let support = |n: (f32, f32)| d * (n.0.abs() + n.1.abs());
    let (ha, hb) = (support(na), support(nb));
    let det = na.0 * nb.1 - na.1 * nb.0;
    if det.abs() < 1e-3 {
        return (ha * na.0, ha * na.1);
    }
    ((ha * nb.1 - hb * na.1) / det, (na.0 * hb - nb.0 * ha) / det)
}

/// Exact-area coverage over a `w`x`h` window whose top left is `(x0, y0)` (the signed
/// area / cover accumulation of font-rs).
pub(crate) struct Coverage {
    x0: f32,
    y0: f32,
    w: usize,
    h: usize,
    acc: Vec<f32>,
}

impl Coverage {
    pub(crate) fn new((x0, y0, w, h): (i32, i32, usize, usize)) -> Self {
        Coverage {
            x0: x0 as f32,
            y0: y0 as f32,
            w,
            h,
            acc: vec![0.0; (w + 2) * h],
        }
    }

    /// Accumulate every contour of `o`, curves flattened.
    pub(crate) fn add(&mut self, o: &Outline) {
        for (c, t) in o.contours() {
            let mut prev = c[0];
            let mut i = 1;
            while i < c.len() {
                i += match t[i] {
                    Seg::Quad => {
                        self.curve(&[prev, c[i], c[i + 1]]);
                        2
                    }
                    Seg::Cubic => {
                        self.curve(&[prev, c[i], c[i + 1], c[i + 2]]);
                        3
                    }
                    Seg::Move | Seg::Line => {
                        self.line(prev, c[i]);
                        1
                    }
                };
                prev = c[i - 1];
            }
            self.line(prev, c[0]);
        }
    }

    /// A Bézier curve of control polygon `p` as line segments, enough that each strays
    /// under ~1/50 px from the curve.
    fn curve(&mut self, p: &[Pt]) {
        let dev = p
            .windows(3)
            .map(|w| (w[0].x - 2.0 * w[1].x + w[2].x).hypot(w[0].y - 2.0 * w[1].y + w[2].y))
            .fold(0.0, f32::max);
        let n = ((dev * 48.0).sqrt().ceil() as usize).clamp(1, 64);
        let mut from = p[0];
        for k in 1..=n {
            let to = bezier(p, k as f32 / n as f32);
            self.line(from, to);
            from = to;
        }
    }

    fn line(&mut self, p: Pt, q: Pt) {
        let wf = self.w as f32;
        let p = Pt {
            x: (p.x - self.x0).clamp(0.0, wf),
            y: p.y - self.y0,
        };
        let q = Pt {
            x: (q.x - self.x0).clamp(0.0, wf),
            y: q.y - self.y0,
        };
        let (dir, p, q) = if p.y < q.y { (1.0, p, q) } else { (-1.0, q, p) };
        let top = p.y.max(0.0);
        let bottom = q.y.min(self.h as f32);
        if top >= bottom {
            return;
        }
        let dxdy = (q.x - p.x) / (q.y - p.y);
        let mut x = p.x + (top - p.y) * dxdy;
        for y in (top as usize)..(bottom.ceil() as usize) {
            let fy = y as f32;
            let dy = (fy + 1.0).min(q.y) - fy.max(p.y);
            let next = x + dxdy * dy;
            self.span(y * (self.w + 2), (x, next), dy * dir);
            x = next;
        }
    }

    /// One row's piece of an edge running from `x` to `next` across height `d` (signed).
    fn span(&mut self, row: usize, (x, next): (f32, f32), d: f32) {
        let (x0, x1) = if x < next { (x, next) } else { (next, x) };
        let x0floor = x0.floor();
        let x0i = x0floor as usize;
        let x1ceil = x1.ceil();
        let x1i = x1ceil as usize;
        let acc = &mut self.acc[row..];
        if x1i <= x0i + 1 {
            let mid = 0.5 * (x + next) - x0floor;
            acc[x0i] += d - d * mid;
            acc[x0i + 1] += d * mid;
            return;
        }
        let s = (x1 - x0).recip();
        let x0f = x0 - x0floor;
        let a0 = 0.5 * s * (1.0 - x0f) * (1.0 - x0f);
        let x1f = x1 - x1ceil + 1.0;
        let am = 0.5 * s * x1f * x1f;
        acc[x0i] += d * a0;
        if x1i == x0i + 2 {
            acc[x0i + 1] += d * (1.0 - a0 - am);
        } else {
            let a1 = s * (1.5 - x0f);
            acc[x0i + 1] += d * (a1 - a0);
            for cell in &mut acc[x0i + 2..x1i - 1] {
                *cell += d * s;
            }
            let a2 = a1 + (x1i - x0i - 3) as f32 * s;
            acc[x1i - 1] += d * (1.0 - a2 - am);
        }
        acc[x1i] += d * am;
    }

    /// The coverage (0..=1) of each window pixel, row-major.
    pub(crate) fn finish(&self) -> impl Iterator<Item = f32> + '_ {
        self.acc.chunks(self.w + 2).flat_map(|row| {
            row[..self.w].iter().scan(0.0, |sum, a| {
                *sum += a;
                Some(f32::abs(*sum).min(1.0))
            })
        })
    }
}

/// The point at `t` of the Bézier curve with control polygon `p` (de Casteljau).
fn bezier(p: &[Pt], t: f32) -> Pt {
    let mut q = p.to_vec();
    while q.len() > 1 {
        for i in 0..q.len() - 1 {
            q[i] = Pt {
                x: q[i].x + (q[i + 1].x - q[i].x) * t,
                y: q[i].y + (q[i + 1].y - q[i].y) * t,
            };
        }
        q.pop();
    }
    q[0]
}

/// Skia's 3-bit luminance (0..=7) of text colour `(r, g, b)`: `SkComputeLuminance` of
/// the sRGB channels (`(54r + 183g + 19b) >> 8`), top 3 bits. Pure blue (18) is class 0,
/// with black; white is 7.
pub(crate) fn luminance_class(r: u8, g: u8, b: u8) -> u32 {
    ((u32::from(r) * 54 + u32::from(g) * 183 + u32::from(b) * 19) >> 8) >> 5
}

/// The edge transfer of `size` px text of luminance class `class`
/// ([`luminance_class`]): coverage `c` inks `1 - (1 - c^(1/g))^e`, where with
/// `t = class / 7` the gamma `g = 1 + LIGHT_GAMMA * t` and the exponent
/// `e = 1 + dark * (1 - t) + LIGHT_BOOST * t`, `dark` being [`DARK_CONTRAST`] tapering to
/// none at [`DARK_CONTRAST_UNTIL`] px. Black text from 28px up is linear.
pub(crate) fn edge_transfer(class: u32, size: f32) -> [f32; 256] {
    let t = class as f32 / 7.0;
    let dark = DARK_CONTRAST * (1.0 - size / DARK_CONTRAST_UNTIL).max(0.0);
    let gamma = (1.0 + LIGHT_GAMMA * t).recip();
    let e = 1.0 + dark * (1.0 - t) + LIGHT_BOOST * t;
    let mut table = [0.0; 256];
    for (i, v) in table.iter_mut().enumerate() {
        *v = 1.0 - (1.0 - (i as f32 / 255.0).powf(gamma)).powf(e);
    }
    table
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glyph::Tracer;

    fn square(o: &mut Outline, (x, y, s): (f32, f32, f32), cw: bool) {
        o.move_to(x, y);
        if cw {
            o.line_to(x + s, y);
            o.line_to(x + s, y + s);
            o.line_to(x, y + s);
        } else {
            o.line_to(x, y + s);
            o.line_to(x + s, y + s);
            o.line_to(x + s, y);
        }
        o.close();
    }

    fn coverage(o: &Outline, w: u32, h: u32) -> Vec<f32> {
        let mut c = Coverage::new(o.bounds(w, h).unwrap());
        c.add(o);
        c.finish().collect()
    }

    #[test]
    fn dilation_grows_with_the_font_size() {
        assert!((dilation(12.0) - 0.054).abs() < 1e-6);
        assert!((dilation(64.0) - 0.288).abs() < 1e-6);
    }

    #[test]
    fn a_square_covers_its_pixels_exactly() {
        // A 2.5 px square at (1.25, 1): columns 1 and 3 are 3/4 and 3/4 covered.
        let mut o = Outline::new();
        square(&mut o, (1.25, 1.0, 2.5), true);
        assert_eq!(o.bounds(10, 10), Some((1, 1, 4, 4)));
        let c = coverage(&o, 10, 10);
        let row: Vec<f32> = c[4..8].to_vec();
        for (got, want) in row.iter().zip([0.75, 1.0, 0.75, 0.0]) {
            assert!((got - want).abs() < 1e-5, "{row:?}");
        }
        // Row 3 is half covered (the square ends at y = 3.5).
        assert!((c[2 * 4 + 1] - 0.5).abs() < 1e-5, "{c:?}");
    }

    #[test]
    fn a_slanted_edge_spanning_several_columns_keeps_the_exact_area() {
        // A triangle (0,0) (8,0) (0,2): its area is 8, whatever the columns it crosses.
        let mut o = Outline::new();
        o.move_to(0.0, 0.0);
        o.line_to(8.0, 0.0);
        o.line_to(0.0, 2.0);
        o.close();
        let total: f32 = coverage(&o, 20, 20).iter().sum();
        assert!((total - 8.0).abs() < 1e-4, "{total}");
        // Shallow enough to cross two and three columns in a row too.
        for run in [1.5, 2.5] {
            let mut o = Outline::new();
            o.move_to(0.0, 0.0);
            o.line_to(run, 0.0);
            o.line_to(0.0, 1.0);
            o.close();
            let total: f32 = coverage(&o, 20, 20).iter().sum();
            assert!((total - run / 2.0).abs() < 1e-4, "{run}: {total}");
        }
    }

    #[test]
    fn curves_flatten_to_their_area() {
        // A quarter disc of radius 8 as a cubic, and a quad parabola segment.
        let mut o = Outline::new();
        let k = 0.552_284_8 * 8.0;
        o.move_to(0.0, 0.0);
        o.line_to(8.0, 0.0);
        o.cubic_to(8.0, k, k, 8.0, 0.0, 8.0);
        o.close();
        let total: f32 = coverage(&o, 20, 20).iter().sum();
        assert!(
            (total - std::f32::consts::PI * 16.0).abs() < 0.15,
            "{total}"
        );
        let mut o = Outline::new();
        o.move_to(0.0, 0.0);
        o.quad_to(3.0, 6.0, 6.0, 0.0);
        o.close();
        let total: f32 = coverage(&o, 20, 20).iter().sum();
        assert!((total - 12.0).abs() < 0.05, "{total}");
    }

    #[test]
    fn coverage_clips_to_the_window() {
        // A square hanging off every side of a 4x4 window covers it whole.
        let mut o = Outline::new();
        square(&mut o, (-3.0, -3.0, 10.0), true);
        assert_eq!(o.bounds(4, 4), Some((0, 0, 4, 4)));
        assert!(coverage(&o, 4, 4).iter().all(|&c| (c - 1.0).abs() < 1e-5));
        // Wholly outside: no window.
        let mut o = Outline::new();
        square(&mut o, (5.0, 5.0, 1.0), true);
        assert_eq!(o.bounds(4, 4), None);
        // An edge wholly above or below the window adds nothing.
        let mut c = Coverage::new((0, 2, 4, 2));
        c.line(Pt { x: 1.0, y: 0.0 }, Pt { x: 1.0, y: 1.5 });
        c.line(Pt { x: 1.0, y: 6.0 }, Pt { x: 1.0, y: 9.0 });
        assert!(c.finish().all(|v| v == 0.0));
    }

    #[test]
    fn emboldening_grows_outer_contours_and_shrinks_holes() {
        for cw in [true, false] {
            let mut o = Outline::new();
            square(&mut o, (2.0, 2.0, 6.0), cw);
            square(&mut o, (4.0, 4.0, 2.0), !cw);
            o.embolden(0.5);
            let outer: Vec<Pt> = o.pts[..4].to_vec();
            let xs: Vec<f32> = outer.iter().map(|p| p.x).collect();
            assert!(xs.contains(&1.5) && xs.contains(&8.5), "{xs:?}");
            // A hole's corners are concave: 0.5 in, plus the concave fill's 0.05.
            let hole: Vec<f32> = o.pts[4..].iter().map(|p| p.x).collect();
            let near = |v: f32| hole.iter().any(|x| (x - v).abs() < 1e-5);
            assert!(near(4.55) && near(5.45), "{hole:?}");
        }
    }

    #[test]
    fn emboldening_leaves_spikes_and_degenerate_contours() {
        let mut o = Outline::new();
        // A needle: its tip turns back on itself.
        o.move_to(0.0, 0.0);
        o.line_to(10.0, 0.5);
        o.line_to(0.0, 1.0);
        o.close();
        // A repeated point is skipped to find its edges.
        o.move_to(20.0, 0.0);
        o.line_to(20.0, 0.0);
        o.line_to(24.0, 0.0);
        o.line_to(24.0, 4.0);
        o.close();
        // A single point has no edge.
        o.move_to(30.0, 30.0);
        o.close();
        o.embolden(0.25);
        assert_eq!(o.pts[1], Pt { x: 10.0, y: 0.5 });
        assert_ne!(o.pts[3], Pt { x: 20.0, y: 0.0 });
        assert_eq!(o.pts[7], Pt { x: 30.0, y: 30.0 });
    }

    #[test]
    fn transforms_map_traced_points_and_the_path_serializes() {
        let mut o = Outline::new();
        o.set_transform(Transform::from_translate(1.0, 2.0));
        o.move_to(0.0, 0.0);
        o.line_to(1.0, 0.0);
        o.quad_to(2.0, 1.0, 1.0, 2.0);
        o.cubic_to(0.5, 2.0, 0.0, 1.5, 0.0, 1.0);
        o.set_transform(Transform::identity());
        o.move_to(5.0, 5.0);
        o.line_to(6.0, 5.0);
        o.line_to(6.0, 6.0);
        // Emboldening closes the open contour first.
        o.embolden(0.0);
        assert_eq!(o.ends, vec![7, 10]);
        assert_eq!(
            o.svg_path(),
            "M1.00 2.00 L2.00 2.00 Q3.00 3.00 2.00 4.00 C1.50 4.00 1.00 3.50 1.00 3.00 Z \
             M5.00 5.00 L6.00 5.00 L6.00 6.00 Z"
        );
    }

    #[test]
    fn edge_transfer_is_linear_for_large_dark_text_and_lighter_for_light_text() {
        let black = edge_transfer(0, 40.0);
        for (i, v) in black.iter().enumerate() {
            assert!((v - i as f32 / 255.0).abs() < 1e-6, "{i}: {v}");
        }
        // Below 28px dark text darkens: e = 1 + 0.15 * (1 - 14/28) at 14px.
        let small = edge_transfer(0, 14.0);
        assert!((small[128] - (1.0 - (1.0 - 128.0 / 255.0f32).powf(1.075))).abs() < 1e-6);
        assert!(small[128] > black[128]);
        // White: g = 1.4, e = 1.3, whatever the size.
        let white = edge_transfer(7, 14.0);
        assert_eq!(white, edge_transfer(7, 40.0));
        let want = 1.0 - (1.0 - 0.5f32.powf(1.0 / 1.4)).powf(1.3);
        assert!((white[128] - want).abs() < 0.005, "{}", white[128]);
        assert_eq!((white[0], white[255]), (0.0, 1.0));
        // A mid grey sits between.
        let grey = edge_transfer(4, 40.0);
        assert!(grey[128] > black[128] && grey[128] < white[128]);
        // Skia's 3-bit luminance: blue is class 0, with black.
        assert_eq!(luminance_class(0, 0, 0), 0);
        assert_eq!(luminance_class(0, 0, 255), 0);
        assert_eq!(luminance_class(128, 128, 128), 4);
        assert_eq!(luminance_class(255, 255, 255), 7);
    }

    #[test]
    fn emboldening_moves_edges_by_a_boxs_support_and_fills_concave_points() {
        // A right triangle: the legs move out d, the 45° hypotenuse (normal (1, 1)/√2)
        // d·√2 along its normal, so x + y = 4 becomes x + y = 4 + 2d (a disc: 4 + d·√2).
        let mut o = Outline::new();
        o.move_to(0.0, 0.0);
        o.line_to(0.0, 4.0);
        o.line_to(4.0, 0.0);
        o.close();
        o.embolden(0.5);
        let corner = o.pts[0];
        assert!(
            (corner.x + 0.5).abs() < 1e-5 && (corner.y + 0.5).abs() < 1e-5,
            "{corner:?}"
        );
        let p = o.pts[1];
        assert!(
            (p.x + p.y - 5.0).abs() < 1e-5 && (p.x + 0.5).abs() < 1e-5,
            "{p:?}"
        );
        // Collinear edges (a point mid-side) move along their one normal.
        let mut o = Outline::new();
        o.move_to(0.0, 0.0);
        o.line_to(0.0, 2.0);
        o.line_to(0.0, 4.0);
        o.line_to(4.0, 4.0);
        o.line_to(4.0, 0.0);
        o.close();
        o.embolden(0.5);
        assert_eq!(o.pts[1], Pt { x: -0.5, y: 2.0 });
        // An L's inner (concave) corner at (2, 2), between the bar's top edge and the
        // stem's right edge, moves to where those meet once moved, (2.5, 1.5), plus
        // 0.1 * d * |sin 90°| along their bisector (1, -1).
        let mut o = Outline::new();
        o.move_to(0.0, 0.0);
        o.line_to(0.0, 4.0);
        o.line_to(4.0, 4.0);
        o.line_to(4.0, 2.0);
        o.line_to(2.0, 2.0);
        o.line_to(2.0, 0.0);
        o.close();
        o.embolden(0.5);
        let inner = o.pts[4];
        let convex = o.pts[0];
        assert!(
            (inner.x - 2.55).abs() < 1e-5 && (inner.y - 1.45).abs() < 1e-5,
            "{inner:?}"
        );
        assert!(
            (convex.x + 0.5).abs() < 1e-5 && (convex.y + 0.5).abs() < 1e-5,
            "{convex:?}"
        );
    }
}
