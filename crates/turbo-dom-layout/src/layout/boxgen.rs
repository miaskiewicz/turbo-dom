//! Box generation (§5.1): the styled tree becomes a box tree. CSS distinguishes
//! block-level and inline-level boxes; a block container holding a mix wraps each
//! inline run in an *anonymous block* so its children are uniformly block-level
//! (AC-5.1). `display:none` is dropped; `t:` directives become opaque markers the
//! fragmenter/emitter handle later.
//!
//! Boxes keep their [`ComputedStyle`] rather than a resolved `BoxStyle`: `%`
//! widths/margins need a containing-block width that is only known during layout,
//! so metric resolution is deferred to the block/inline/flex/table passes. Each
//! box gets a pre-order [`NodeId`] for round-tripping back to its template node.

use std::cell::RefCell;

use crate::layout_snapshot::sources::{BuildSources, SourceRecord};

use crate::style::{ComputedStyle, StyledElement, StyledNode, StyledText};
use crate::tag::{Attr, MarkerKind, Tag};

use super::fragment::NodeId;
use super::value::{
    display_of, is_ctx_independent, resolve_box_style, BoxStyle, Display, ResolveCtx,
};

/// A box in the layout tree.
#[derive(Debug, Clone)]
pub struct LayoutBox {
    pub node_id: NodeId,
    source_role: SourceRole,
    pub style: ComputedStyle,
    /// Source element attributes (e.g. `colspan`, `href`); empty for anonymous
    /// boxes. Layout/emit read HTML attributes that are not CSS properties here.
    pub attrs: Vec<Attr>,
    pub display: Display,
    pub kind: BoxKind,
    /// A raster image this box paints (§7.4): a replaced `<img>` (which also
    /// sizes the box) or a `background-image` (painted behind the box content).
    pub image: Option<ImageSource>,
    pub(crate) svg_viewport: Option<Box<super::svg_viewport::SvgViewport>>,
    /// A `mask-image: url(...)` (or `-webkit-mask-image`): the icon technique where
    /// an SVG is used as an alpha mask and the box's paint (its `background-color`,
    /// falling back to `color`) shows through only where the mask is opaque. Ubiquitous
    /// for Wikipedia's UI glyphs (TOC carets, search/eye/pencil). The box paints as a
    /// tinted mask instead of a solid rectangle.
    pub mask: Option<String>,
    /// Memoized style resolution. Layout resolves each box's `BoxStyle` several
    /// times (max-content measurement, then placement); when the metrics cannot
    /// vary with the containing block, the first resolution is reused instead of
    /// re-parsing the ~25 properties. The (one-time) independence classification
    /// is also cached so context-*dependent* boxes never re-scan their values.
    style_cache: RefCell<StyleCache>,
    // Valid native scales are positive; zero retains the generic/no-scale policy.
    html_layout_policy: std::cell::Cell<(bool, f32)>,
    /// Memoized intrinsic widths (max-content, min-content). Both are pure
    /// functions of the box subtree + fonts (no layout context), but the flex/table
    /// sizers call them repeatedly and RE-descend the subtree each time, so a deeply
    /// nested tree measured naively is superlinear. Cache the first result per box.
    natural_cache: std::cell::Cell<Option<f32>>,
    min_content_cache: std::cell::Cell<Option<f32>>,
    /// A replaced `<img>`'s intrinsic pixel width, stamped once from the image
    /// resolver before layout ([`crate::layout::stamp_intrinsic_widths`]). The
    /// intrinsic size needs the decoded bytes (only the layout entry has the
    /// resolver), but `natural_width`/`min_content_width` — which run deep in the
    /// flex/table sizers without a resolver — must see it so a `<div>` wrapping an
    /// `<img>` (a logo) measures the image's width instead of collapsing to 0.
    pub(crate) intrinsic_w: std::cell::Cell<Option<f32>>,
    /// Memoized flex/grid measurement: the content `(width, height)` this box lays
    /// out to at a given proposed width. taffy probes each item's size several times
    /// per solve (min/max-content, then the resolved width) and each probe does a
    /// FULL sub-layout, so a deeply nested flex tree is exponential without this —
    /// keyed by the rounded proposed width (a handful of distinct values per box).
    measure_cache: RefCell<Option<Box<MeasuredSizes>>>,
    /// Memoized margin-collapsing walks ([`MarginMemo`], miaskiewicz/turbo-dom#202).
    margin_memo: MarginMemoSlot,
}

fn same_original_inputs(
    a: Option<&std::sync::Arc<crate::layout_snapshot::text_units::NativeUnitMap>>,
    b: Option<&std::sync::Arc<crate::layout_snapshot::text_units::NativeUnitMap>>,
) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => std::sync::Arc::ptr_eq(a, b),
        _ => false,
    }
}

/// One-entry memos of the margin-collapsing walks of `block.rs` (miaskiewicz/turbo-dom#202).
/// Each walk descends the first- or last-child chain, and every level of a nested
/// document calls it again, so a `d`-deep chain cost O(d^2) style resolves. Each memo
/// holds the last `(key, value)`, the key packing the walk's containing-block width and
/// one style length as bits. The leading-margin and self-collapsing walks are pure
/// functions of the subtree and the key, so they are dropped only when the layout policy
/// changes; the trailing margin also reads the dropped end margins of the latest layout,
/// so it is dropped whenever the box is laid out again ([`LayoutBox::begin_layout`]).
/// A clone (flex / table temporary boxes) starts empty.
#[derive(Debug, Default)]
struct MarginMemo {
    leading: std::cell::Cell<Option<(u64, f32)>>,
    kids_collapse: std::cell::Cell<Option<(u64, bool)>>,
    trailing: std::cell::Cell<Option<(u64, f32)>>,
}

impl MarginMemo {
    fn clear(&self) {
        self.leading.set(None);
        self.kids_collapse.set(None);
        self.trailing.set(None);
    }
}

/// A box's lazily allocated [`MarginMemo`]; a clone (flex / table temporary boxes)
/// starts empty.
#[derive(Debug, Default)]
pub(crate) struct MarginMemoSlot(std::cell::OnceCell<Box<MarginMemo>>);

impl Clone for MarginMemoSlot {
    fn clone(&self) -> Self {
        MarginMemoSlot::default()
    }
}

/// The memo key for a walk at containing-block width `cw` with style length `extra`.
pub(crate) fn memo_key(cw: f32, extra: f32) -> u64 {
    (u64::from(cw.to_bits()) << 32) | u64::from(extra.to_bits())
}

fn memoized<T: Copy>(
    cell: &std::cell::Cell<Option<(u64, T)>>,
    key: u64,
    f: impl FnOnce() -> T,
) -> T {
    if let Some((k, v)) = cell.get() {
        if k == key {
            return v;
        }
    }
    let v = f();
    cell.set(Some((key, v)));
    v
}

// Lazily boxed so optional viewport data does not enlarge the public inline
// item enum or force allocations for boxes that never need measurement caching.
#[derive(Debug, Clone)]
struct MeasuredSizes {
    original_units: Option<std::sync::Arc<crate::layout_snapshot::text_units::NativeUnitMap>>,
    values: Vec<(u32, (f32, f32))>,
}

