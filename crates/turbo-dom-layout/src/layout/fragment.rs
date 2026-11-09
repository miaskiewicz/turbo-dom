//! The galley: layout's output (§5.5). A [`Fragment`] tree in one continuous
//! top-down coordinate space (y grows downward, height unbounded). Each fragment
//! carries its source [`NodeId`] so emitted PDF maps back to template nodes
//! (AC-5.11), its painted [`FragmentContent`], and [`BreakMeta`] the fragmenter
//! (Stage 4) reads to decide page breaks.
//!
//! Deferred to later phases (added when first populated): the footnote/running-
//! element payloads on `BreakMeta` (Phases 7/8).

use crate::tag::MarkerKind;
use crate::text::FontFace;

use super::value::{BorderEdges, BreakRule, Rgba};

/// A stable identifier round-tripping a fragment to its source template node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct NodeId(pub u32);

/// One shaped glyph, positioned relative to its `TextLine` fragment's origin so
/// shifting the fragment (e.g. during pagination) never rewrites glyph offsets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PositionedGlyph {
    pub glyph_id: u16,
    pub x: f32,
    pub y: f32,
    /// The byte range of this glyph's source cluster in its line's `text`
    /// (`TextLine::text` / `GlyphRun::text`): the characters the glyph draws,
    /// for a PDF `/ToUnicode` map or `/ActualText` (turbo-dom#205). A ligature
    /// carries its whole cluster; the later glyphs of a multi-glyph cluster
    /// carry an empty range, so the line's glyph texts concatenate to the
    /// shaped text once.
    pub text_start: u32,
    pub text_end: u32,
}

impl PositionedGlyph {
    /// This glyph's cluster text within `line_text` (its `TextLine`'s `text`).
    /// Empty for a continuation glyph or a range outside `line_text`.
    #[must_use]
    pub fn text<'a>(&self, line_text: &'a str) -> &'a str {
        line_text
            .get(self.text_start as usize..self.text_end as usize)
            .unwrap_or("")
    }
}

/// A repeatable table part: re-emitted atop each page a table spans (§6.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepeatKind {
    Header,
    Footer,
}

/// What a fragment paints.
#[derive(Debug, Clone)]
pub enum FragmentContent {
    /// A block/box rectangle: background fill and/or border.
    Box {
        background: Option<Rgba>,
        border: BorderEdges,
        /// The used `border-*-radius` of each corner in px (horizontal and vertical:
        /// elliptical corners), percentages resolved against the border box and scaled
        /// down by the overlap rule at layout (miaskiewicz/turbo-dom#253; it was one
        /// `f32`). [`BorderRadii::ZERO`] = square corners.
        border_radius: BorderRadii,
        /// `box-shadow` (first/topmost layer), painted behind the box by the raster
        /// so overlay cards/modals read as raised chrome. `None` = no shadow.
        shadow: Option<BoxShadow>,
        /// A `linear-gradient(...)` background image, painted over the box (a
        /// gradient hides the `background` colour). `None` = solid/absent background.
        gradient: Option<LinearGradient>,
        /// A CSS `transform` (translate/rotate/scale/matrix): the raster paints this
        /// box AND its whole subtree through the affine matrix, about the box's
        /// `transform-origin`. `None` = untransformed. Carousels/slide decks position
        /// slides via `translate`, so without this they pile up at one spot.
        transform: Option<Transform2D>,
    },
    /// One laid-out line of shaped text.
    TextLine {
        glyphs: Vec<PositionedGlyph>,
        /// The shaped text the glyphs' cluster ranges index (turbo-dom#205).
        text: String,
        face: FontFace,
        font_size: f32,
        color: Rgba,
        /// Laid out in a vertical writing mode (miaskiewicz/turbo-dom#234): the glyph
        /// positions stay in the line's own frame (`x` along the inline axis, `y` down
        /// from the line-over edge) and the run paints turned a quarter clockwise, a
        /// glyph at `(x, y)` landing at `(frag.x + frag.width - y, frag.y + x)`.
        vertical: bool,
        /// The glyphs stand upright (miaskiewicz/turbo-dom#239): in a `vertical` run
        /// each glyph turns back a quarter about its pen, so it paints unturned there.
        upright: bool,
    },
    /// A classified marker element ([`MarkerKind`]) kept opaque for the consumer
    /// (html2pdf's paged-media directives: the fragmenter/emitter read it).
    Marker(MarkerKind),
    /// A raster image (`<img>` or `background-image`, §7.4). The fragment's
    /// `width`/`height` are the painted size (already overflow-capped at layout);
    /// the variant carries only what the emitter needs beyond geometry: the
    /// resolver `name` to fetch the bytes again at emit time, the source's
    /// intrinsic pixel size (diagnostic / round-trip), and whether it carries an
    /// alpha channel (so an `SMask` is emitted).
    Image(ImagePlacement),
    /// A background layer's tiles, all of them (0.7.0 round-2 review, finding 1: one
    /// fragment per tile truncated a fine grid on a tall page). The fragment's box is
    /// the area the tiles are laid out over; painters tile [`TilePattern::tile`] across
    /// it (a pattern shader, an SVG `<pattern>`) or walk [`TilePattern::tiles`] over
    /// the part they paint.
    Tiles(TilePattern),
    /// A multi-column container's `column-rule-*`, painted between every adjacent
    /// pair of column strips (CSS Multicol 1 §5 "The column-rule properties";
    /// miaskiewicz/turbo-dom#398 PR-D). The fragment's `(x, y, width, height)` is
    /// the rule's bounding rect — centered in the `column-gap`, as tall as the
    /// column content box — and the painter draws the chosen [`ColumnRuleStyle`]
    /// in `color` inside it.
    ColumnRule { style: ColumnRuleStyle, color: Rgba },
}

