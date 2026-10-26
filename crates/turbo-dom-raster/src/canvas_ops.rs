//! Shared front-end for the 2D-canvas op-log painters.
//!
//! A snapshot of a page's `<canvas>` draw calls arrives as a JSON op-log (the
//! record a fingerprint probe replays into `getContext('2d')`). Both backends —
//! the default tiny-skia [`crate::paint_canvas`] replay and the opt-in wgpu→Metal
//! [`crate::paint_canvas_gpu`] one — parse it here and share the same normalized
//! [`Op`] stream, CSS-colour/font parsing, and affine [`Matrix`], so the two agree
//! on *what* to draw and differ only in *how* the pixels are produced.

use turbo_dom_layout::text::{FontFace, FontRegistry};
use turbo_dom_layout::Rgba;

/// One normalized canvas draw/state op. Coordinates stay in canvas user space;
/// each backend folds the current [`Matrix`] in when it emits geometry. Unknown
/// ops in the log are dropped by [`parse_ops`] rather than erroring (a forward-
/// compatible probe may record state we don't paint).
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Op {
    Save,
    Restore,
    Translate(f32, f32),
    Scale(f32, f32),
    Rotate(f32),
    /// `transform(a,b,c,d,e,f)`: post-multiply the current matrix.
    Transform([f32; 6]),
    /// `setTransform(a,b,c,d,e,f)`: replace the current matrix.
    SetTransform([f32; 6]),
    ResetTransform,
    FillStyle(Rgba),
    StrokeStyle(Rgba),
    GlobalAlpha(f32),
    /// `font` shorthand, reduced to the pixel size + the family fallback list.
    Font {
        size_px: f32,
        families: Vec<String>,
    },
    FillRect(f32, f32, f32, f32),
    StrokeRect(f32, f32, f32, f32),
    ClearRect(f32, f32, f32, f32),
    BeginPath,
    ClosePath,
    MoveTo(f32, f32),
    LineTo(f32, f32),
    Rect(f32, f32, f32, f32),
    Arc {
        x: f32,
        y: f32,
        r: f32,
        start: f32,
        end: f32,
        ccw: bool,
    },
    Fill,
    Stroke,
    FillText {
        text: String,
        x: f32,
        y: f32,
    },
    StrokeText {
        text: String,
        x: f32,
        y: f32,
    },
}

/// Parse the render tier's 2D-canvas op-log into a normalized [`Op`] stream. The log is the
/// JSON array the render tier serializes from `ctx._ops`: each entry is a fixed TUPLE
/// `[name, argsArray, fillStyle, strokeStyle, font, textBaseline, textAlign, globalAlpha,
/// globalCompositeOperation]` — the op name, its positional numeric/string args, and the
/// drawing state snapshotted at the call. Because the state travels ON each tuple, we emit the
/// carried `fillStyle`/`strokeStyle`/`font`/`globalAlpha` as state [`Op`]s just before the
/// geometry op, so the stateful interpreter paints each op with the state it was recorded under.
/// A non-array top level or unparseable JSON is an `Err`; malformed / unknown entries are skipped.
pub(crate) fn parse_ops(ops_json: &str) -> Result<Vec<Op>, String> {
    let root: serde_json::Value =
        serde_json::from_str(ops_json).map_err(|e| format!("canvas op-log json: {e}"))?;
    let entries = root
        .as_array()
        .ok_or_else(|| "canvas op-log: expected a JSON array".to_string())?;
    let mut out = Vec::with_capacity(entries.len() * 2);
    for e in entries {
        push_tuple(e, &mut out);
    }
    Ok(out)
}