/// Per-box cache of the resolved style and its context-independence verdict.
#[derive(Debug, Clone)]
enum StyleCache {
    /// Not yet classified — resolve and decide on first use.
    Unknown,
    /// Style depends on the layout context; always re-resolve (no value cached).
    Dependent,
    /// Style is context-independent; this resolution is reused for every `ctx`.
    /// Boxed to keep the enum small (a `BoxStyle` dwarfs the unit variants).
    Cached(Box<BoxStyle>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceRole {
    Ordinary,
    DocumentRoot,
    NativeIframe,
    AnonymousInlineFlow,
    /// The anonymous table grid box of a captioned table (miaskiewicz/turbo-dom#51).
    TableGrid,
}

impl LayoutBox {
    pub(crate) fn anonymous_inline_flow(&self) -> bool {
        self.source_role == SourceRole::AnonymousInlineFlow
    }

    /// Whether this is the anonymous table grid box of a table with captions: the
    /// table's own box is then the table wrapper box, holding the captions and this grid
    /// (CSS Tables 3 §3.1; miaskiewicz/turbo-dom#51).
    pub(crate) fn table_grid(&self) -> bool {
        self.source_role == SourceRole::TableGrid
    }

    pub(crate) fn document_root(&self) -> bool {
        self.source_role == SourceRole::DocumentRoot
    }
    pub(crate) fn native_iframe(&self) -> bool {
        self.source_role == SourceRole::NativeIframe
    }

    /// Whether the box is absolutely positioned (`absolute` / `fixed`): out of flow,
    /// so inline content around it is neither split nor moved by it.
    pub(crate) fn out_of_flow(&self) -> bool {
        super::value::position_of(&self.style).is_out_of_flow()
    }

    /// Whether the box floats (`float: left | right`; an absolutely positioned box
    /// ignores `float`, see [`LayoutBox::out_of_flow`]).
    pub(crate) fn floated(&self) -> bool {
        super::value::float_of(&self.style) != super::value::Float::None
    }

    /// Whether the box takes only a place in surrounding inline content instead of
    /// splitting it: an absolutely positioned box (its static position) or a float
    /// (placed at the top of its line; miaskiewicz/turbo-dom#28 item 4).
    pub(crate) fn line_placeholder(&self) -> bool {
        self.out_of_flow() || self.floated()
    }

    pub(crate) fn is_replaced(&self) -> bool {
        self.native_iframe()
            || self.svg_viewport.is_some()
            || self.image.as_ref().is_some_and(|source| source.replaced)
    }

    /// This box's margin memo, allocated on first use (boxed so inline items stay small).
    fn margin_memo(&self) -> &MarginMemo {
        self.margin_memo.0.get_or_init(Box::default)
    }

    /// The memoized leading (collapsed top) margin for `key` ([`MarginMemo`]).
    pub(crate) fn leading_margin_cached(&self, key: u64, f: impl FnOnce() -> f32) -> f32 {
        memoized(&self.margin_memo().leading, key, f)
    }

    /// Whether all in-flow children are self-collapsing, memoized for `key`.
    pub(crate) fn kids_collapse_cached(&self, key: u64, f: impl FnOnce() -> bool) -> bool {
        memoized(&self.margin_memo().kids_collapse, key, f)
    }

    /// The memoized trailing (collapse-through bottom) margin for `key`.
    pub(crate) fn trailing_margin_cached(&self, key: u64, f: impl FnOnce() -> f32) -> f32 {
        memoized(&self.margin_memo().trailing, key, f)
    }

    /// Drop what this box's previous layout memoized (its trailing margin reads the
    /// dropped end margins that this layout records anew).
    pub(crate) fn begin_layout(&self) {
        if let Some(memo) = self.margin_memo.0.get() {
            memo.trailing.set(None);
        }
    }

    /// The memoized max-content width, computing it once via `f` on a miss.
    pub(crate) fn natural_cached(&self, f: impl FnOnce() -> f32) -> f32 {
        if let Some(w) = self.natural_cache.get() {
            w
        } else {
            let w = f();
            self.natural_cache.set(Some(w));
            w
        }
    }

    /// The memoized min-content width, computing it once via `f` on a miss.
    pub(crate) fn min_content_cached(&self, f: impl FnOnce() -> f32) -> f32 {
        if let Some(w) = self.min_content_cache.get() {
            w
        } else {
            let w = f();
            self.min_content_cache.set(Some(w));
            w
        }
    }

    /// The memoized measured content size at proposed width `w`, computing it once
    /// per distinct (rounded) width via `f`.
    pub(crate) fn measure_cached(&self, w: f32, f: impl FnOnce() -> (f32, f32)) -> (f32, f32) {
        let key = w.round().clamp(0.0, u32::MAX as f32) as u32;
        // Scope the immutable borrow to this `let` so it is dropped before `f()` (a
        // recursive sub-layout) or the `borrow_mut` below can run.
        let hit = self.measure_cache.borrow().as_ref().and_then(|cache| {
            cache
                .values
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, size)| *size)
        });
        if let Some(size) = hit {
            return size;
        }
        let size = f();
        let mut storage = self.measure_cache.borrow_mut();
        let cache = storage.get_or_insert_with(|| {
            Box::new(MeasuredSizes {
                values: Vec::new(),
                original_units: None,
            })
        });
        cache.values.push((key, size));
        size
    }

    /// Resolve this box's [`BoxStyle`] for `ctx`, reusing the cached resolution
    /// when the style is context-independent.
    pub(crate) fn resolved(&self, ctx: ResolveCtx) -> BoxStyle {
        let style = self.resolved_base(ctx);
        let mut style = match &self.svg_viewport {
            Some(viewport) => viewport.apply_attributes(style),
            None => style,
        };
        super::value::quantize_html_box_style(&mut style, self.native_scale());
        style
    }

    fn native_scale(&self) -> Option<f32> {
        let scale = self.html_layout_policy.get().1;
        (scale != 0.0).then_some(scale)
    }
    pub(crate) fn uses_html_font_metrics(&self) -> bool {
        self.html_layout_policy.get().0
    }
    pub(crate) fn set_html_layout_policy(&self, policy: (bool, Option<f32>)) {
        let encoded = (policy.0, policy.1.unwrap_or(0.0));
        if self.html_layout_policy.replace(encoded) != encoded {
            self.natural_cache.set(None);
            self.min_content_cache.set(None);
            self.clear_measured_values();
            if let Some(memo) = self.margin_memo.0.get() {
                memo.clear();
            }
        }
        self.set_child_html_layout_policy(policy);
    }

    pub(crate) fn original_units(
        &self,
    ) -> Option<std::sync::Arc<crate::layout_snapshot::text_units::NativeUnitMap>> {
        self.measure_cache
            .borrow()
            .as_ref()
            .and_then(|cache| cache.original_units.clone())
    }
    pub(crate) fn set_original_units(
        &self,
        units: Option<&std::sync::Arc<crate::layout_snapshot::text_units::NativeUnitMap>>,
    ) {
        let changed = !same_original_inputs(self.original_units().as_ref(), units);
        if changed {
            let mut storage = self.measure_cache.borrow_mut();
            let cache = storage.get_or_insert_with(|| {
                Box::new(MeasuredSizes {
                    values: Vec::new(),
                    original_units: None,
                })
            });
            cache.original_units = units.cloned();
            drop(storage);
            self.natural_cache.set(None);
            self.min_content_cache.set(None);
            self.clear_measured_values();
            if let Some(memo) = self.margin_memo.0.get() {
                memo.clear();
            }
        }
        self.set_child_original_units(units);
    }
    fn clear_measured_values(&self) {
        let mut storage = self.measure_cache.borrow_mut();
        if storage
            .as_ref()
            .is_some_and(|cache| cache.original_units.is_none())
        {
            *storage = None;
        } else if let Some(cache) = storage.as_mut() {
            cache.values.clear();
        }
    }
    fn set_child_original_units(
        &self,
        units: Option<&std::sync::Arc<crate::layout_snapshot::text_units::NativeUnitMap>>,
    ) {
        match &self.kind {
            BoxKind::Block(children)
            | BoxKind::Flex(children)
            | BoxKind::Grid(children)
            | BoxKind::Table(children) => {
                for child in children {
                    child.set_original_units(units);
                }
            }
            BoxKind::Lines(items) => {
                for item in items {
                    if let InlineItem::Atomic(child) = item {
                        child.set_original_units(units);
                    }
                }
            }
            BoxKind::Marker(_) => {}
        }
    }

    fn set_child_html_layout_policy(&self, policy: (bool, Option<f32>)) {
        match &self.kind {
            BoxKind::Block(children)
            | BoxKind::Flex(children)
            | BoxKind::Grid(children)
            | BoxKind::Table(children) => {
                for child in children {
                    child.set_html_layout_policy(policy);
                }
            }
            BoxKind::Lines(items) => {
                for item in items {
                    if let InlineItem::Atomic(child) = item {
                        child.set_html_layout_policy(policy);
                    }
                }
            }
            BoxKind::Marker(_) => {}
        }
    }

    pub(crate) fn resolved_base(&self, ctx: ResolveCtx) -> BoxStyle {
        match &*self.style_cache.borrow() {
            StyleCache::Cached(bs) => return bs.as_ref().clone(),
            StyleCache::Dependent => return resolve_box_style(&self.style, ctx),
            StyleCache::Unknown => {}
        }
        self.resolve_and_classify(ctx)
    }

    /// First-use path: resolve once, then remember whether the result can be
    /// reused for any context.
    fn resolve_and_classify(&self, ctx: ResolveCtx) -> BoxStyle {
        let bs = resolve_box_style(&self.style, ctx);
        *self.style_cache.borrow_mut() = if is_ctx_independent(&self.style) {
            StyleCache::Cached(Box::new(bs.clone()))
        } else {
            StyleCache::Dependent
        };
        bs
    }
}

/// A raster image referenced by a box: the resolver name plus whether it is the
/// box's *replaced content* (an `<img>`, which drives the box size) or a
/// `background-image` (sized by the box, painted behind it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageSource {
    pub name: String,
    pub replaced: bool,
}

/// What a box contains, by formatting context.
#[derive(Debug, Clone)]
pub enum BoxKind {
    /// A block container whose children are all block-level (anonymous blocks
    /// already inserted around inline runs).
    Block(Vec<LayoutBox>),
    /// A block container establishing an inline formatting context: a paragraph
    /// of inline items laid out into line boxes.
    Lines(Vec<InlineItem>),
    /// A flex container; children are flex items.
    Flex(Vec<LayoutBox>),
    /// A grid container; children are grid items (auto-placed unless they carry
    /// explicit `grid-row`/`grid-column` lines).
    Grid(Vec<LayoutBox>),
    /// A table; children are rows/row-groups (interpreted by `table.rs` via
    /// each child's `display`).
    Table(Vec<LayoutBox>),
    /// An opaque block-level marker element (see [`MarkerKind`]).
    Marker(MarkerKind),
}