/// A supported `column-rule-style` keyword (CSS Multicol 1 §5.2;
/// miaskiewicz/turbo-dom#398 PR-D, follow-up miaskiewicz/turbo-dom#419 for the
/// 3-D family). A fragment is emitted only for the keywords this enum lists,
/// so the painter never sees one it cannot draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnRuleStyle {
    /// A filled band of `color` filling the rule rect.
    Solid,
    /// Circular dots of diameter `width` centered along the rule, period `2 * width`
    /// (CSS Backgrounds 3 §4.2 "Line styles"; Blink's `kDottedStroke`).
    Dotted,
    /// Rectangular dashes of length `2 * width` separated by `width` gaps
    /// (Blink's `kDashedStroke`).
    Dashed,
    /// Two parallel bands, each `floor(width / 3)` thick, flush with the rule's
    /// long edges, separated by the remaining width (CSS Backgrounds 3 §4.2).
    Double,
    /// Chrome 156 splits a `groove` column-rule into two halves across the
    /// rule's inline axis: the left half paints in [`border_style_colour`]'s
    /// dark shade, the right half in the light shade (miaskiewicz/turbo-dom#419).
    Groove,
    /// `ridge` is `groove` with the two shades swapped (left light, right
    /// dark) — the Chrome 156 fixture for a vertical rule shows `ridge` and
    /// `inset` producing identical pixels.
    Ridge,
    /// For a column-rule (a vertical strip), Chrome 156 paints `inset` as
    /// left half light, right half dark — the same pattern as `ridge`.
    Inset,
    /// `outset` mirrors `inset`: left half dark, right half light — the same
    /// pattern as `groove` for a vertical rule.
    Outset,
}

/// A repeating background tile: [`TilePattern::tile`] painted `tile_width x
/// tile_height` with one tile at `(x, y)` and the others every `step_x` / `step_y` px
/// along each axis (`0` = not repeated on that axis). Each tile snaps to whole pixels
/// (`floor(v + 0.5)` of its edges), so neighbours meet without a seam; tiles are at
/// least 1px.
#[derive(Debug, Clone)]
pub struct TilePattern {
    /// What one tile paints: a background image ([`FragmentContent::Image`]) or a
    /// gradient box ([`FragmentContent::Box`]).
    pub tile: Box<FragmentContent>,
    /// One tile's unsnapped origin (absolute px).
    pub x: f32,
    pub y: f32,
    pub tile_width: f32,
    pub tile_height: f32,
    pub step_x: f32,
    pub step_y: f32,
}