/// Translate one recorded tuple into zero or more [`Op`]s (the carried state snapshot, then the
/// geometry op). Skips a tuple with no op name or missing required args.
fn push_tuple(v: &serde_json::Value, out: &mut Vec<Op>) {
    let Some(t) = v.as_array() else { return };
    let Some(name) = t.first().and_then(serde_json::Value::as_str) else {
        return;
    };
    let args = t.get(1).and_then(serde_json::Value::as_array);
    let num = |i: usize| {
        args.and_then(|a| a.get(i))
            .and_then(serde_json::Value::as_f64)
            .map_or(0.0, |f| f as f32)
    };
    let arg_str = |i: usize| {
        args.and_then(|a| a.get(i))
            .and_then(serde_json::Value::as_str)
    };
    let arg_flag = |i: usize| {
        args.and_then(|a| a.get(i))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
    };

    // Drawing-state snapshot carried on the tuple (indices 2/3/4/7).
    if let Some(c) = t
        .get(2)
        .and_then(serde_json::Value::as_str)
        .and_then(parse_css_color)
    {
        out.push(Op::FillStyle(c));
    }
    if let Some(c) = t
        .get(3)
        .and_then(serde_json::Value::as_str)
        .and_then(parse_css_color)
    {
        out.push(Op::StrokeStyle(c));
    }
    if let Some(f) = t.get(4).and_then(serde_json::Value::as_str) {
        let (size_px, families) = parse_font(f);
        out.push(Op::Font { size_px, families });
    }
    if let Some(a) = t.get(7).and_then(serde_json::Value::as_f64) {
        out.push(Op::GlobalAlpha((a as f32).clamp(0.0, 1.0)));
    }

    let op = match name {
        "save" => Op::Save,
        "restore" => Op::Restore,
        "translate" => Op::Translate(num(0), num(1)),
        "scale" => Op::Scale(num(0), num(1)),
        "rotate" => Op::Rotate(num(0)),
        "transform" => Op::Transform([num(0), num(1), num(2), num(3), num(4), num(5)]),
        "setTransform" => Op::SetTransform([num(0), num(1), num(2), num(3), num(4), num(5)]),
        "resetTransform" => Op::ResetTransform,
        "fillRect" => Op::FillRect(num(0), num(1), num(2), num(3)),
        "strokeRect" => Op::StrokeRect(num(0), num(1), num(2), num(3)),
        "clearRect" => Op::ClearRect(num(0), num(1), num(2), num(3)),
        "beginPath" => Op::BeginPath,
        "closePath" => Op::ClosePath,
        "moveTo" => Op::MoveTo(num(0), num(1)),
        "lineTo" => Op::LineTo(num(0), num(1)),
        "rect" => Op::Rect(num(0), num(1), num(2), num(3)),
        "arc" => Op::Arc {
            x: num(0),
            y: num(1),
            r: num(2),
            start: num(3),
            end: num(4),
            ccw: arg_flag(5),
        },
        "fill" => Op::Fill,
        "stroke" => Op::Stroke,
        "fillText" => match arg_str(0) {
            Some(txt) => Op::FillText {
                text: txt.to_string(),
                x: num(1),
                y: num(2),
            },
            None => return,
        },
        "strokeText" => match arg_str(0) {
            Some(txt) => Op::StrokeText {
                text: txt.to_string(),
                x: num(1),
                y: num(2),
            },
            None => return,
        },
        _ => return,
    };
    out.push(op);
}

/// Parse a canvas `fillStyle`/`strokeStyle` colour string through turbo-dom's CSS colour
/// parser ([`parse_color`](turbo_dom_layout::turbo_dom::rtdom::color::parse_color): hex,
/// every named colour, `rgb()`/`rgba()` with number or percentage channels, `hsl()`/
/// `hsla()`), each channel and the alpha rounded to a byte as a canvas stores it. `None`
/// for anything else (gradients/patterns are objects, not strings), which leaves the
/// previous style, as a canvas ignores an invalid assignment.
pub(crate) fn parse_css_color(s: &str) -> Option<Rgba> {
    let c = turbo_dom_layout::turbo_dom::rtdom::color::parse_color(s)?;
    let byte = |v: f64| v.round() as u8;
    Some(Rgba::new(
        byte(c.r),
        byte(c.g),
        byte(c.b),
        byte(c.a * 255.0),
    ))
}

/// Reduce a CSS `font` shorthand to `(size_px, families)`. We only need the pixel
/// size and the family fallback list (weight/style are not modelled by the
/// bundled face set). The first `NNpx`/`NNpt` token is the size; everything after
/// it is the comma-joined family list. Defaults to `10px sans-serif` (the canvas
/// initial font) when no size is found.
pub(crate) fn parse_font(shorthand: &str) -> (f32, Vec<String>) {
    let mut size_px = 10.0f32;
    let mut families = Vec::new();
    if let Some((size_tok, rest)) = split_font_size(shorthand) {
        size_px = size_tok;
        families = rest
            .split(',')
            .map(|f| {
                f.trim()
                    .trim_matches(|c| c == '\'' || c == '"')
                    .trim()
                    .to_string()
            })
            .filter(|f| !f.is_empty())
            .collect();
    }
    if families.is_empty() {
        families.push("sans-serif".to_string());
    }
    (size_px, families)
}