/// One inline-level item inside a [`BoxKind::Lines`] context.
#[derive(Debug, Clone)]
pub enum InlineItem {
    SourceBreak {
        node_id: NodeId,
        style: ComputedStyle,
    },
    /// A real native HTML inline boundary, retained before flattening text.
    Boundary {
        node_id: NodeId,
        style: ComputedStyle,
        start: bool,
    },
    /// A run of text, styled by its containing element.
    Text {
        node_id: NodeId,
        style: ComputedStyle,
        text: String,
    },
    /// An atomic inline box (`inline-block`, or a block nested in inline flow).
    Atomic(LayoutBox),
    /// A forced line break (`<br>`): it ends the current line and continues on the
    /// next (browser-faithful inline behavior). It carries no metrics of its own —
    /// the line-height in effect comes from the line's content and the block strut.
    LineBreak,
    /// An inline marker element (e.g. html2pdf's footnote reference).
    Marker { node_id: NodeId, kind: MarkerKind },
}

/// A block-level box or an inline-level run, before anonymous-block wrapping.
// A transient classification value moved straight into the box tree; the big
// `Block` variant holds the actual `LayoutBox`, so boxing it would just add an
// allocation on the hot construction path for no real gain.
#[allow(clippy::large_enum_variant)]
enum Level {
    Block(LayoutBox),
    Inline(Vec<InlineItem>),
    Split(Vec<FlowLevel>),
}

/// Classification after native splits have been lifted into their containing flow.
#[allow(clippy::large_enum_variant)]
enum FlowLevel {
    Block(LayoutBox),
    Inline(Vec<InlineItem>),
}

/// Box-generation options (the layout-level switches of [`super::LayoutOptions`]).
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct BuildOptions {
    /// Lay an inline `<a>` with an internal (`#name`) or absolute-URL href out as an
    /// atomic inline box (its own fragment); block-level links keep their block box.
    pub(crate) atomic_internal_links: bool,
    /// Record each box's source [`turbo_dom::Handle`] (see [`super::NodeSources`]).
    pub(crate) record_sources: bool,
    /// Keep an absolutely positioned box whose `clip: rect()` keeps nothing (the
    /// sr-only pattern; [`super::value::clip_keeps_nothing`]) in the box tree: it paints
    /// nothing, so a render leaves it out, but the geometry seam
    /// ([`crate::layout_boxes`]) reports its border box like Chrome
    /// (miaskiewicz/turbo-dom#75).
    pub(crate) keep_zero_area_clips: bool,
}

/// The source handles recorded per box id in source-recording mode.
#[derive(Debug, Default)]
pub(crate) struct Recorded {
    pub(crate) elements: std::collections::HashMap<NodeId, turbo_dom::Handle>,
    pub(crate) texts: std::collections::HashMap<NodeId, turbo_dom::Handle>,
    /// The anonymous table grid boxes ([`LayoutBox::table_grid`]).
    pub(crate) table_grids: std::collections::HashSet<NodeId>,
}

/// A monotonic source of pre-order node ids.
struct Ids {
    next: u32,
    /// Snapshot mode (live geometry): the snapshot's source identities. Also
    /// switches the snapshot-only box-generation rules (hidden boxes, markers,
    /// anonymous styles).
    sources: Option<BuildSources>,
    document_roots: std::collections::HashSet<usize>,
    /// Source-recording mode — independent of snapshot mode: it only records the
    /// [`StyledElement::source`]/text sources per allocated id.
    recorded: Option<Recorded>,
    atomic_internal_links: bool,
    keep_zero_area_clips: bool,
    /// Whether a box in the top layer (the fullscreen element, miaskiewicz/turbo-dom#82)
    /// was built: [`build_root`] then hoists it.
    top_layer: bool,
}

impl Ids {
    fn new(sources: Option<BuildSources>, options: BuildOptions) -> Ids {
        Ids {
            next: 0,
            sources,
            document_roots: std::collections::HashSet::default(),
            recorded: options.record_sources.then(Recorded::default),
            atomic_internal_links: options.atomic_internal_links,
            keep_zero_area_clips: options.keep_zero_area_clips,
            top_layer: false,
        }
    }

    fn alloc(&mut self) -> NodeId {
        let id = NodeId(self.next);
        self.next += 1;
        id
    }

    fn sourced(&mut self, key: usize) -> NodeId {
        let id = self.alloc();
        if let Some(sources) = &mut self.sources {
            sources.record(id, key);
        }
        id
    }

    fn element(&mut self, element: &StyledElement) -> NodeId {
        let id = self.sourced(std::ptr::from_ref::<StyledElement>(element) as usize);
        if let (Some(recorded), Some(source)) = (&mut self.recorded, element.source) {
            recorded.elements.insert(id, source);
        }
        id
    }

    fn text(&mut self, node: &StyledNode, text: &StyledText) -> NodeId {
        let id = self.sourced(std::ptr::from_ref::<StyledNode>(node) as usize);
        if let (Some(recorded), Some(source)) = (&mut self.recorded, text.source) {
            recorded.texts.insert(id, source);
        }
        id
    }

    fn is_svg_viewport(&self, element: &StyledElement) -> bool {
        self.sources.as_ref().is_some_and(|sources| {
            sources
                .svg_viewports
                .contains(&(std::ptr::from_ref(element) as usize))
        })
    }

    fn hidden(&self, style: &ComputedStyle) -> bool {
        if self.sources.is_some() {
            display_of(style) == Display::None
        } else {
            is_hidden(style, self.keep_zero_area_clips)
        }
    }
}

fn is_marker(el: &StyledElement) -> bool {
    matches!(el.tag, Tag::Marker(_))
}

fn text_item(
    node: &StyledNode,
    text: &StyledText,
    style: &ComputedStyle,
    ids: &mut Ids,
) -> InlineItem {
    InlineItem::Text {
        node_id: ids.text(node, text),
        style: style.clone(),
        text: text.text.clone(),
    }
}

// --------------------------------------------------------------------------
// classification
// --------------------------------------------------------------------------

fn marker_level(kind: MarkerKind, el: &StyledElement, ids: &mut Ids) -> Level {
    if kind.inline {
        Level::Inline(vec![InlineItem::Marker {
            node_id: ids.element(el),
            kind,
        }])
    } else {
        Level::Block(build_block_box(el, ids))
    }
}

/// Whether a box (and its subtree) is not rendered: `display:none`, or a visually
/// hidden / screen-reader-only pattern ([`is_visually_hidden`]). `visibility:hidden`
/// and `opacity:0` boxes are NOT dropped: like Chrome they keep their layout box and
/// only paint nothing (miaskiewicz/turbo-dom#209, [`super::invisible`]); a hover- or
/// click-revealed menu is usually absolutely positioned, so it takes no flow space.
/// `keep_zero_area_clips`: a box whose `clip: rect()` keeps nothing is not dropped for
/// it ([`BuildOptions`]).
pub(crate) fn is_hidden(style: &ComputedStyle, keep_zero_area_clips: bool) -> bool {
    if matches!(display_of(style), Display::None) {
        return true;
    }
    is_visually_hidden(style, keep_zero_area_clips)
}

/// The "visually hidden" / screen-reader-only patterns: content that's in the DOM
/// (for a11y) but clipped away so it never paints. Real pages (Wikipedia, Nike)
/// use dozens; without honoring them their text renders — and, being
/// `position:absolute` with no offset, piles up at the containing block's origin.
fn is_visually_hidden(style: &ComputedStyle, keep_zero_area_clips: bool) -> bool {
    // An absolutely positioned box whose `clip: rect()` keeps nothing — the classic
    // sr-only clip (`rect(1px,1px,1px,1px)` / `rect(0,0,0,0)`). A partial clip lays out
    // and paints its region (the fragment's `clip`); `clip` on any other box does not
    // apply (miaskiewicz/turbo-dom#52). Kept for geometry, such a box is laid out with
    // its (empty) clip region, whatever other hiding pattern it also matches (the
    // sr-only class is also 1px with clipped overflow; miaskiewicz/turbo-dom#75).
    if super::value::clip_keeps_nothing(style) {
        return !keep_zero_area_clips;
    }
    clip_path_hides(style) || tiny_clipped(style)
}

/// `clip-path: inset(50%|100%)` clips the whole box away.
fn clip_path_hides(style: &ComputedStyle) -> bool {
    style
        .get("clip-path")
        .map(str::trim)
        .is_some_and(|cp| cp.contains("inset(") && (cp.contains("50%") || cp.contains("100%")))
}

/// A 0/1px box with clipped overflow — the modern visually-hidden pattern.
fn tiny_clipped(style: &ComputedStyle) -> bool {
    let tiny = |p: &str| matches!(style.get(p).map(str::trim), Some("0" | "0px" | "1px"));
    let overflow_hidden = ["overflow", "overflow-x", "overflow-y"]
        .iter()
        .any(|p| style.get(p).map(str::trim) == Some("hidden"));
    tiny("width") && tiny("height") && overflow_hidden
}

/// Whether the element is a `<br>` — an inline forced line break, handled by the
/// inline layout, not a box (it has no `display`/`height` of its own).
fn is_br(el: &StyledElement) -> bool {
    matches!(&el.tag, Tag::Html(n) if n == "br")
}