/// `v` snapped to the pixel grid: the same at every whole-pixel offset (unlike
/// `f32::round` on negative halves).
fn snap(v: f32) -> f32 {
    (v + 0.5).floor()
}

/// The starts along one axis of tiles every `step` px from `start` (0: one tile only)
/// that can meet `(from, len)`: from the last one at or before `from`.
pub(crate) fn axis_starts(
    start: f32,
    step: f32,
    (from, len): (f32, f32),
) -> impl Iterator<Item = f32> {
    let (first, count) = if step > 0.0 {
        // The offset reduced modulo the step, so a huge position keeps its phase.
        let first = from - (from - start).rem_euclid(step);
        (first, ((from + len - first) / step).ceil().max(0.0) as u64)
    } else {
        (start, 1)
    };
    (0..count).map(move |i| first + i as f32 * step)
}

impl TilePattern {
    /// The snapped rectangles `(x, y, width, height)` of the tiles that meet `region`
    /// (absolute px), row by row. There are about `region` area / tile area of them.
    pub fn tiles(
        &self,
        region: (f32, f32, f32, f32),
    ) -> impl Iterator<Item = (f32, f32, f32, f32)> + '_ {
        let (rx, ry, rw, rh) = region;
        let meets = move |start: f32, size: f32, from: f32, len: f32| {
            start < from + len && start + size > from
        };
        axis_starts(self.y, self.step_y, (ry, rh))
            .filter(move |&ty| meets(ty, self.tile_height, ry, rh))
            .flat_map(move |ty| {
                axis_starts(self.x, self.step_x, (rx, rw))
                    .filter(move |&tx| meets(tx, self.tile_width, rx, rw))
                    .map(move |tx| {
                        let (x0, y0) = (snap(tx), snap(ty));
                        let w = snap(tx + self.tile_width) - x0;
                        let h = snap(ty + self.tile_height) - y0;
                        (x0, y0, w, h)
                    })
            })
    }

    /// The pattern's period in whole pixels when every tile snaps to the same size the
    /// same distance apart (whole-pixel steps, and whole-pixel tiles on each repeated
    /// axis): `(step_x, step_y)`, `0` on an axis that does not repeat. `None` otherwise
    /// (the snapped tiles alternate sizes, so a painter walks [`TilePattern::tiles`]).
    #[must_use]
    pub fn period(&self) -> Option<(f32, f32)> {
        let whole = |v: f32| v.fract() == 0.0;
        let axis =
            |step: f32, tile: f32| (step == 0.0 || (whole(step) && whole(tile))).then_some(step);
        Some((
            axis(self.step_x, self.tile_width)?,
            axis(self.step_y, self.tile_height)?,
        ))
    }
}

/// A resolved `box-shadow` layer (§CSS backgrounds §7). Geometry is px in the
/// galley's coordinate space; the raster stamps a rounded rect of `color` at the
/// box's outline, offset by `(offset_x, offset_y)`, grown by `spread`, blurred by
/// `blur`. `inset` shadows paint inside the box (v1 paints outer only).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoxShadow {
    pub offset_x: i32,
    pub offset_y: i32,
    pub blur: u32,
    pub spread: i32,
    pub color: Rgba,
    pub inset: bool,
}

/// A resolved CSS 2D `transform`. `matrix` is the CSS `matrix(a,b,c,d,e,f)` the
/// `transform` list multiplies out to (mapping a point `(x,y)` to
/// `(a·x + c·y + e, b·x + d·y + f)`), applied about `(origin_x, origin_y)` — the
/// `transform-origin` as a px offset from the box's own top-left. The raster
/// composes it about the box's absolute position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform2D {
    pub matrix: [f32; 6],
    pub origin_x: f32,
    pub origin_y: f32,
}

/// A CSS `linear-gradient(...)` background (§CSS images §3.1). `angle_deg` is the
/// CSS gradient-line angle (0 = to top, 90 = to right, clockwise); `stops` are the
/// colour stops with positions resolved to a 0..1 fraction of the gradient line,
/// in order. The raster maps this to its own gradient shader.
#[derive(Debug, Clone, PartialEq)]
pub struct LinearGradient {
    pub angle_deg: f32,
    pub stops: Vec<GradientStop>,
}