/// Find the first `NNpx`/`NNpt` size token, returning `(px, tail_after_token)`.
fn split_font_size(shorthand: &str) -> Option<(f32, &str)> {
    // Iterate with byte offsets so the family tail is sliced from the same string;
    // `split_whitespace` collapses whitespace runs, so an index-based re-split would
    // desync on a double space (`bold  20px Arial`).
    for (start, tok) in shorthand.split_whitespace().map(|t| {
        let off = t.as_ptr() as usize - shorthand.as_ptr() as usize;
        (off, t)
    }) {
        if let Some(px) = font_size_px(tok) {
            let tail = shorthand[start + tok.len()..].trim_start();
            return Some((px, tail));
        }
    }
    None
}

/// A `NNpx` (verbatim) or `NNpt` (→px at 96/72) size token, else `None`.
fn font_size_px(tok: &str) -> Option<f32> {
    if let Some(px) = tok.strip_suffix("px") {
        return px.parse::<f32>().ok();
    }
    if let Some(pt) = tok.strip_suffix("pt") {
        return pt.parse::<f32>().ok().map(|v| v * 96.0 / 72.0);
    }
    None
}

/// A 2×3 affine matrix `[a, b, c, d, e, f]` in canvas order: a point maps as
/// `x' = a·x + c·y + e`, `y' = b·x + d·y + f`. Shared by both backends so path
/// points land in the same device pixels regardless of who paints them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Matrix(pub [f32; 6]);

impl Matrix {
    pub(crate) fn identity() -> Self {
        Matrix([1.0, 0.0, 0.0, 1.0, 0.0, 0.0])
    }

    /// Map a point through the matrix.
    pub(crate) fn apply(&self, x: f32, y: f32) -> (f32, f32) {
        let [a, b, c, d, e, f] = self.0;
        (a * x + c * y + e, b * x + d * y + f)
    }

    /// `self ∘ m`: the returned matrix applies `m` first, then `self` — canvas's
    /// `transform()`/`translate()`/… semantics (the new transform composes into
    /// the current one on the right).
    pub(crate) fn then(&self, m: [f32; 6]) -> Self {
        let [a1, b1, c1, d1, e1, f1] = self.0;
        let [a2, b2, c2, d2, e2, f2] = m;
        Matrix([
            a1 * a2 + c1 * b2,
            b1 * a2 + d1 * b2,
            a1 * c2 + c1 * d2,
            b1 * c2 + d1 * d2,
            a1 * e2 + c1 * f2 + e1,
            b1 * e2 + d1 * f2 + f1,
        ])
    }

    pub(crate) fn translate(&self, x: f32, y: f32) -> Self {
        self.then([1.0, 0.0, 0.0, 1.0, x, y])
    }

    pub(crate) fn scale(&self, x: f32, y: f32) -> Self {
        self.then([x, 0.0, 0.0, y, 0.0, 0.0])
    }

    pub(crate) fn rotate(&self, rad: f32) -> Self {
        let (s, c) = rad.sin_cos();
        self.then([c, s, -s, c, 0.0, 0.0])
    }
}

/// Resolve the bundled font face for a canvas `font` family list. Cached: the
/// bundled registry is parsed once and cloned (faces are `Arc`-shared, so the
/// clone is cheap) — canvas text never uses system fonts (deterministic/offline).
/// Always a face: this crate enables turbo-dom-layout's `bundled-fonts`, and
/// `FontRegistry::select` falls back to the first bundled face when no family matches.
pub(crate) fn resolve_face(families: &[String]) -> FontFace {
    static BUNDLED: std::sync::LazyLock<FontRegistry> = std::sync::LazyLock::new(FontRegistry::new);
    let refs: Vec<&str> = families.iter().map(String::as_str).collect();
    BUNDLED
        .select(&refs, 400, false)
        .cloned()
        .expect("the bundled faces are registered")
}

// ---------------------------------------------------------------------------
// Shared interpreter: state machine + geometry, backend-agnostic
// ---------------------------------------------------------------------------