fn line_break(el: &StyledElement, ids: &mut Ids) -> InlineItem {
    if ids
        .sources
        .as_ref()
        .is_some_and(|sources| sources.inline_borders)
    {
        InlineItem::SourceBreak {
            node_id: ids.element(el),
            style: el.style.clone(),
        }
    } else {
        InlineItem::LineBreak
    }
}
fn classify_html(el: &StyledElement, ids: &mut Ids) -> Option<Level> {
    if ids.hidden(&el.style) {
        return None;
    }
    if is_br(el) {
        return Some(Level::Inline(vec![line_break(el, ids)]));
    }
    if atomic_link(el, ids) {
        // An inline `<a>` link (`#name` or absolute URL) is an atomic inline box so it carries
        // its own fragment (and thus a link rectangle) through layout (AC-3.25).
        return Some(Level::Inline(vec![InlineItem::Atomic(build_block_box(
            el, ids,
        ))]));
    }
    classify_display(el, ids)
}

/// Whether `el` is an internal link laid out atomically: the option is on and the
/// tree is not a native-inline snapshot.
fn atomic_link(el: &StyledElement, ids: &Ids) -> bool {
    ids.atomic_internal_links
        && display_of(&el.style) == Display::Inline
        && is_link(el)
        && !ids
            .sources
            .as_ref()
            .is_some_and(|sources| sources.inline_borders)
}

/// An `<a>` whose href is an internal `#name` (non-empty fragment) or an absolute
/// URL (`scheme:…`): the links html2pdf annotates (miaskiewicz/turbo-dom#210).
fn is_link(el: &StyledElement) -> bool {
    matches!(&el.tag, Tag::Html(name) if name == "a")
        && attr_value(&el.attrs, "href").is_some_and(|href| {
            href.strip_prefix('#')
                .map_or_else(|| has_scheme(href), |target| !target.is_empty())
        })
}

/// Whether `href` starts with a URL scheme (`ALPHA *( ALPHA / DIGIT / "+" / "-" / "." ) ":"`).
fn has_scheme(href: &str) -> bool {
    href.split_once(':').is_some_and(|(scheme, _)| {
        scheme.starts_with(|c: char| c.is_ascii_alphabetic())
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
    })
}

#[allow(clippy::unnecessary_wraps)] // keeps the Option/Result shape its call sites (and sibling helpers) share
fn classify_display(el: &StyledElement, ids: &mut Ids) -> Option<Level> {
    if native_out_of_flow(el, ids) {
        return Some(Level::Block(build_block_box(el, ids)));
    }
    // `display:none` was already dropped by the `is_hidden` guard above, so it can
    // never reach here — no `Display::None` arm (it would be dead code).
    match display_of(&el.style) {
        Display::Inline => Some(native_inline_items(el, ids)),
        Display::InlineBlock | Display::InlineTable | Display::InlineFlex | Display::InlineGrid => {
            Some(Level::Inline(vec![InlineItem::Atomic(build_block_box(
                el, ids,
            ))]))
        }
        _ => Some(Level::Block(build_block_box(el, ids))),
    }
}

fn native_out_of_flow(element: &StyledElement, ids: &Ids) -> bool {
    ids.sources
        .as_ref()
        .is_some_and(|sources| sources.inline_borders)
        && matches!(element.style.get("position"), Some("absolute" | "fixed"))
}

fn native_inline_items(el: &StyledElement, ids: &mut Ids) -> Level {
    let native = ids
        .sources
        .as_ref()
        .is_some_and(|sources| sources.inline_borders);
    if native && super::inline_boxes::atomic_element(el) {
        Level::Inline(vec![InlineItem::Atomic(build_block_box(el, ids))])
    } else if native {
        native_inline_flow(el, ids)
    } else {
        flatten_inline(el, ids)
    }
}

fn append_level(out: &mut Vec<FlowLevel>, level: Level) {
    match level {
        Level::Split(levels) => out.extend(levels),
        Level::Block(block) => out.push(FlowLevel::Block(block)),
        Level::Inline(items) => out.push(FlowLevel::Inline(items)),
    }
}

fn native_inline_run(
    items: Vec<InlineItem>,
    node_id: NodeId,
    source: &ComputedStyle,
    first: bool,
    last: bool,
) -> Vec<InlineItem> {
    let style = source.inline_fragment_style(first, last);
    let mut run = Vec::with_capacity(items.len() + 2);
    run.push(InlineItem::Boundary {
        node_id,
        style: style.clone(),
        start: true,
    });
    run.extend(items);
    run.push(InlineItem::Boundary {
        node_id,
        style,
        start: false,
    });
    run
}

fn native_inline_flow(el: &StyledElement, ids: &mut Ids) -> Level {
    let node_id = ids.element(el);
    let mut children = Vec::new();
    super::contents::visit(&el.children, &el.style, &mut |child, style| {
        if let Some(level) = classify(child, style, ids) {
            append_level(&mut children, level);
        }
    });
    let mut out = Vec::new();
    let mut run = Vec::new();
    let mut first = true;
    for child in children {
        match child {
            FlowLevel::Inline(items) => run.extend(items),
            // An absolutely positioned box or a float does not split its inline
            // ancestor: it stays in the run as a placeholder (its static position /
            // the top of its line).
            FlowLevel::Block(block) if block.line_placeholder() => {
                run.push(InlineItem::Atomic(block));
            }
            FlowLevel::Block(block) => {
                out.push(FlowLevel::Inline(native_inline_run(
                    std::mem::take(&mut run),
                    node_id,
                    &el.style,
                    first,
                    false,
                )));
                if let Some(sources) = &mut ids.sources {
                    sources.inline_block_owner(block.node_id, node_id);
                }
                out.push(FlowLevel::Block(block));
                first = false;
            }
        }
    }
    let last = native_inline_run(run, node_id, &el.style, first, true);
    if first {
        Level::Inline(last)
    } else {
        out.push(FlowLevel::Inline(last));
        Level::Split(out)
    }
}

fn classify_svg_viewport(element: &StyledElement, ids: &mut Ids) -> Level {
    match super::svg_viewport::display(element) {
        Display::Inline | Display::InlineBlock => {
            Level::Inline(vec![InlineItem::Atomic(build_block_box(element, ids))])
        }
        _ => Level::Block(build_block_box(element, ids)),
    }
}

fn classify(node: &StyledNode, parent_style: &ComputedStyle, ids: &mut Ids) -> Option<Level> {
    let el = match node {
        StyledNode::Text(t) => {
            return Some(Level::Inline(vec![text_item(node, t, parent_style, ids)]))
        }
        StyledNode::Element(e) => e,
    };
    // Live snapshots exclude every display-none subtree, including markers;
    // the PDF path preserves its existing marker interpretation.
    if ids.sources.is_some() && ids.hidden(&el.style) {
        return None;
    }
    if ids.is_svg_viewport(el) {
        return Some(classify_svg_viewport(el, ids));
    }
    classify_element(el, ids)
}

fn classify_element(el: &StyledElement, ids: &mut Ids) -> Option<Level> {
    match &el.tag {
        Tag::Marker(kind) => Some(marker_level(*kind, el, ids)),
        Tag::Html(_) => classify_html(el, ids),
    }
}

/// Flatten an inline element's content into inline items (its own style applies). A
/// nested in-flow block splits the inline around it (CSS 2.2 §9.2.1.1,
/// miaskiewicz/turbo-dom#101): the element becomes a [`Level::Split`] of its inline runs
/// and the blocks, which its block container wraps into anonymous line boxes and block
/// boxes; an absolutely positioned or floated box stays in its run (a placeholder).
fn flatten_inline(el: &StyledElement, ids: &mut Ids) -> Level {
    let mut levels = Vec::new();
    super::contents::visit(&el.children, &el.style, &mut |child, style| {
        if let Some(level) = classify(child, style, ids) {
            append_level(&mut levels, level);
        }
    });
    // Without a block the split is a single run (the same as `Level::Inline` to every
    // consumer, `append_level`).
    let mut out = Vec::new();
    let mut run = Vec::new();
    for level in levels {
        match level {
            FlowLevel::Block(block) if !block.line_placeholder() => {
                out.push(FlowLevel::Inline(std::mem::take(&mut run)));
                out.push(FlowLevel::Block(block));
            }
            other => run.extend(inline_items_of(vec![other])),
        }
    }
    out.push(FlowLevel::Inline(run));
    Level::Split(out)
}

// --------------------------------------------------------------------------
// block-box construction + anonymous wrapping
// --------------------------------------------------------------------------

fn box_kind_for(display: Display, el: &StyledElement, ids: &mut Ids) -> BoxKind {
    // A button `<input type="submit|button|reset">` carries its label in the `value`
    // ATTRIBUTE, not as text content — render it so the box isn't empty (google's
    // "Google Search" / "I'm Feeling Lucky" buttons were blank grey rectangles).
    if let Some(label) = input_button_text(el) {
        let text = vec![StyledNode::Text(label.into())];
        return build_flow(&text, &el.style, ids);
    }
    match display {
        Display::Flex | Display::InlineFlex => BoxKind::Flex(flex_grid_children(el, ids)),
        Display::Grid | Display::InlineGrid => BoxKind::Grid(flex_grid_children(el, ids)),
        Display::Table | Display::InlineTable => table_kind(el, ids),
        _ => build_flow(&el.children, &el.style, ids),
    }
}