/// One colour stop of a [`LinearGradient`]: a colour at a 0..1 position along the
/// gradient line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GradientStop {
    pub color: Rgba,
    pub pos: f32,
}

/// A placed raster image: the resolver key plus its intrinsic pixel size and
/// alpha flag (§7.4). The pixels themselves are re-resolved at emit time so the
/// galley stays small and the same bytes feed both layout sizing and embedding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImagePlacement {
    /// The image name passed to the caller's `ImageResolver` (the `<img>` `src`
    /// or the `background-image` `url(...)`).
    pub name: String,
    /// Intrinsic width in px as displayed (after the EXIF `orientation` below).
    pub intrinsic_w: u32,
    /// Intrinsic height in px as displayed.
    pub intrinsic_h: u32,
    /// Whether the source had transparency (drives `SMask` emission).
    pub has_alpha: bool,
    /// When set, the image is a CSS `mask-image`: paint this solid colour everywhere
    /// the source is opaque (using the source's alpha as a stencil) rather than
    /// blitting the source pixels. Tints Wikipedia's monochrome SVG UI glyphs.
    pub tint: Option<Rgba>,
    /// The EXIF orientation the painters apply to the stored pixels to fill the box
    /// (`image-orientation: from-image`; identity under `image-orientation: none` or
    /// when the image carries none) — miaskiewicz/turbo-html2pdf#30.
    pub orientation: crate::image::Orientation,
}

/// One corner's used radii in px: `h` along the horizontal axis, `v` along the vertical
/// one. Either at zero makes the corner square.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CornerRadii {
    pub h: f32,
    pub v: f32,
}

impl CornerRadii {
    /// A circular corner of radius `r`.
    #[must_use]
    pub const fn circle(r: f32) -> CornerRadii {
        CornerRadii { h: r, v: r }
    }

    /// Whether the corner is visibly rounded (both radii over half a pixel).
    #[must_use]
    pub fn is_rounded(self) -> bool {
        self.h > 0.5 && self.v > 0.5
    }
}

/// A box's four used corner radii (CSS Backgrounds 3 §5, as Chrome 156 paints them):
/// physical corners, each elliptical, already scaled so adjacent radii never overlap
/// (miaskiewicz/turbo-dom#253).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BorderRadii {
    pub top_left: CornerRadii,
    pub top_right: CornerRadii,
    pub bottom_right: CornerRadii,
    pub bottom_left: CornerRadii,
}

impl From<f32> for BorderRadii {
    /// Every corner circular with radius `r` (the pre-#253 single scalar).
    fn from(r: f32) -> BorderRadii {
        BorderRadii::uniform(r)
    }
}

impl BorderRadii {
    /// Square corners.
    pub const ZERO: BorderRadii = BorderRadii::uniform(0.0);

    /// Every corner circular with radius `r`.
    #[must_use]
    pub const fn uniform(r: f32) -> BorderRadii {
        let c = CornerRadii::circle(r);
        BorderRadii {
            top_left: c,
            top_right: c,
            bottom_right: c,
            bottom_left: c,
        }
    }

    /// The corners in CSS order: top-left, top-right, bottom-right, bottom-left.
    #[must_use]
    pub fn corners(&self) -> [CornerRadii; 4] {
        [
            self.top_left,
            self.top_right,
            self.bottom_right,
            self.bottom_left,
        ]
    }

    /// The radii from corners in CSS order (see [`Self::corners`]).
    #[must_use]
    pub fn from_corners(
        [top_left, top_right, bottom_right, bottom_left]: [CornerRadii; 4],
    ) -> Self {
        BorderRadii {
            top_left,
            top_right,
            bottom_right,
            bottom_left,
        }
    }

    /// Whether any corner is visibly rounded ([`CornerRadii::is_rounded`]).
    #[must_use]
    pub fn is_rounded(&self) -> bool {
        self.corners().into_iter().any(CornerRadii::is_rounded)
    }

    /// `Some(r)` when every corner is the same circle of radius `r` (what a painter
    /// limited to one radius, such as the pre-#253 `f32`, can draw exactly).
    #[must_use]
    pub fn as_uniform(&self) -> Option<f32> {
        let r = self.top_left.h;
        self.corners()
            .into_iter()
            .all(|c| c == CornerRadii::circle(r))
            .then_some(r)
    }