/// A finished subpath in **device pixels** (the current transform already folded
/// in). `closed` marks a `closePath`/rect/glyph contour, which a stroke joins end
/// to start; a fill closes every subpath implicitly.
pub(crate) struct SubPath {
    pub pts: Vec<(f32, f32)>,
    pub closed: bool,
}

/// The pixel primitives a backend must provide. The [`run`] interpreter feeds
/// each in device space with the effective colour (`globalAlpha` already folded
/// into the alpha byte), so a backend only rasterizes — it holds no canvas state.
pub(crate) trait CanvasBackend {
    /// Fill `subs` as one region with the non-zero winding rule (so glyph holes
    /// and even-odd-looking overlaps behave like `ctx.fill()`).
    fn fill(&mut self, subs: &[SubPath], color: Rgba);
    /// Stroke `subs` with a device-space `width`.
    fn stroke(&mut self, subs: &[SubPath], color: Rgba, width: f32);
    /// Clear a (possibly transformed) quad to transparent black, replacing — not
    /// compositing — the covered pixels (`ctx.clearRect`).
    fn clear(&mut self, quad: &[(f32, f32); 4]);
    /// Fill text. Default: trace glyph outlines (ttf-parser) into subpaths + fill. A backend may
    /// override to rasterize text through a system stack (CoreText) whose glyph hinting/AA matches a
    /// real browser's — `getImageData` over the text then reads back like Chrome.
    fn fill_text(&mut self, st: &State, text: &str, x: f32, y: f32, color: Rgba) {
        self.fill(&glyph_subpaths(st, text, x, y), color);
    }
    /// Stroke text (same default-trace path as [`fill_text`]).
    fn stroke_text(&mut self, st: &State, text: &str, x: f32, y: f32, color: Rgba, width: f32) {
        self.stroke(&glyph_subpaths(st, text, x, y), color, width);
    }
}

/// Mutable drawing state, snapshotted by `save`/restored by `restore`.
#[derive(Clone)]
pub(crate) struct State {
    pub(crate) ctm: Matrix,
    fill: Rgba,
    stroke: Rgba,
    alpha: f32,
    // Strokes render at the CSS default 1px: the render tier's recorded op-log tuple carries no
    // lineWidth (it snapshots only fillStyle/strokeStyle/font/globalAlpha), so per-stroke widths
    // can't be reconstructed here. Honest limitation of the vendored recorder, not a bug to fake.
    line_width: f32,
    pub(crate) font_px: f32,
    pub(crate) font_families: Vec<String>,
}

impl Default for State {
    fn default() -> Self {
        // Canvas 2D initial state: black fill/stroke, 1px lines, `10px sans-serif`.
        State {
            ctm: Matrix::identity(),
            fill: Rgba::BLACK,
            stroke: Rgba::BLACK,
            alpha: 1.0,
            line_width: 1.0,
            font_px: 10.0,
            font_families: vec!["sans-serif".to_string()],
        }
    }
}