/// A table's children. With a caption (`display: table-caption`) the table's box is the
/// table wrapper box: its children are the captions, in source order, and one anonymous
/// table grid box holding every other child (rows and row groups) that carries the
/// table's frame, background and table properties (CSS Tables 3 §3.1, as Chrome 156 lays
/// it out; miaskiewicz/turbo-dom#51). Without one the children are the rows themselves.
fn table_kind(el: &StyledElement, ids: &mut Ids) -> BoxKind {
    let kids = child_block_boxes(el, ids);
    if !kids.iter().any(|kid| kid.display == Display::TableCaption) {
        return BoxKind::Table(kids);
    }
    let (mut captions, rows): (Vec<LayoutBox>, Vec<LayoutBox>) = kids
        .into_iter()
        .partition(|kid| kid.display == Display::TableCaption);
    let node_id = ids.alloc();
    if let Some(recorded) = &mut ids.recorded {
        recorded.table_grids.insert(node_id);
    }
    captions.push(LayoutBox {
        node_id,
        source_role: SourceRole::TableGrid,
        style: grid_style(&el.style),
        attrs: Vec::new(),
        display: Display::Table,
        kind: BoxKind::Table(rows),
        image: image_of(el),
        svg_viewport: None,
        mask: mask_image(&el.style),
        style_cache: RefCell::new(StyleCache::Unknown),
        html_layout_policy: std::cell::Cell::new((false, 0.0)),
        natural_cache: std::cell::Cell::new(None),
        min_content_cache: std::cell::Cell::new(None),
        intrinsic_w: std::cell::Cell::new(None),
        measure_cache: RefCell::new(None),
        margin_memo: MarginMemoSlot::default(),
    });
    BoxKind::Table(captions)
}

/// The table grid box's style: the table's, less what applies to the table wrapper box
/// (margins, positioning, floating, stacking and transforms).
fn grid_style(table: &ComputedStyle) -> ComputedStyle {
    let mut style = table.clone();
    for property in [
        "margin",
        "margin-top",
        "margin-right",
        "margin-bottom",
        "margin-left",
        "position",
        "inset",
        "top",
        "right",
        "bottom",
        "left",
        "float",
        "clear",
        "z-index",
        "transform",
        "transform-origin",
    ] {
        style.remove(property);
    }
    style.set("display", "table".to_string());
    style
}

/// Whether `kind` is a table wrapper box's (a captioned table; see [`table_kind`]).
fn is_table_wrapper(kind: &BoxKind) -> bool {
    matches!(kind, BoxKind::Table(kids) if kids.iter().any(LayoutBox::table_grid))
}

/// The label of a button-like `<input>` (`type` submit/button/reset) — its `value`
/// attribute, shown as the box's text. `None` for any other element/input type.
fn input_button_text(el: &StyledElement) -> Option<String> {
    if !matches!(&el.tag, Tag::Html(n) if n == "input") {
        return None;
    }
    let ty = attr_value(&el.attrs, "type").unwrap_or("text");
    matches!(ty, "submit" | "button" | "reset")
        .then(|| attr_value(&el.attrs, "value").map(str::to_string))
        .flatten()
}

fn source_role(element: &StyledElement, ids: &Ids) -> SourceRole {
    if native_iframe(element, ids) {
        SourceRole::NativeIframe
    } else if ids
        .document_roots
        .contains(&(std::ptr::from_ref::<StyledElement>(element) as usize))
    {
        SourceRole::DocumentRoot
    } else {
        SourceRole::Ordinary
    }
}

fn native_iframe(element: &StyledElement, ids: &Ids) -> bool {
    ids.sources
        .as_ref()
        .is_some_and(|sources| sources.inline_borders)
        && matches!(&element.tag, Tag::Html(name) if name == "iframe")
}

fn build_block_box(el: &StyledElement, ids: &mut Ids) -> LayoutBox {
    let node_id = ids.element(el);
    if let Tag::Marker(kind) = &el.tag {
        return LayoutBox {
            node_id,
            source_role: SourceRole::Ordinary,
            style: el.style.clone(),
            attrs: el.attrs.clone(),
            display: Display::Block,
            kind: BoxKind::Marker(*kind),
            image: None,
            svg_viewport: None,
            mask: None,
            style_cache: RefCell::new(StyleCache::Unknown),
            html_layout_policy: std::cell::Cell::new((false, 0.0)),
            natural_cache: std::cell::Cell::new(None),
            min_content_cache: std::cell::Cell::new(None),
            intrinsic_w: std::cell::Cell::new(None),
            measure_cache: RefCell::new(None),
            margin_memo: MarginMemoSlot::default(),
        };
    }
    let svg_viewport = ids.is_svg_viewport(el).then(|| {
        Box::new(super::svg_viewport::SvgViewport::from_element(
            el,
            ids.sources.as_ref().unwrap().svg_layout_scale,
        ))
    });
    let (display, kind) = if svg_viewport.is_some() {
        (super::svg_viewport::display(el), BoxKind::Lines(Vec::new()))
    } else {
        let display = display_of(&el.style);
        (display, box_kind_for(display, el, ids))
    };
    // A table wrapper box paints nothing of its own: its grid box has the images.
    let wrapper = is_table_wrapper(&kind);
    ids.top_layer |= el.style.get(TOP_LAYER).is_some();
    LayoutBox {
        node_id,
        source_role: source_role(el, ids),
        style: el.style.clone(),
        attrs: el.attrs.clone(),
        display,
        kind,
        image: image_of(el).filter(|_| !wrapper),
        svg_viewport,
        mask: mask_image(&el.style).filter(|_| !wrapper),
        style_cache: RefCell::new(StyleCache::Unknown),
        html_layout_policy: std::cell::Cell::new((false, 0.0)),
        natural_cache: std::cell::Cell::new(None),
        min_content_cache: std::cell::Cell::new(None),
        intrinsic_w: std::cell::Cell::new(None),
        measure_cache: RefCell::new(None),
        margin_memo: MarginMemoSlot::default(),
    }
}

/// The raster image a styled element references: an `<img src>` is replaced
/// content; a `background-image: url(...)` paints behind the box. The `<img>`
/// source wins when both are present.
fn image_of(el: &StyledElement) -> Option<ImageSource> {
    if let Some(src) = img_src(el) {
        return Some(ImageSource {
            name: src.to_string(),
            replaced: true,
        });
    }
    background_image(&el.style).map(|name| ImageSource {
        name,
        replaced: false,
    })
}

/// The image URL an `<img>` paints, or `None` for any other tag. Prefers a plain
/// `src`, then a lazy-load data attribute, then the first `srcset` candidate.
fn img_src(el: &StyledElement) -> Option<&str> {
    match &el.tag {
        Tag::Html(name) if name == "img" => img_url_attr(&el.attrs),
        _ => None,
    }
}

/// The painted URL among an `<img>`'s attributes: `src` first, else a lazy-load
/// data attribute (`data-src`/`data-landscape-url`/…), else the first `srcset`
/// candidate. Nike's hero `<img>`s ship no `src`/`srcset` — the real URL lives in
/// `data-landscape-url` (client JS copies it to `src` on scroll) — so without this
/// fallback every hero image laid out as an empty box (a blank hero band).
fn img_url_attr(attrs: &[Attr]) -> Option<&str> {
    const KEYS: [&str; 6] = [
        "src",
        "data-src",
        "data-landscape-url",
        "data-portrait-url",
        "data-original",
        "data-image-src",
    ];
    for k in KEYS {
        if let Some(v) = attr_value(attrs, k).filter(|v| !v.trim().is_empty()) {
            return Some(v);
        }
    }
    attr_value(attrs, "srcset").and_then(srcset_first_url)
}

/// The URL of a `srcset`'s first candidate (`url [descriptor]`, comma-separated).
fn srcset_first_url(srcset: &str) -> Option<&str> {
    srcset
        .split(',')
        .next()
        .and_then(|c| c.split_whitespace().next())
}

/// The value of the named attribute in `attrs`, if present.
fn attr_value<'a>(attrs: &'a [Attr], name: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|a| a.name == name)
        .map(|a| a.value.as_str())
}

/// The url of a `background-image: url(...)` longhand, or of a `url(...)` in the
/// `background` shorthand (real stylesheets set images via the shorthand). A
/// `none`/gradient/unparsable value yields `None`.
fn background_image(style: &ComputedStyle) -> Option<String> {
    if let Some(url) = style.get("background-image").and_then(url_token) {
        return Some(url);
    }
    let bg = style.get("background")?;
    super::value::css_value_tokens(bg)
        .into_iter()
        .find_map(url_token)
}