    /// Scale every radius down by one factor so the radii along each side of a
    /// `width` x `height` box fit it (CSS Backgrounds 3 §5.5 "Overlapping Curves").
    #[must_use]
    pub fn fit(self, width: f32, height: f32) -> BorderRadii {
        let [tl, tr, br, bl] = self.corners();
        let ratio = |side: f32, a: f32, b: f32| {
            if a + b > 0.0 {
                side.max(0.0) / (a + b)
            } else {
                f32::INFINITY
            }
        };
        let f = ratio(width, tl.h, tr.h)
            .min(ratio(width, bl.h, br.h))
            .min(ratio(height, tl.v, bl.v))
            .min(ratio(height, tr.v, br.v));
        if f >= 1.0 {
            return self;
        }
        self.map(|c| CornerRadii {
            h: c.h * f,
            v: c.v * f,
        })
    }

    /// The radii of an edge `top` / `right` / `bottom` / `left` px inside this one (the
    /// padding edge from the border edge, the content edge from the padding edge): each
    /// radius less the adjacent side's inset, floored at zero (CSS Backgrounds 3 §5.2).
    #[must_use]
    pub fn inset(self, top: f32, right: f32, bottom: f32, left: f32) -> BorderRadii {
        let shrink = |c: CornerRadii, h: f32, v: f32| CornerRadii {
            h: (c.h - h).max(0.0),
            v: (c.v - v).max(0.0),
        };
        BorderRadii {
            top_left: shrink(self.top_left, left, top),
            top_right: shrink(self.top_right, right, top),
            bottom_right: shrink(self.bottom_right, right, bottom),
            bottom_left: shrink(self.bottom_left, left, bottom),
        }
    }

    /// Apply `f` to every corner.
    #[must_use]
    pub fn map(self, f: impl Fn(CornerRadii) -> CornerRadii) -> BorderRadii {
        BorderRadii::from_corners(self.corners().map(f))
    }
}

/// The region an `overflow` other than `visible` clips a box's descendants to (CSS
/// Overflow 3 §3): its padding box with the inner corner radii, in px from the fragment's
/// border-box top-left. An axis whose overflow is `visible` (beside `clip`) is not clipped:
/// its extent is unbounded (miaskiewicz/turbo-dom#253).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OverflowClip {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub radii: BorderRadii,
}

/// The region an absolutely positioned box's `clip: rect()` keeps (CSS 2.1 §11.1.2), in
/// px from the fragment's border-box top-left (so it moves with the fragment). The box
/// and every descendant fragment paint only inside it; it may be empty (paints nothing)
/// and may extend past the border box (miaskiewicz/turbo-dom#52).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClipRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    /// The region's corner radii ([`BorderRadii::ZERO`] = square): a background layer
    /// clipped to a rounded box's clip area (miaskiewicz/turbo-dom#248; per corner and
    /// elliptical since #253).
    pub radii: BorderRadii,
}

/// Break hints attached to a fragment, consumed by the fragmenter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BreakMeta {
    pub break_before: BreakRule,
    pub break_after: BreakRule,
    pub break_inside_avoid: bool,
    pub orphans: u8,
    pub widows: u8,
    pub repeatable: Option<RepeatKind>,
    /// Indices into the document's footnote list owned by this fragment (§6.4):
    /// the footnote markers it carries, in document order. The fragmenter reads
    /// this to learn which notes a page references once the fragment lands, so it
    /// can reserve their measured area. Empty for everything but footnote markers.
    pub footnotes: Vec<usize>,
}

impl Default for BreakMeta {
    fn default() -> Self {
        BreakMeta {
            break_before: BreakRule::Auto,
            break_after: BreakRule::Auto,
            break_inside_avoid: false,
            orphans: 2,
            widows: 2,
            repeatable: None,
            footnotes: Vec::new(),
        }
    }
}