/// Replay a normalized [`Op`] stream against `backend`, tracking the full canvas
/// 2D state (transform stack, styles, current path) and emitting device-space
/// geometry. Never panics: unpaintable ops are simply no-ops.
pub(crate) fn run(ops: &[Op], backend: &mut dyn CanvasBackend) {
    let mut st = State::default();
    let mut stack: Vec<State> = Vec::new();
    let mut path: Vec<SubPath> = Vec::new();

    for op in ops {
        match op {
            Op::Save => stack.push(st.clone()),
            Op::Restore => {
                if let Some(prev) = stack.pop() {
                    st = prev;
                }
            }
            Op::Translate(x, y) => st.ctm = st.ctm.translate(*x, *y),
            Op::Scale(x, y) => st.ctm = st.ctm.scale(*x, *y),
            Op::Rotate(a) => st.ctm = st.ctm.rotate(*a),
            Op::Transform(m) => st.ctm = st.ctm.then(*m),
            Op::SetTransform(m) => st.ctm = Matrix(*m),
            Op::ResetTransform => st.ctm = Matrix::identity(),
            Op::FillStyle(c) => st.fill = *c,
            Op::StrokeStyle(c) => st.stroke = *c,
            Op::GlobalAlpha(a) => st.alpha = *a,
            Op::Font { size_px, families } => {
                st.font_px = *size_px;
                st.font_families.clone_from(families);
            }
            Op::FillRect(x, y, w, h) => {
                backend.fill(
                    &[rect_subpath(&st.ctm, *x, *y, *w, *h)],
                    alpha_mul(st.fill, st.alpha),
                );
            }
            Op::StrokeRect(x, y, w, h) => backend.stroke(
                &[rect_subpath(&st.ctm, *x, *y, *w, *h)],
                alpha_mul(st.stroke, st.alpha),
                device_line_width(&st.ctm, st.line_width),
            ),
            Op::ClearRect(x, y, w, h) => backend.clear(&rect_corners(&st.ctm, *x, *y, *w, *h)),
            Op::BeginPath => path.clear(),
            Op::ClosePath => {
                if let Some(last) = path.last_mut() {
                    last.closed = true;
                }
            }
            Op::MoveTo(x, y) => path.push(SubPath {
                pts: vec![st.ctm.apply(*x, *y)],
                closed: false,
            }),
            Op::LineTo(x, y) => append_point(&mut path, st.ctm.apply(*x, *y)),
            Op::Rect(x, y, w, h) => path.push(rect_subpath(&st.ctm, *x, *y, *w, *h)),
            Op::Arc {
                x,
                y,
                r,
                start,
                end,
                ccw,
            } => arc_into(&mut path, &st.ctm, *x, *y, *r, *start, *end, *ccw),
            Op::Fill => backend.fill(&path, alpha_mul(st.fill, st.alpha)),
            Op::Stroke => backend.stroke(
                &path,
                alpha_mul(st.stroke, st.alpha),
                device_line_width(&st.ctm, st.line_width),
            ),
            Op::FillText { text, x, y } => {
                backend.fill_text(&st, text, *x, *y, alpha_mul(st.fill, st.alpha));
            }
            Op::StrokeText { text, x, y } => {
                backend.stroke_text(
                    &st,
                    text,
                    *x,
                    *y,
                    alpha_mul(st.stroke, st.alpha),
                    device_line_width(&st.ctm, st.line_width),
                );
            }
        }
    }
}

/// Fold `globalAlpha` into a colour's alpha byte.
fn alpha_mul(c: Rgba, alpha: f32) -> Rgba {
    Rgba::new(
        c.r,
        c.g,
        c.b,
        (f32::from(c.a) * alpha.clamp(0.0, 1.0)).round() as u8,
    )
}

/// Line widths scale by the transform; approximate with the matrix's area scale
/// (√|det|), which is exact for uniform scales and reasonable for the rest.
fn device_line_width(m: &Matrix, w: f32) -> f32 {
    let [a, b, c, d, ..] = m.0;
    let det = (a * d - b * c).abs().sqrt();
    (w * if det > 0.0 { det } else { 1.0 }).max(0.0)
}

/// The four device-space corners of a canvas-space rect (a parallelogram under a
/// skew/rotate transform).
fn rect_corners(m: &Matrix, x: f32, y: f32, w: f32, h: f32) -> [(f32, f32); 4] {
    [
        m.apply(x, y),
        m.apply(x + w, y),
        m.apply(x + w, y + h),
        m.apply(x, y + h),
    ]
}

/// A closed subpath for a rect.
fn rect_subpath(m: &Matrix, x: f32, y: f32, w: f32, h: f32) -> SubPath {
    SubPath {
        pts: rect_corners(m, x, y, w, h).to_vec(),
        closed: true,
    }
}

/// Append a device point to the open current subpath. On an empty path a bare `lineTo`
/// acts as `moveTo`; after a closed subpath (`closePath`, `rect`) the new subpath starts at
/// the closed one's first point, as canvas's `closePath` leaves the current point there.
fn append_point(path: &mut Vec<SubPath>, p: (f32, f32)) {
    match path.last_mut() {
        Some(last) if !last.closed => last.pts.push(p),
        Some(last) => {
            let start = last.pts[0];
            path.push(SubPath {
                pts: vec![start, p],
                closed: false,
            });
        }
        None => path.push(SubPath {
            pts: vec![p],
            closed: false,
        }),
    }
}