/// The url of a `mask-image`/`-webkit-mask-image: url(...)` (or a `url(...)` in the
/// `mask` shorthand). Icons use this with `background-color` to tint an SVG glyph.
fn mask_image(style: &ComputedStyle) -> Option<String> {
    for prop in ["mask-image", "-webkit-mask-image", "mask", "-webkit-mask"] {
        let Some(v) = style.get(prop).map(str::trim) else {
            continue;
        };
        // A single `url(...)` may be a `data:` URI whose body contains commas (an
        // inline `<svg>` mask) — take it whole rather than splitting on commas.
        if let Some(url) = url_token(v) {
            return Some(url);
        }
        if let Some(url) = super::value::css_value_tokens(v)
            .into_iter()
            .find_map(url_token)
        {
            return Some(url);
        }
    }
    None
}

/// The bare url inside a `url(...)` token (quotes stripped), or `None` for any
/// other token.
fn url_token(token: &str) -> Option<String> {
    let inner = token.trim().strip_prefix("url(")?.strip_suffix(')')?;
    let name = inner.trim().trim_matches(['"', '\'']).trim();
    (!name.is_empty()).then(|| name.to_string())
}

/// Table anonymous-row/cell synthesis remains separate from flex/grid items.
fn child_block_boxes(el: &StyledElement, ids: &mut Ids) -> Vec<LayoutBox> {
    let mut boxes = Vec::new();
    super::contents::visit(&el.children, &el.style, &mut |node, _| {
        boxes.extend(block_child(node, ids));
    });
    boxes
}

fn flex_grid_children(el: &StyledElement, ids: &mut Ids) -> Vec<LayoutBox> {
    let mut boxes = Vec::new();
    let mut text = Vec::new();
    super::contents::visit(&el.children, &el.style, &mut |node, style| match node {
        StyledNode::Text(value) => text.push(text_item(node, value, style, ids)),
        StyledNode::Element(_) => {
            if let Some(child) = block_child(node, ids) {
                flush_flex_text(&mut text, &el.style, ids, &mut boxes);
                boxes.push(child);
            }
        }
    });
    flush_flex_text(&mut text, &el.style, ids, &mut boxes);
    boxes
}

fn flush_flex_text(
    text: &mut Vec<InlineItem>,
    style: &ComputedStyle,
    ids: &mut Ids,
    boxes: &mut Vec<LayoutBox>,
) {
    // A flex container never renders white-space-only text, whatever its
    // `white-space` (CSS Flexbox 1 §4).
    if text.iter().all(is_white_space_text) {
        text.clear();
        return;
    }
    // The item inherits text properties, never the container's width or frame.
    boxes.push(anon_lines_box(std::mem::take(text), style, ids));
}

fn block_child(node: &StyledNode, ids: &mut Ids) -> Option<LayoutBox> {
    let el = node.as_element()?;
    if !is_marker(el) && ids.hidden(&el.style) {
        return None;
    }
    Some(build_block_box(el, ids))
}

/// Whether an inline run between blocks generates no anonymous box: all its text is
/// white space its `white-space` collapses away (CSS 2.1 §9.2.2.1).
fn run_is_blank(run: &[InlineItem]) -> bool {
    run.iter().all(is_blank_text)
}

fn is_white_space_text(item: &InlineItem) -> bool {
    matches!(item, InlineItem::Text { text, .. } if text.chars().all(is_white_space))
}

fn is_white_space(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\n' | '\r' | '\x0c')
}

/// White-space-only text that collapses away: under `pre`/`pre-wrap`/`break-spaces`
/// nothing does, under `pre-line` everything but a newline.
fn is_blank_text(item: &InlineItem) -> bool {
    let InlineItem::Text { text, style, .. } = item else {
        return false;
    };
    match style.get("white-space") {
        Some("pre" | "pre-wrap" | "break-spaces") => text.is_empty(),
        // A pre-line newline is a forced line break (miaskiewicz/turbo-dom#121).
        Some("pre-line") => text.chars().all(|ch| ch != '\n' && is_white_space(ch)),
        _ => text.chars().all(is_white_space),
    }
}

/// The anonymous block box wrapping an inline run next to block-level siblings. Its
/// style is anonymous-box style (CSS 2.1 §9.2.1.1): only what it inherits from its
/// parent, never the parent's frame, size, margins, background or `position` — on
/// every layout path (miaskiewicz/turbo-dom#28 item 3; the PDF path used to copy the
/// parent's whole style, so the run was padded, sized and offset a second time).
fn anon_lines_box(
    items: Vec<InlineItem>,
    parent_style: &ComputedStyle,
    ids: &mut Ids,
) -> LayoutBox {
    LayoutBox {
        node_id: ids.alloc(),
        source_role: SourceRole::AnonymousInlineFlow,
        style: parent_style.inherited_only(),
        attrs: Vec::new(),
        display: Display::Block,
        kind: BoxKind::Lines(items),
        image: None,
        svg_viewport: None,
        mask: None,
        style_cache: RefCell::new(StyleCache::Unknown),
        html_layout_policy: std::cell::Cell::new((false, 0.0)),
        natural_cache: std::cell::Cell::new(None),
        min_content_cache: std::cell::Cell::new(None),
        intrinsic_w: std::cell::Cell::new(None),
        measure_cache: RefCell::new(None),
        margin_memo: MarginMemoSlot::default(),
    }
}

fn flush_run(
    run: &mut Vec<InlineItem>,
    parent_style: &ComputedStyle,
    ids: &mut Ids,
    out: &mut Vec<LayoutBox>,
) {
    if run.is_empty() || run_is_blank(run) {
        run.clear();
        return;
    }
    let items = std::mem::take(run);
    out.push(anon_lines_box(items, parent_style, ids));
}

fn wrap_runs(
    levels: Vec<FlowLevel>,
    parent_style: &ComputedStyle,
    ids: &mut Ids,
) -> Vec<LayoutBox> {
    let mut out = Vec::new();
    let mut run: Vec<InlineItem> = Vec::new();
    for level in levels {
        match level {
            FlowLevel::Inline(items) => run.extend(items),
            // An absolutely positioned box or a float inside inline content stays in
            // that line run (it neither ends nor splits it); one between blocks — or
            // before any inline content — stays a block.
            FlowLevel::Block(b) if b.line_placeholder() && !run_is_blank(&run) => {
                run.push(InlineItem::Atomic(b));
            }
            FlowLevel::Block(b) => {
                flush_run(&mut run, parent_style, ids, &mut out);
                out.push(b);
            }
        }
    }
    flush_run(&mut run, parent_style, ids, &mut out);
    out
}

/// Whether a flow needs a block formatting context: it has an in-flow block-level
/// child, or absolutely positioned boxes with no inline content to sit in.
fn needs_block_context(levels: &[FlowLevel]) -> bool {
    let mut blocks = false;
    let mut content = false;
    for level in levels {
        match level {
            FlowLevel::Block(b) if !b.out_of_flow() => return true,
            FlowLevel::Block(_) => blocks = true,
            FlowLevel::Inline(items) => content |= !run_is_blank(items),
        }
    }
    blocks && !content
}

fn inline_items_of(levels: Vec<FlowLevel>) -> Vec<InlineItem> {
    // Generic inline flattening keeps its legacy atomic treatment of nested blocks.
    let mut out = Vec::new();
    for level in levels {
        match level {
            FlowLevel::Inline(items) => out.extend(items),
            FlowLevel::Block(block) => out.push(InlineItem::Atomic(block)),
        }
    }
    out
}

/// Build the formatting context for a flow of children: a block context (with
/// anonymous-block wrapping) if any child is block-level, else an inline context.
fn build_flow(children: &[StyledNode], parent_style: &ComputedStyle, ids: &mut Ids) -> BoxKind {
    let mut levels = Vec::new();
    super::contents::visit(children, parent_style, &mut |node, style| {
        if let Some(level) = classify(node, style, ids) {
            append_level(&mut levels, level);
        }
    });
    if needs_block_context(&levels) {
        BoxKind::Block(wrap_runs(levels, parent_style, ids))
    } else {
        BoxKind::Lines(inline_items_of(levels))
    }
}

/// Build the box tree for a document flow. The root is an anonymous block box
/// (id 0) whose `kind` is the top-level formatting context.
#[must_use]
pub fn build_box_tree(styled: &[StyledNode]) -> LayoutBox {
    build_box_tree_with(styled, BuildOptions::default()).0
}

/// [`build_box_tree`] with box-generation options; returns the recorded sources
/// (empty unless `record_sources`) and the id count.
pub(crate) fn build_box_tree_with(
    styled: &[StyledNode],
    options: BuildOptions,
) -> (LayoutBox, Recorded, u32) {
    let mut ids = Ids::new(None, options);
    // A styled document (the HTML drive's `Roots::Document`) has its root element at
    // the top: it lays out as the document root (its own block formatting context, so
    // margins never collapse through it), exactly as in the live-DOM layout.
    ids.document_roots = html_document_roots(styled);
    let root = build_root(styled, &mut ids);
    (root, ids.recorded.unwrap_or_default(), ids.next)
}

/// Geometry uses the identical tree once, with source identities out of band.
pub(crate) fn build_sourced_tree(
    styled: &[StyledNode],
    sources: BuildSources,
) -> (
    LayoutBox,
    std::collections::HashMap<NodeId, SourceRecord>,
    u32,
) {
    let mut ids = Ids::new(Some(sources), BuildOptions::default());
    ids.document_roots = html_document_roots(styled);
    let root = build_root(styled, &mut ids);
    (
        root,
        ids.sources.expect("source mode installed").records,
        ids.next,
    )
}