/// A positioned, sized fragment in the galley.
#[derive(Debug, Clone)]
pub struct Fragment {
    pub node_id: NodeId,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub content: FragmentContent,
    pub break_meta: BreakMeta,
    pub children: Vec<Fragment>,
    /// The box's used `z-index` (`None` = `auto`), meaningful only when
    /// [`Fragment::is_positioned`]. Drives stacking-context paint order (§9.9);
    /// see [`Fragment::paint_z`]/[`Fragment::paint_order`].
    pub z_index: Option<i32>,
    /// Whether this fragment's box is positioned (`position` != `static`). A
    /// positioned box paints above its in-flow siblings within the same stacking
    /// context, and (with a `z-index`) participates in z ordering.
    pub is_positioned: bool,
    /// The box's `clip: rect()` region (`None` = unclipped): painters clip this fragment
    /// and its subtree to it.
    pub clip: Option<ClipRect>,
    /// The region the box's `overflow` clips its descendants to (`None` = `visible`):
    /// painters clip the children, not the box's own background and border
    /// (miaskiewicz/turbo-dom#253).
    pub overflow_clip: Option<OverflowClip>,
    /// The box's CSS `opacity` (`0..=1`, `1` = opaque): painters composite this
    /// fragment and its whole subtree as one group at this alpha. Below `1` it also
    /// makes a stacking context ([`Fragment::stacks_as_positioned`]).
    pub opacity: f32,
}

impl Fragment {
    /// A fragment with default break metadata and no children.
    #[must_use]
    pub fn new(
        node_id: NodeId,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        content: FragmentContent,
    ) -> Fragment {
        Fragment {
            node_id,
            x,
            y,
            width,
            height,
            content,
            break_meta: BreakMeta::default(),
            children: Vec::new(),
            z_index: None,
            is_positioned: false,
            clip: None,
            overflow_clip: None,
            opacity: 1.0,
        }
    }

    /// The bottom edge of this fragment (`y + height`).
    #[must_use]
    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }

    /// The effective stacking level used to order this fragment against its
    /// siblings when painting: the used `z-index` for a positioned box, else `0`
    /// (an in-flow box stacks at level 0, ignoring any stray `z-index`).
    #[must_use]
    pub fn paint_z(&self) -> i32 {
        if self.is_positioned {
            self.z_index.unwrap_or(0)
        } else {
            0
        }
    }

    /// Whether this box paints in its stacking context's `z-index: auto`/`0` layer
    /// (CSS 2.2 §9.9 / Appendix E step 8) rather than with the in-flow boxes: a
    /// positioned box, or one whose `opacity < 1` or `transform` makes it a stacking
    /// context (Chrome 156 paints such a box over an earlier `position: relative`
    /// sibling; miaskiewicz/turbo-dom#206).
    #[must_use]
    pub fn stacks_as_positioned(&self) -> bool {
        let transformed = matches!(
            &self.content,
            FragmentContent::Box {
                transform: Some(_),
                ..
            }
        );
        self.is_positioned || self.opacity < 1.0 || transformed
    }

    /// This fragment's child indices in back-to-front paint order (CSS 2.2 §9.9,
    /// a pragmatic subset): a *stable* sort by `(paint_z, stacks_as_positioned)`, so
    /// within one stacking context negative-`z` boxes paint first, then in-flow
    /// non-positioned boxes, then `z:auto`/`0` positioned (or `opacity`/`transform`
    /// stacking-context) boxes, then positive-`z`
    /// — with source (DOM) order breaking ties. The children `Vec` itself keeps
    /// its layout (top-down galley) order so the paginator's flow walk is
    /// unaffected; only painters reorder via this view.
    #[must_use]
    pub fn paint_order(&self) -> Vec<usize> {
        let mut order: Vec<usize> = (0..self.children.len()).collect();
        order.sort_by_key(|&i| {
            let c = &self.children[i];
            (c.paint_z(), c.stacks_as_positioned())
        });
        order
    }

    /// Shift this fragment and its whole subtree by `(dx, dy)`.
    pub fn translate(&mut self, dx: f32, dy: f32) {
        self.x += dx;
        self.y += dy;
        // A tile pattern's origin is absolute too (miaskiewicz/turbo-dom#274).
        if let FragmentContent::Tiles(pattern) = &mut self.content {
            pattern.x += dx;
            pattern.y += dy;
        }
        for child in &mut self.children {
            child.translate(dx, dy);
        }
    }
}