/// Flatten a canvas `arc` into line segments (in user space, then transformed) and
/// append them to the current subpath, connecting from the current point.
#[allow(clippy::too_many_arguments)] // arc params + the transform
fn arc_into(
    path: &mut Vec<SubPath>,
    m: &Matrix,
    x: f32,
    y: f32,
    r: f32,
    start: f32,
    end: f32,
    ccw: bool,
) {
    if r <= 0.0 {
        return;
    }
    // Normalize the sweep the way canvas does: CW extends the end forward, CCW back.
    let a0 = start;
    let mut a1 = end;
    let tau = std::f32::consts::TAU;
    if ccw {
        while a1 > a0 {
            a1 -= tau;
        }
    } else {
        while a1 < a0 {
            a1 += tau;
        }
    }
    // Canvas clamps the drawn sweep to a single full turn — an end angle > 2π from start does not
    // overdraw multiple laps. Clamp the endpoint (not just the segment count) so the interpolation
    // below spans at most 2π.
    let dir = if a1 >= a0 { 1.0 } else { -1.0 };
    let sweep = (a1 - a0).abs().min(tau);
    a1 = a0 + dir * sweep;
    let segs = ((sweep / tau) * 64.0).ceil().max(6.0) as usize;
    // Sample a0 → a1 in `segs` steps. The first point connects from the current
    // point (canvas draws a line to the arc's start); `append_point` handles that.
    for i in 0..=segs {
        let ang = a0 + (a1 - a0) * (i as f32 / segs as f32);
        append_point(path, m.apply(x + r * ang.cos(), y + r * ang.sin()));
    }
}

/// Shape `text` with the state's font (a bundled face; see [`resolve_face`]) and trace
/// each glyph's outline into device-space subpaths (flattened) at baseline `(x, y)`.
/// Empty when no glyph has an outline (spaces, an empty string).
pub(crate) fn glyph_subpaths(st: &State, text: &str, x: f32, y: f32) -> Vec<SubPath> {
    let face = resolve_face(&st.font_families);
    // Non-zero: `ttf-parser` rejects a `head` whose units-per-em is outside 16..=16384.
    let scale = st.font_px / f32::from(face.units_per_em());
    let ttf = crate::glyph::outline_face(&face);
    let mut sink = FlattenSink {
        ctm: st.ctm,
        subs: Vec::new(),
        cur: Vec::new(),
    };
    let mut pen_x = x;
    for g in face.shape(text) {
        let pen = crate::glyph::Pen {
            origin_x: pen_x + g.x_offset as f32 * scale,
            baseline_y: y - g.y_offset as f32 * scale,
            scale,
        };
        crate::glyph::trace_glyph(&ttf, g.glyph_id, pen, &mut sink);
        sink.flush();
        pen_x += g.x_advance as f32 * scale;
    }
    sink.subs
}

/// A [`crate::glyph::Tracer`] that flattens each glyph contour to a polyline and
/// applies the current transform, yielding [`SubPath`]s. Beziers are subdivided
/// into a fixed number of steps — the fidelity limit both backends share.
struct FlattenSink {
    ctm: Matrix,
    subs: Vec<SubPath>,
    cur: Vec<(f32, f32)>,
}

/// Line segments a quadratic/cubic curve is flattened into (text is small, so a
/// modest count is visually clean while keeping tessellation cheap).
const CURVE_STEPS: usize = 8;

impl FlattenSink {
    /// Emit the accumulated contour (if any) as a closed subpath.
    fn flush(&mut self) {
        if self.cur.len() >= 2 {
            self.subs.push(SubPath {
                pts: std::mem::take(&mut self.cur),
                closed: true,
            });
        } else {
            self.cur.clear();
        }
    }
    fn push(&mut self, x: f32, y: f32) {
        self.cur.push(self.ctm.apply(x, y));
    }
}

impl crate::glyph::Tracer for FlattenSink {
    fn move_to(&mut self, x: f32, y: f32) {
        // A new contour begins: emit the previous one, then start fresh.
        self.flush();
        self.push(x, y);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.push(x, y);
    }
    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        let (x0, y0) = last_local(&self.cur, self.ctm, (x, y));
        for i in 1..=CURVE_STEPS {
            let t = i as f32 / CURVE_STEPS as f32;
            let mt = 1.0 - t;
            let px = mt * mt * x0 + 2.0 * mt * t * cx + t * t * x;
            let py = mt * mt * y0 + 2.0 * mt * t * cy + t * t * y;
            self.push(px, py);
        }
    }
    fn cubic_to(&mut self, c1x: f32, c1y: f32, c2x: f32, c2y: f32, x: f32, y: f32) {
        let (x0, y0) = last_local(&self.cur, self.ctm, (x, y));
        for i in 1..=CURVE_STEPS {
            let t = i as f32 / CURVE_STEPS as f32;
            let mt = 1.0 - t;
            let px = mt * mt * mt * x0
                + 3.0 * mt * mt * t * c1x
                + 3.0 * mt * t * t * c2x
                + t * t * t * x;
            let py = mt * mt * mt * y0
                + 3.0 * mt * mt * t * c1y
                + 3.0 * mt * t * t * c2y
                + t * t * t * y;
            self.push(px, py);
        }
    }
    fn close(&mut self) {
        self.flush();
    }
}