fn html_document_roots(styled: &[StyledNode]) -> std::collections::HashSet<usize> {
    styled.iter().filter_map(|node| match node {
        StyledNode::Element(element) if matches!(&element.tag, Tag::Html(name) if name.eq_ignore_ascii_case("html")) =>
            Some(std::ptr::from_ref::<StyledElement>(element) as usize),
        _ => None,
    }).collect()
}

fn build_root(styled: &[StyledNode], ids: &mut Ids) -> LayoutBox {
    let node_id = ids.alloc();
    let style = ComputedStyle::default();
    let mut kind = build_flow(styled, &style, ids);
    if ids.top_layer {
        hoist_top_layer(&mut kind, ids);
    }
    anonymous_box(node_id, style, kind)
}

/// The internal property the fullscreen UA rule sets on the fullscreen element
/// (`crate::style::fullscreen_user_agent_css`): its box is in the top layer.
const TOP_LAYER: &str = "-internal-top-layer";

/// The `z-index` that keeps the top layer above every other box.
const TOP_LAYER_Z: &str = "2147483647";

/// Move every top-layer box (the fullscreen element, miaskiewicz/turbo-dom#82) out of
/// its place to the end of the initial containing block's content (`root`), each after
/// its `::backdrop` (a black viewport-filling fixed box), above all other boxes: Chrome
/// lays the top layer out as children of the initial containing block, so no ancestor's
/// offset, transform, clip or stacking context reaches it, and its old place collapses.
fn hoist_top_layer(root: &mut BoxKind, ids: &mut Ids) {
    let mut layer = Vec::new();
    take_top_layer(root, &mut layer);
    for mut element in layer {
        element.style.set("z-index", TOP_LAYER_Z.to_string());
        let backdrop = backdrop_box(ids.alloc());
        for b in [backdrop, element] {
            match root {
                BoxKind::Lines(items) => items.push(InlineItem::Atomic(b)),
                BoxKind::Block(kids)
                | BoxKind::Flex(kids)
                | BoxKind::Grid(kids)
                | BoxKind::Table(kids) => kids.push(b),
                BoxKind::Marker(_) => {}
            }
        }
    }
}

/// Remove the top-layer boxes under `kind` into `out`, in document order (a nested one
/// after its top-layer ancestor).
fn take_top_layer(kind: &mut BoxKind, out: &mut Vec<LayoutBox>) {
    let taken = match kind {
        BoxKind::Block(kids) | BoxKind::Flex(kids) | BoxKind::Grid(kids) | BoxKind::Table(kids) => {
            take_top_layer_boxes(kids, out)
        }
        BoxKind::Lines(items) => take_top_layer_items(items, out),
        BoxKind::Marker(_) => Vec::new(),
    };
    for mut b in taken {
        let mut nested = Vec::new();
        take_top_layer(&mut b.kind, &mut nested);
        out.push(b);
        out.append(&mut nested);
    }
}

/// Whether a box is in the top layer.
fn in_top_layer(b: &LayoutBox) -> bool {
    b.style.get(TOP_LAYER).is_some()
}

/// [`take_top_layer`] over block-level children: the top-layer children (returned) leave
/// `kids`; the others' subtrees give theirs to `out`.
fn take_top_layer_boxes(kids: &mut Vec<LayoutBox>, out: &mut Vec<LayoutBox>) -> Vec<LayoutBox> {
    let (taken, kept): (Vec<_>, Vec<_>) = std::mem::take(kids).into_iter().partition(in_top_layer);
    *kids = kept;
    for b in kids.iter_mut() {
        take_top_layer(&mut b.kind, out);
    }
    taken
}

/// [`take_top_layer`] over line content: top-layer atomics (returned) leave `items`.
fn take_top_layer_items(items: &mut Vec<InlineItem>, out: &mut Vec<LayoutBox>) -> Vec<LayoutBox> {
    let mut taken = Vec::new();
    for item in std::mem::take(items) {
        match item {
            InlineItem::Atomic(b) if in_top_layer(&b) => taken.push(b),
            other => items.push(other),
        }
    }
    for item in items.iter_mut() {
        if let InlineItem::Atomic(b) = item {
            take_top_layer(&mut b.kind, out);
        }
    }
    taken
}

/// The fullscreen element's `::backdrop` (Chrome 156: `position: fixed; inset: 0;
/// background: black`), an anonymous box under it in the top layer.
fn backdrop_box(node_id: NodeId) -> LayoutBox {
    let mut style = ComputedStyle::default();
    for (property, value) in [
        ("display", "block"),
        ("position", "fixed"),
        ("top", "0"),
        ("right", "0"),
        ("bottom", "0"),
        ("left", "0"),
        ("background-color", "rgb(0, 0, 0)"),
        ("z-index", TOP_LAYER_Z),
    ] {
        style.set(property, value.to_string());
    }
    anonymous_box(node_id, style, BoxKind::Block(Vec::new()))
}