/// Recover the curve's start point in **local** (pre-transform) space from the
/// last device point — invert the matrix. Falls back to the curve end if the
/// matrix is singular (degenerate; the segment then collapses harmlessly).
fn last_local(cur: &[(f32, f32)], m: Matrix, fallback: (f32, f32)) -> (f32, f32) {
    let Some(&(dx, dy)) = cur.last() else {
        return fallback;
    };
    let [a, b, c, d, e, f] = m.0;
    let det = a * d - b * c;
    if det.abs() < 1e-9 {
        return fallback;
    }
    let (px, py) = (dx - e, dy - f);
    ((d * px - c * py) / det, (a * py - b * px) / det)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgba_accepts_fractional_and_percentage_alpha() {
        // 0..1 fractional alpha.
        let c = parse_css_color("rgba(10, 20, 30, 0.5)").unwrap();
        assert_eq!((c.r, c.g, c.b), (10, 20, 30));
        assert_eq!(c.a, 128); // round(0.5 * 255)
                              // CSS4 percentage alpha — must map N% → N/100, not reject.
        let p = parse_css_color("rgba(10, 20, 30, 50%)").unwrap();
        assert_eq!(p.a, 128);
        // Space/slash CSS4 form: `rgb(r g b / a)`.
        let s = parse_css_color("rgb(10 20 30 / 50%)").unwrap();
        assert_eq!((s.r, s.g, s.b, s.a), (10, 20, 30, 128));
        let s2 = parse_css_color("rgb(10 20 30 / 0.25)").unwrap();
        assert_eq!(s2.a, 64);
    }

    /// One tuple with no carried state (only the name and args).
    fn bare(name: &str, args: &str) -> String {
        format!(r#"["{name}",{args}]"#)
    }

    #[test]
    fn tuples_without_a_name_or_a_text_argument_are_skipped() {
        let log = format!(
            "[[1,[2]],[],{},{},{},{},{}]",
            bare("fillText", "[3,1,2]"),
            bare("strokeText", "[]"),
            bare("fillText", r#"["hi",1,2]"#),
            bare("strokeText", r#"["yo",3]"#),
            bare("arc", "[1,2,3,0,1,true]"),
        );
        assert_eq!(
            parse_ops(&log).unwrap(),
            [
                Op::FillText {
                    text: "hi".into(),
                    x: 1.0,
                    y: 2.0
                },
                Op::StrokeText {
                    text: "yo".into(),
                    x: 3.0,
                    y: 0.0
                },
                Op::Arc {
                    x: 1.0,
                    y: 2.0,
                    r: 3.0,
                    start: 0.0,
                    end: 1.0,
                    ccw: true
                },
            ]
        );
    }

    #[test]
    fn carried_state_becomes_state_ops_before_the_op() {
        let ops = parse_ops(r##"[["save",[],"#00f","bogus","bold 12px Arial",null,null,0.25]]"##)
            .unwrap();
        assert_eq!(
            ops,
            [
                Op::FillStyle(Rgba::new(0, 0, 255, 255)),
                Op::Font {
                    size_px: 12.0,
                    families: vec!["Arial".into()]
                },
                Op::GlobalAlpha(0.25),
                Op::Save,
            ],
            "an unparseable strokeStyle carries no state"
        );
        let clamped = parse_ops(r#"[["restore",[],null,null,null,null,null,7]]"#).unwrap();
        assert_eq!(clamped, [Op::GlobalAlpha(1.0), Op::Restore]);
    }

    #[test]
    fn font_shorthand_defaults_without_a_size_or_family() {
        let sans = vec!["sans-serif".to_string()];
        assert_eq!(
            parse_font("bold Arial"),
            (10.0, sans.clone()),
            "no size token"
        );
        assert_eq!(parse_font("14px"), (14.0, sans.clone()), "no family");
        assert_eq!(parse_font("xpx 9px , ''"), (9.0, sans.clone()));
        assert_eq!(parse_font(""), (10.0, sans));
    }

    #[test]
    fn curves_with_no_current_point_start_at_their_end_point() {
        // A glyph contour always opens with `move_to`; a curve without one starts at its
        // own end point, so the flattened segment begins (t = 1/8) between it and the
        // control point.
        let mut sink = FlattenSink {
            ctm: Matrix::identity(),
            subs: Vec::new(),
            cur: Vec::new(),
        };
        crate::glyph::Tracer::quad_to(&mut sink, 8.0, 0.0, 0.0, 0.0);
        let t = 1.0 / CURVE_STEPS as f32;
        assert_eq!(sink.cur[0], (2.0 * (1.0 - t) * t * 8.0, 0.0));
        assert_eq!(sink.cur.last(), Some(&(0.0, 0.0)));
        crate::glyph::Tracer::close(&mut sink);
        crate::glyph::Tracer::cubic_to(&mut sink, 0.0, 8.0, 0.0, 8.0, 0.0, 0.0);
        assert_eq!(sink.cur.len(), CURVE_STEPS);
        assert_eq!(sink.cur.last(), Some(&(0.0, 0.0)));
        assert_eq!(sink.subs.len(), 1, "the closed quad");
    }

    /// Glyph subpaths of `text` at 20px `family` under `ctm`.
    fn glyphs(text: &str, family: &str, ctm: Matrix) -> Vec<SubPath> {
        let st = State {
            ctm,
            font_px: 20.0,
            font_families: vec![family.to_string()],
            ..State::default()
        };
        glyph_subpaths(&st, text, 2.0, 20.0)
    }

    #[test]
    fn glyph_subpaths_trace_quadratic_and_cubic_faces_into_the_canvas() {
        for family in ["serif", "sans-serif", "no such family"] {
            let subs = glyphs("Ag", family, Matrix::identity());
            assert!(subs.len() >= 2, "{family}: a contour per glyph at least");
            let pts = subs.iter().flat_map(|s| &s.pts);
            assert!(
                pts.clone()
                    .all(|&(x, y)| (0.0..40.0).contains(&x) && (0.0..=26.0).contains(&y)),
                "{family}: inside the 20px run at baseline 20"
            );
            assert!(subs.iter().all(|s| s.closed), "{family}");
        }
        // Under a translation the same outlines move; spaces trace nothing.
        let at = glyphs("A", "serif", Matrix::identity());
        let moved = glyphs("A", "serif", Matrix::identity().translate(10.0, 0.0));
        assert_eq!(moved[0].pts[0], (at[0].pts[0].0 + 10.0, at[0].pts[0].1));
        assert!(glyphs("  ", "serif", Matrix::identity()).is_empty());
        assert!(glyphs("", "serif", Matrix::identity()).is_empty());
    }

    #[test]
    fn a_singular_transform_collapses_text_onto_a_line() {
        // `scale(0, 1)`: every outline point lands on x = 0 (a curve's start, unrecoverable
        // through the singular matrix, falls back to its end point).
        for family in ["serif", "sans-serif"] {
            let subs = glyphs("Ag", family, Matrix::identity().scale(0.0, 1.0));
            assert!(!subs.is_empty(), "{family}");
            assert!(
                subs.iter().flat_map(|s| &s.pts).all(|&(x, _)| x == 0.0),
                "{family}"
            );
        }
    }

    #[test]
    fn font_shorthand_tolerates_whitespace_runs() {
        // Single space (baseline).
        let (size, fams) = parse_font("bold 20px Arial");
        assert_eq!(size, 20.0);
        assert_eq!(fams, vec!["Arial".to_string()]);
        // Double space before the size token must not desync the family tail.
        let (size2, fams2) = parse_font("bold  20px Arial, sans-serif");
        assert_eq!(size2, 20.0);
        assert_eq!(fams2, vec!["Arial".to_string(), "sans-serif".to_string()]);
        // Leading/interior runs, pt→px conversion.
        let (size3, fams3) = parse_font("  italic   12pt   'Times New Roman'");
        assert_eq!(size3, 16.0); // 12pt * 96/72
        assert_eq!(fams3, vec!["Times New Roman".to_string()]);
    }
}