/// A sourceless block box.
fn anonymous_box(node_id: NodeId, style: ComputedStyle, kind: BoxKind) -> LayoutBox {
    LayoutBox {
        node_id,
        source_role: SourceRole::Ordinary,
        style,
        attrs: Vec::new(),
        display: Display::Block,
        kind,
        image: None,
        svg_viewport: None,
        mask: None,
        style_cache: RefCell::new(StyleCache::Unknown),
        html_layout_policy: std::cell::Cell::new((false, 0.0)),
        natural_cache: std::cell::Cell::new(None),
        min_content_cache: std::cell::Cell::new(None),
        intrinsic_w: std::cell::Cell::new(None),
        measure_cache: RefCell::new(None),
        margin_memo: MarginMemoSlot::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `div` with `style` (property, value) pairs and `children`.
    fn styled_div(pairs: &[(&str, &str)], children: Vec<StyledNode>) -> StyledNode {
        let mut style = ComputedStyle::default();
        for (property, value) in pairs {
            style.set(property, (*value).to_string());
        }
        StyledNode::Element(StyledElement {
            tag: Tag::Html("div".into()),
            attrs: vec![],
            style,
            children,
            source: None,
        })
    }

    const FULLSCREEN: &[(&str, &str)] = &[("position", "fixed"), (TOP_LAYER, "fullscreen")];

    fn kids(b: &LayoutBox) -> &[LayoutBox] {
        match &b.kind {
            BoxKind::Block(k) => k,
            other => panic!("not a block: {other:?}"),
        }
    }

    /// miaskiewicz/turbo-dom#82: top-layer boxes move, in document order (a nested one
    /// after its top-layer ancestor), to the end of the root, each after a black fixed
    /// backdrop; both get the top `z-index`; boxes without the marker stay put.
    #[test]
    fn top_layer_boxes_move_to_the_root_after_their_backdrops() {
        let styled = vec![
            styled_div(
                &[],
                vec![styled_div(FULLSCREEN, vec![styled_div(FULLSCREEN, vec![])])],
            ),
            styled_div(&[("height", "5px")], vec![]),
        ];
        let root = build_box_tree(&styled);
        let top = kids(&root);
        assert_eq!(top.len(), 6);
        // The two ordinary boxes keep their ids (0 is the root); the first lost its child.
        assert_eq!((top[0].node_id, top[1].node_id), (NodeId(1), NodeId(4)));
        assert!(kids(&top[0]).is_empty());
        let (outer, inner) = (&top[3], &top[5]);
        assert_eq!((outer.node_id, inner.node_id), (NodeId(2), NodeId(3)));
        assert!(
            kids(outer).is_empty(),
            "the nested top-layer box left it too"
        );
        for (backdrop, id) in [(&top[2], 5), (&top[4], 6)] {
            assert_eq!(backdrop.node_id, NodeId(id));
            assert!(backdrop.attrs.is_empty() && kids(backdrop).is_empty());
            assert_eq!(backdrop.style.get("background-color"), Some("rgb(0, 0, 0)"));
            for side in ["position", "top", "right", "bottom", "left"] {
                assert!(backdrop.style.get(side).is_some(), "{side}");
            }
            assert_eq!(backdrop.style.get("position"), Some("fixed"));
            assert_eq!(backdrop.style.get("top"), Some("0"));
            assert_eq!(backdrop.style.get("right"), Some("0"));
            assert_eq!(backdrop.style.get("bottom"), Some("0"));
            assert_eq!(backdrop.style.get("left"), Some("0"));
            assert_eq!(backdrop.style.get("display"), Some("block"));
        }
        for b in [&top[2], outer, &top[4], inner] {
            assert_eq!(b.style.get("z-index"), Some(TOP_LAYER_Z));
        }
        assert_eq!(top[1].style.get("z-index"), None);
    }

    /// A top-layer box in line content (an inline run at the root) moves to the end of
    /// that run, as atomics after its backdrop.
    #[test]
    fn top_layer_boxes_in_line_content_move_to_the_end_of_the_run() {
        let text = StyledNode::Text(crate::style::StyledText {
            text: "x".into(),
            source: None,
        });
        let mut inline = ComputedStyle::default();
        inline.set("display", "inline-block".into());
        let mut fullscreen_inline = inline.clone();
        fullscreen_inline.set("position", "fixed".into());
        fullscreen_inline.set(TOP_LAYER, "fullscreen".into());
        let atom = |style: ComputedStyle| {
            StyledNode::Element(StyledElement {
                tag: Tag::Html("span".into()),
                attrs: vec![],
                style,
                children: vec![],
                source: None,
            })
        };
        let root = build_box_tree(&[atom(fullscreen_inline), text, atom(inline)]);
        let BoxKind::Lines(items) = &root.kind else {
            panic!("an inline root: {:?}", root.kind);
        };
        let atoms: Vec<(u32, bool)> = items
            .iter()
            .filter_map(|item| match item {
                InlineItem::Atomic(b) => Some((b.node_id.0, b.style.get(TOP_LAYER).is_some())),
                _ => None,
            })
            .collect();
        assert_eq!(atoms, vec![(3, false), (4, false), (1, true)]);
    }

    fn html_el(tag: &str, children: Vec<StyledNode>) -> StyledNode {
        StyledNode::Element(StyledElement {
            tag: Tag::Html(tag.to_string()),
            attrs: vec![],
            style: ComputedStyle::default(),
            children,
            source: None,
        })
    }

    fn bare(tag: &str) -> StyledElement {
        StyledElement {
            tag: Tag::Html(tag.to_string()),
            attrs: vec![],
            style: ComputedStyle::default(),
            children: vec![],
            source: None,
        }
    }

    fn reuse_policy_tree(text: &str) -> LayoutBox {
        use crate::test_support::literal::{style_nodes, Element, Node};
        let nodes = vec![Node::Element(Element {
            tag: Tag::Html("div".into()),
            attrs: vec![Attr { name: "style".into(), value: "display:flex;width:41.145px;font-family:ParityGo;font-size:13px;line-height:10px;padding-left:3.145px".into() }],
            children: vec![Node::Element(Element { tag: Tag::Html("div".into()), attrs: vec![Attr { name: "style".into(), value: "display:flex;padding-left:2.61px".into() }], children: vec![Node::Text(text.into())] })],
        })];
        build_box_tree(&style_nodes(
            &nodes,
            &crate::style::build_cascade("", "", crate::style::TokenSet::default()),
        ))
    }

    fn reuse_policy_layout(
        root: &LayoutBox,
        fonts: &crate::FontRegistry,
        policy: (bool, Option<f32>),
    ) -> super::super::fragment::Fragment {
        match policy {
            (true, scale) => {
                super::super::block::layout_tree_live(
                    root,
                    (400.0, 300.0),
                    fonts,
                    &crate::ImageCtx::none(),
                    &mut crate::Diagnostics::default(),
                    super::super::block::TextLayoutPolicy {
                        start: Some((100, true, scale)),
                        original_units: None,
                    },
                )
                .0
            }
            (false, _) => super::super::block::layout_tree(
                root,
                400.0,
                fonts,
                &mut crate::Diagnostics::default(),
            ),
        }
    }

    fn cache_children(root: &LayoutBox) -> Vec<&LayoutBox> {
        match &root.kind {
            BoxKind::Block(children)
            | BoxKind::Flex(children)
            | BoxKind::Grid(children)
            | BoxKind::Table(children) => children.iter().collect(),
            BoxKind::Lines(items) => items
                .iter()
                .filter_map(|item| match item {
                    InlineItem::Atomic(child) => Some(child),
                    _ => None,
                })
                .collect(),
            BoxKind::Marker(_) => Vec::new(),
        }
    }
    fn cache_signature(root: &LayoutBox) -> Vec<(Option<f32>, Option<f32>, Option<String>)> {
        let mut output = vec![(
            root.natural_cache.get(),
            root.min_content_cache.get(),
            root.measure_cache
                .borrow()
                .as_ref()
                .map(|value| format!("{:?}", value.values)),
        )];
        for child in cache_children(root) {
            output.extend(cache_signature(child));
        }
        output
    }

    #[test]
    fn reusable_box_tree_resets_intrinsic_measurement_and_placement_policy() {
        let mut fonts = crate::FontRegistry::new();
        fonts.add(
            crate::FontFace::from_bytes(
                include_bytes!("../../assets/fonts/Go-Regular.ttf").to_vec(),
                "ParityGo",
                400,
                false,
            )
            .unwrap(),
        );
        let reused = reuse_policy_tree("ab ab");
        for policy in [
            (true, Some(2.0)),
            (false, None),
            (true, None),
            (true, Some(1.0)),
            (true, Some(2.0)),
        ] {
            let fresh = reuse_policy_tree("ab ab");
            let actual = reuse_policy_layout(&reused, &fonts, policy);
            let expected = reuse_policy_layout(&fresh, &fonts, policy);
            assert_eq!(
                format!("{actual:?}"),
                format!("{expected:?}"),
                "policy {policy:?}"
            );
            assert_eq!(
                super::super::flex::natural_width(&reused, &fonts),
                super::super::flex::natural_width(&fresh, &fonts)
            );
            assert_eq!(
                super::super::flex::min_content_width(&reused, &fonts),
                super::super::flex::min_content_width(&fresh, &fonts)
            );
            let actual_cache = cache_signature(&reused);
            let expected_cache = cache_signature(&fresh);
            assert!(actual_cache.iter().any(|entry| entry.2.is_some()));
            assert_eq!(actual_cache, expected_cache);
        }
    }

    fn source_units_for(
        root: &LayoutBox,
        units: &[u16],
    ) -> crate::layout_snapshot::text_units::NativeUnitMap {
        let mut map = std::collections::HashMap::new();
        if let BoxKind::Lines(items) = &root.kind {
            for item in items {
                if let InlineItem::Text { node_id, .. } = item {
                    map.insert(*node_id, std::sync::Arc::from(units));
                }
            }
        }
        for child in cache_children(root) {
            map.extend(source_units_for(child, units));
        }
        map
    }
    #[test]
    fn original_unit_policy_reuse_releases_stale_intrinsic_and_measure_caches() {
        let mut fonts = crate::FontRegistry::new();
        fonts.add(
            crate::FontFace::from_bytes(
                include_bytes!("../../assets/fonts/Go-Regular.ttf").to_vec(),
                "ParityGo",
                400,
                false,
            )
            .unwrap(),
        );
        let reused = reuse_policy_tree("\u{fffd}");
        for original in [Some(0xd800), Some(0xfffd), None, Some(0xdcff)] {
            let fresh = reuse_policy_tree("\u{fffd}");
            let reused_map =
                original.map(|unit| std::sync::Arc::new(source_units_for(&reused, &[unit])));
            let fresh_map =
                original.map(|unit| std::sync::Arc::new(source_units_for(&fresh, &[unit])));
            reused.set_original_units(reused_map.as_ref());
            fresh.set_original_units(fresh_map.as_ref());
            reused.set_html_layout_policy((true, Some(2.0)));
            fresh.set_html_layout_policy((true, Some(2.0)));
            assert_eq!(
                super::super::flex::natural_width(&reused, &fonts),
                super::super::flex::natural_width(&fresh, &fonts)
            );
            assert_eq!(
                super::super::flex::min_content_width(&reused, &fonts),
                super::super::flex::min_content_width(&fresh, &fonts)
            );
            let layout = |tree: &LayoutBox, map| {
                super::super::block::layout_tree_live(
                    tree,
                    (400.0, 300.0),
                    &fonts,
                    &crate::ImageCtx::none(),
                    &mut crate::Diagnostics::default(),
                    super::super::block::TextLayoutPolicy {
                        start: Some((100, true, Some(2.0))),
                        original_units: map,
                    },
                )
                .0
            };
            assert_eq!(
                format!("{:?}", layout(&reused, reused_map.clone())),
                format!("{:?}", layout(&fresh, fresh_map))
            );
            assert_eq!(cache_signature(&reused), cache_signature(&fresh));
            assert!(same_original_inputs(
                reused.original_units().as_ref(),
                reused_map.as_ref()
            ));
        }
    }

    #[test]
    fn is_br_matches_only_the_br_tag() {
        assert!(is_br(&bare("br")));
        assert!(!is_br(&bare("span")));
    }

    #[test]
    fn br_becomes_an_inline_line_break() {
        // `<div>a<br>b</div>`: the <br> is an inline forced break within the div's
        // inline formatting context (a `LineBreak` item), not a block box.
        let tree = build_box_tree(&[html_el(
            "div",
            vec![
                StyledNode::Text("a".into()),
                html_el("br", vec![]),
                StyledNode::Text("b".into()),
            ],
        )]);
        let BoxKind::Block(kids) = &tree.kind else {
            panic!("root is a block");
        };
        let BoxKind::Lines(items) = &kids[0].kind else {
            panic!("div establishes an inline (lines) context");
        };
        assert!(
            items.iter().any(|i| matches!(i, InlineItem::LineBreak)),
            "the <br> produced a LineBreak item"
        );
    }
}
