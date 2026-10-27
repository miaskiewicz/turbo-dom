//! Scrollable overflow from real native border fragments and resolved styles.
//! Clipping and scroll state remain separate from immutable layout provenance.
use crate::layout::value::Position;
use crate::layout_snapshot::{GeometryKind, LayoutSnapshot, SourceId};
use crate::ComputedStyle;
use std::collections::BTreeMap;

/// One element's used style as the embedder collected it after the cascade: its
/// style, nearest source ancestor in the flat tree, local name and effective zoom.
pub struct SourceStyle {
    pub style: ComputedStyle,
    pub parent: Option<SourceId>,
    pub tag: String,
    /// Effective CSS zoom: the product of computed factors along the flat tree.
    pub zoom: f32,
}
/// Every element's [`SourceStyle`] by source.
pub type SourceStyles = BTreeMap<SourceId, SourceStyle>;

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ScrollMetrics {
    pub width: i32,
    pub height: i32,
    pub scroll_x: bool,
    pub scroll_y: bool,
    pub max_x: f64,
    pub max_y: f64,
}
fn overflow_value(value: &str) -> Option<&'static str> {
    ["visible", "clip", "hidden", "scroll", "auto"]
        .into_iter()
        .find(|keyword| value.eq_ignore_ascii_case(keyword))
}
fn raw_overflow(style: &ComputedStyle, axis: &str) -> &'static str {
    if let Some(value) = style.get(axis).and_then(overflow_value) {
        return value;
    }
    let mut shorthand = style
        .get("overflow")
        .unwrap_or("visible")
        .split_ascii_whitespace();
    let first = shorthand
        .next()
        .and_then(overflow_value)
        .unwrap_or("visible");
    if axis == "overflow-y" {
        shorthand.next().and_then(overflow_value).unwrap_or(first)
    } else {
        first
    }
}
#[must_use]
pub fn overflow(style: &ComputedStyle, axis: &str) -> &'static str {
    let value = raw_overflow(style, axis);
    let other = raw_overflow(
        style,
        if axis == "overflow-x" {
            "overflow-y"
        } else {
            "overflow-x"
        },
    );
    // Chrome 156 pairs `visible` with a scrollable axis as `auto`, but keeps `clip`
    // (that axis clips without scrolling; miaskiewicz/turbo-dom#124).
    if value == "visible" && !matches!(other, "visible" | "clip") {
        "auto"
    } else {
        value
    }
}
fn permits_scroll(value: &str) -> bool {
    matches!(value, "auto" | "scroll" | "hidden")
}
fn clips(value: &str) -> bool {
    value != "visible"
}
fn rounded(value: f32) -> Result<i32, String> {
    if !value.is_finite() || value < 0.0 || f64::from(value) > f64::from(i32::MAX) {
        return Err("native scroll extent exceeds integer range".into());
    }
    Ok(value.round() as i32)
}
pub fn scroll_metrics(
    snapshot: &LayoutSnapshot,
    styles: &SourceStyles,
    id: SourceId,
) -> Result<ScrollMetrics, String> {
    let source = snapshot.source(id).ok_or("native scroll source missing")?;
    if source.excluded_from_layout
        || styles
            .get(&id)
            .is_some_and(|entry| entry.style.get("display") == Some("contents"))
    {
        return Ok(ScrollMetrics {
            width: 0,
            height: 0,
            scroll_x: false,
            scroll_y: false,
            max_x: 0.0,
            max_y: 0.0,
        });
    }
    if snapshot.unsupported_svg_source_geometry(id) {
        return Err("native SVG descendant scroll geometry is unsupported".into());
    }
    let first = source
        .fragment_indices
        .iter()
        .map(|&i| &snapshot.fragments()[i])
        .find(|fragment| fragment.kind == GeometryKind::BorderBox)
        .ok_or("native scroll requires inline border provenance")?;
    let used = first.used_box.ok_or("native scroll used box missing")?;
    let padding = used.padding_rect(first.rect);
    let own = styles
        .get(&id)
        .ok_or("native scroll computed style missing")?;
    let x = overflow(&own.style, "overflow-x");
    let y = overflow(&own.style, "overflow-y");
    if own.style.get("direction") == Some("rtl")
        || own
            .style
            .get("writing-mode")
            .is_some_and(|v| v != "horizontal-tb")
    {
        return Err("native scroll RTL/vertical writing provenance pending".into());
    }
    let matrix = snapshot
        .native_world_transform(first.index)
        .ok_or("native scroll transform unavailable")?;
    let determinant = matrix[0] * matrix[3] - matrix[1] * matrix[2];
    if determinant == 0.0 || !determinant.is_finite() {
        return Err(
            "native scroll degenerate-container relative transform provenance pending".into(),
        );
    }
    let projected = |fragment: &crate::layout_snapshot::SnapshotFragment| {
        let points = fragment.quad.map(|point| {
            let x = f64::from(point.x) - matrix[4];
            let y = f64::from(point.y) - matrix[5];
            [
                ((matrix[3] * x - matrix[2] * y) / determinant) as f32,
                ((-matrix[1] * x + matrix[0] * y) / determinant) as f32,
            ]
        });
        [
            points
                .iter()
                .map(|point| point[0])
                .fold(f32::NEG_INFINITY, f32::max),
            points
                .iter()
                .map(|point| point[1])
                .fold(f32::NEG_INFINITY, f32::max),
        ]
    };
    let mut right = padding.x + padding.width;
    let mut bottom = padding.y + padding.height;
    for &index in &source.descendant_fragment_indices {
        let fragment = &snapshot.fragments()[index];
        if !matches!(
            fragment.kind,
            GeometryKind::BorderBox | GeometryKind::TextLineContainer
        ) {
            continue;
        }
        if !contained(styles, fragment, id) {
            continue;
        }
        // A nested clipping ancestor owns its descendants' overflow. Its own
        // border still contributes to the outer scroll area's extent.
        let mut blocked_x = false;
        let mut blocked_y = false;
        for ancestor in &fragment.source_ancestors {
            if *ancestor == id {
                blocked_x = false;
                blocked_y = false;
                continue;
            }
            // A propagated overflow belongs to the viewport and clips nothing here.
            if *ancestor == fragment.source_id.unwrap_or(id)
                || is_root(styles, *ancestor)
                || propagated_body(styles, *ancestor)
            {
                continue;
            }
            if let Some(ancestor) = styles.get(ancestor) {
                if ancestor.style.get("display") == Some("contents") {
                    continue;
                }
                blocked_x |= clips(overflow(&ancestor.style, "overflow-x"));
                blocked_y |= clips(overflow(&ancestor.style, "overflow-y"));
            }
        }
        let transformed = projected(fragment);
        let rect = fragment.rect;
        if !blocked_x {
            right = right.max(transformed[0]).max(rect.x + rect.width);
        }
        if !blocked_y {
            bottom = bottom.max(transformed[1]).max(rect.y + rect.height);
        }
        // Flow content's ending padding/margin contributes to scroll containers,
        // while out-of-flow boxes and transforms contribute only their edges.
        if let Some(metrics) = fragment.used_box {
            if metrics.position != Position::Absolute && metrics.position != Position::Fixed {
                if !blocked_x && permits_scroll(x) {
                    right =
                        right.max(rect.x + rect.width + metrics.margin.right + used.padding.right);
                }
                if !blocked_y && permits_scroll(y) {
                    bottom = bottom
                        .max(rect.y + rect.height + metrics.margin.bottom + used.padding.bottom);
                }
            }
        }
    }
    Ok(ScrollMetrics {
        width: rounded(right - padding.x)?,
        height: rounded(bottom - padding.y)?,
        scroll_x: permits_scroll(x),
        scroll_y: permits_scroll(y),
        max_x: f64::from((right - padding.x - padding.width).max(0.0)),
        max_y: f64::from((bottom - padding.y - padding.height).max(0.0)),
    })
}

/// Whether a fragment belongs to the element's scrollable overflow: every
/// absolute/fixed box between them (including the fragment's own box) must find
/// its containing block at or inside the element, never an outer block or the
/// viewport.
fn contained(
    styles: &SourceStyles,
    fragment: &crate::layout_snapshot::SnapshotFragment,
    id: SourceId,
) -> bool {
    // Only called on the element's `descendant_fragment_indices`, which
    // `layout_snapshot::sources::index_fragments` fills with exactly the fragments
    // listing the element among their `source_ancestors`.
    let start = fragment
        .source_ancestors
        .iter()
        .position(|ancestor| *ancestor == id)
        .expect("a descendant fragment lists its scroller among its source ancestors");
    let own = fragment
        .source_id
        .filter(|own| fragment.source_ancestors.last() != Some(own));
    let chain = fragment.source_ancestors[start..].iter().chain(own.iter());
    let (mut absolute, mut fixed) = (false, false);
    for (index, source) in chain.enumerate() {
        let Some(entry) = styles
            .get(source)
            .filter(|entry| !has_value(&entry.style, "display", "contents"))
        else {
            continue;
        };
        if index > 0
            && (has_value(&entry.style, "position", "absolute") && !absolute
                || has_value(&entry.style, "position", "fixed") && !fixed)
        {
            return false;
        }
        let transformed = entry
            .style
            .get("transform")
            .is_some_and(|value| !value.trim().eq_ignore_ascii_case("none"));
        absolute |= transformed
            || !has_value(&entry.style, "position", "static")
                && entry.style.get("position").is_some();
        fixed |= transformed;
    }
    true
}

pub fn scroll_translation(
    snapshot: &LayoutSnapshot,
    styles: &SourceStyles,
    id: SourceId,
    offsets: &BTreeMap<SourceId, [f64; 2]>,
) -> Result<[f32; 2], String> {
    let source = snapshot.source(id).ok_or("native scroll source missing")?;
    if source.excluded_from_layout {
        return Ok([0.0; 2]);
    }
    let fragment = source
        .fragment_indices
        .iter()
        .map(|&i| &snapshot.fragments()[i])
        .find(|fragment| fragment.kind == GeometryKind::BorderBox)
        .or_else(|| {
            snapshot
                .text_runs()
                .iter()
                .find(|run| run.source_id == Some(id) || run.source_ancestors.contains(&id))
                .map(|run| &snapshot.fragments()[run.fragment_index])
        })
        .ok_or("native scroll requires placed source provenance")?;
    let position = fragment
        .used_box
        .map_or(Position::Static, |used| used.position);
    if position == Position::Fixed {
        return Ok([0.0; 2]);
    }
    let viewport = if fixed_ancestor(styles, &fragment.source_ancestors, id) {
        [0.0; 2]
    } else {
        offsets.get(&VIEWPORT).copied().unwrap_or([0.0; 2])
    };
    let containing = if position == Position::Absolute {
        fragment.source_ancestors.iter().rposition(|ancestor| {
            styles.get(ancestor).is_some_and(|style| {
                style.style.get("display") != Some("contents")
                    && style
                        .style
                        .get("position")
                        .is_some_and(|value| value != "static")
            })
        })
    } else {
        Some(fragment.source_ancestors.len())
    };
    let mut translation = [0.0; 2];
    for (i, ancestor) in fragment.source_ancestors.iter().enumerate() {
        if containing.is_none_or(|index| i > index) {
            continue;
        }
        let Some(offset) = offsets.get(ancestor) else {
            continue;
        };
        if offset == &[0.0; 2] {
            continue;
        }
        let ancestor = snapshot
            .source(*ancestor)
            .ok_or("native scroll ancestor source missing")?;
        let box_ = ancestor
            .fragment_indices
            .iter()
            .map(|&index| &snapshot.fragments()[index])
            .find(|fragment| fragment.kind == GeometryKind::BorderBox)
            .ok_or("native scroll ancestor box missing")?;
        let matrix = box_.world_transform;
        translation[0] += matrix[0] * offset[0] as f32 + matrix[2] * offset[1] as f32;
        translation[1] += matrix[1] * offset[0] as f32 + matrix[3] * offset[1] as f32;
    }
    // The initial containing block scrolls with the identity viewport matrix.
    translation[0] += viewport[0] as f32;
    translation[1] += viewport[1] as f32;
    Ok(translation)
}

/// Scroll state key for the document viewport; real sources use document-qualified
/// handles and can never reach the all-ones identity.
pub const VIEWPORT: SourceId = SourceId(u64::MAX);

/// The root element has no flat-tree parent; its scrolling box is the viewport.
#[must_use]
pub fn is_root(styles: &SourceStyles, id: SourceId) -> bool {
    styles.get(&id).is_some_and(|entry| entry.parent.is_none())
}

fn has_value(style: &ComputedStyle, property: &str, value: &str) -> bool {
    style
        .get(property)
        .is_some_and(|actual| actual.trim().eq_ignore_ascii_case(value))
}

// A fixed box (and everything it contains) stays attached to the viewport.
fn fixed_ancestor(styles: &SourceStyles, ancestors: &[SourceId], own: SourceId) -> bool {
    ancestors.iter().any(|ancestor| {
        *ancestor != own
            && styles.get(ancestor).is_some_and(|entry| {
                !has_value(&entry.style, "display", "contents")
                    && has_value(&entry.style, "position", "fixed")
            })
    })
}

/// A body whose overflow propagated to the viewport is not its own scroll container.
#[must_use]
pub fn propagated_body(styles: &SourceStyles, id: SourceId) -> bool {
    let Some(entry) = styles.get(&id) else {
        return false;
    };
    let Some(parent) = entry.parent.and_then(|parent| styles.get(&parent)) else {
        return false;
    };
    entry.tag.eq_ignore_ascii_case("body")
        && parent.parent.is_none()
        && overflow(&parent.style, "overflow-x") == "visible"
        && overflow(&parent.style, "overflow-y") == "visible"
        && (overflow(&entry.style, "overflow-x") != "visible"
            || overflow(&entry.style, "overflow-y") != "visible")
}

/// Whether an element establishes an element scrolling box (not the viewport).
#[must_use]
pub fn element_scroller(styles: &SourceStyles, id: SourceId) -> bool {
    let Some(entry) = styles.get(&id) else {
        return false;
    };
    !is_root(styles, id)
        && !propagated_body(styles, id)
        && !has_value(&entry.style, "display", "contents")
        && (permits_scroll(overflow(&entry.style, "overflow-x"))
            || permits_scroll(overflow(&entry.style, "overflow-y")))
}

/// Viewport scrollable overflow: the initial containing block plus every
/// in-flow or absolute descendant edge not owned by a nested clipping box.
pub fn viewport_metrics(
    snapshot: &LayoutSnapshot,
    styles: &SourceStyles,
    root: SourceId,
    size: [f32; 2],
) -> Result<ScrollMetrics, String> {
    let source = snapshot
        .source(root)
        .ok_or("native scroll source missing")?;
    let mut right = size[0];
    let mut bottom = size[1];
    for &index in source
        .fragment_indices
        .iter()
        .chain(&source.descendant_fragment_indices)
    {
        let fragment = &snapshot.fragments()[index];
        if !matches!(
            fragment.kind,
            GeometryKind::BorderBox | GeometryKind::TextLineContainer
        ) {
            continue;
        }
        let own = fragment.source_id.unwrap_or(root);
        let position = fragment.used_box.map(|used| used.position);
        if position == Some(Position::Fixed)
            || fixed_ancestor(styles, &fragment.source_ancestors, own)
        {
            continue;
        }
        let mut blocked = [false; 2];
        for ancestor in &fragment.source_ancestors {
            if *ancestor == own || is_root(styles, *ancestor) || propagated_body(styles, *ancestor)
            {
                continue;
            }
            if let Some(entry) = styles.get(ancestor) {
                if has_value(&entry.style, "display", "contents") {
                    continue;
                }
                blocked[0] |= clips(overflow(&entry.style, "overflow-x"));
                blocked[1] |= clips(overflow(&entry.style, "overflow-y"));
            }
        }
        let rect = fragment.rect;
        let quad = |axis: usize| {
            fragment
                .quad
                .iter()
                .map(|point| if axis == 0 { point.x } else { point.y })
                .fold(f32::NEG_INFINITY, f32::max)
        };
        let flow = fragment
            .used_box
            .filter(|_| position != Some(Position::Absolute))
            .map_or([0.0; 2], |used| [used.margin.right, used.margin.bottom]);
        if !blocked[0] {
            right = right.max(quad(0)).max(rect.x + rect.width + flow[0]);
        }
        if !blocked[1] {
            bottom = bottom.max(quad(1)).max(rect.y + rect.height + flow[1]);
        }
    }
    Ok(ScrollMetrics {
        width: rounded(right)?,
        height: rounded(bottom)?,
        scroll_x: true,
        scroll_y: true,
        max_x: f64::from((right - size[0]).max(0.0)),
        max_y: f64::from((bottom - size[1]).max(0.0)),
    })
}

/// Chromium `ScrollAlignment` behaviors for one axis: fully visible, hidden and
/// partially visible exposure.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Align {
    None,
    Start,
    Center,
    End,
    Closest,
}

pub fn alignment(block: u32, mode: u32) -> Result<[Align; 3], String> {
    Ok(match (mode, block) {
        (0, 0) => [Align::Start; 3],
        (0, 1) => [Align::Center; 3],
        (0, 2) => [Align::End; 3],
        (0, 3) | (2, _) => [Align::None, Align::Closest, Align::Closest],
        (1, _) => [Align::None, Align::Center, Align::Closest],
        _ => return Err("native scroll alignment is invalid".into()),
    })
}

// LayoutUnit precision (1/64 px) decides visibility exactly like layout does.
fn layout_unit(value: f64) -> f64 {
    (value * 64.0).round() / 64.0
}

/// Scroll position exposing `[start, start + length)` in one scroller axis whose
/// visible content range is `[current, current + visible)`.
#[must_use]
pub fn expose(start: f64, length: f64, current: f64, visible: f64, behaviors: [Align; 3]) -> f64 {
    let (start, length, current, visible) = (
        layout_unit(start),
        layout_unit(length),
        layout_unit(current),
        layout_unit(visible),
    );
    let end = start + length;
    let intersection = (end.min(current + visible) - start.max(current)).max(0.0);
    // A target covering the whole visible range counts as fully visible: Chrome 156
    // still centers it for `center` (miaskiewicz/turbo-dom#125).
    let mut behavior = if intersection == length || intersection == visible {
        behaviors[0]
    } else if intersection > 0.0 {
        behaviors[2]
    } else {
        behaviors[1]
    };
    if behavior == Align::Closest {
        let visible_end = current + visible;
        behavior =
            if (end > visible_end && length < visible) || (end < visible_end && length > visible) {
                Align::End
            } else {
                Align::Start
            };
    }
    match behavior {
        Align::None | Align::Closest => current,
        Align::Start => start,
        Align::Center => (start + end - visible) / 2.0,
        Align::End => end - visible,
    }
}

/// The CSSOM-View Chrome 156 control (`tests/fixtures/cssom_view_chrome156.json`, generated
/// by turbo-dom's `scripts/cssom-view-chrome-control.mjs`): each case laid out through the
/// live HTML snapshot as turbo-surf takes it (`testing::layout`), shared by the scroll,
/// visibility, text and range tests.
#[cfg(test)]
pub(crate) mod chrome {
    use super::{SourceStyles, VIEWPORT};
    use crate::layout_snapshot::{LayoutSnapshot, Rect, SourceId};
    use crate::live::source_id;
    use crate::live::testing::{self, DOCUMENT};
    use serde_json::Value;
    use std::collections::BTreeMap;
    use turbo_dom::Tree;

    const FIXTURE: &str = include_str!("../../tests/fixtures/cssom_view_chrome156.json");

    pub(crate) struct Case {
        pub json: Value,
        pub tree: Tree,
        pub snapshot: LayoutSnapshot,
        pub styles: SourceStyles,
    }

    /// The control case `id`, laid out at Chrome's 1280x800 viewport.
    pub(crate) fn case(id: &str) -> Case {
        let fixture: Value = serde_json::from_str(FIXTURE).unwrap();
        assert!(fixture["chrome"].as_str().unwrap().starts_with("156."));
        let json = fixture["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["id"] == id)
            .unwrap_or_else(|| panic!("no control case {id}"))
            .clone();
        let html = json["html"].as_str().unwrap();
        let css = html
            .split_once("<style>")
            .and_then(|(_, rest)| rest.split_once("</style>"))
            .map_or("", |(css, _)| css);
        let (tree, snapshot, styles) = if html.contains("<img") {
            with_images(html, css)
        } else {
            testing::layout(html, css, (1280.0, 800.0), Some(1.0))
        };
        Case {
            json,
            tree,
            snapshot,
            styles,
        }
    }

    /// Every image of a control document is its 150x80 PNG `data:` URI.
    struct DataImages;
    impl crate::ImageResolver for DataImages {
        fn resolve(&self, name: &str) -> Option<&[u8]> {
            use std::sync::OnceLock;
            static PNG: OnceLock<Vec<u8>> = OnceLock::new();
            let encoded = name.strip_prefix("data:image/png;base64,")?;
            Some(PNG.get_or_init(|| base64_decode(encoded)))
        }
    }

    fn base64_decode(text: &str) -> Vec<u8> {
        let value = |c: u8| match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            _ => 63,
        };
        let digits: Vec<u8> = text.bytes().filter(|&c| c != b'=').map(value).collect();
        digits
            .chunks(4)
            .flat_map(|chunk| {
                let bits = chunk
                    .iter()
                    .enumerate()
                    .fold(0u32, |acc, (i, &d)| acc | u32::from(d) << (18 - 6 * i));
                let bytes = bits.to_be_bytes();
                bytes[1..chunk.len()].to_vec()
            })
            .collect()
    }

    /// [`testing::layout`] with the control's `data:` images resolved.
    fn with_images(html: &str, css: &str) -> (Tree, LayoutSnapshot, SourceStyles) {
        use crate::live::{snapshot, LiveOptions, LiveStyle, LiveTree, OriginRules};
        let tree = Tree::parse(html);
        let children = testing::light_children(&tree);
        let captured = testing::capture_light(&tree, &children);
        let origins = testing::prematch(&tree, css, 1280.0);
        let origins: Vec<_> = origins
            .iter()
            .map(|(origin, rules)| OriginRules {
                origin: *origin,
                rules: rules.iter().map(Some).collect(),
            })
            .collect();
        let provider = |h| children.get(&h).map_or(&[][..], Vec::as_slice);
        let style = LiveStyle {
            tree: LiveTree {
                tree: &tree,
                document: DOCUMENT,
                children: &provider,
            },
            origins: &origins,
            scope_depth: &|_| Ok(0),
            inline_override: &|_| None,
            inline_declarations: &|text| Ok(crate::style::parse_declarations(text)),
        };
        let fonts = crate::FontRegistry::new();
        let images = crate::ImageCtx {
            resolver: &DataImages,
            body_height: None,
        };
        let inputs = crate::layout_snapshot::SnapshotInputs {
            width: 1280.0,
            height: 800.0,
            fonts: &fonts,
            images: &images,
        };
        let (snapshot, styles) = snapshot(
            &style,
            &captured,
            &LiveOptions { layout_scale: 1.0 },
            |styled| Ok(testing::collect(styled)),
            &inputs,
            &mut crate::Diagnostics::default(),
        )
        .unwrap();
        (tree, snapshot, styles)
    }

    impl Case {
        /// The source of the `[data-cid]` element (`"viewport"`: the scroll key).
        pub(crate) fn id(&self, cid: &str) -> SourceId {
            if cid == "viewport" {
                return VIEWPORT;
            }
            let handle = self
                .tree
                .query_selector(&format!("[data-cid=\"{cid}\"]"))
                .unwrap_or_else(|| panic!("no element {cid}"));
            source_id(DOCUMENT, handle)
        }
        pub(crate) fn root(&self) -> SourceId {
            source_id(DOCUMENT, self.tree.query_selector("html").unwrap())
        }
        /// The case's scroll offsets, keyed like turbo-surf's scroll state.
        pub(crate) fn offsets(&self) -> BTreeMap<SourceId, [f64; 2]> {
            self.json["scroll"]
                .as_object()
                .unwrap()
                .iter()
                .map(|(cid, xy)| {
                    (
                        self.id(cid),
                        [xy[0].as_f64().unwrap(), xy[1].as_f64().unwrap()],
                    )
                })
                .collect()
        }
        /// Chrome's `getBoundingClientRect()` of `cid` (after the case's scrolls).
        pub(crate) fn chrome_box(&self, cid: &str) -> Rect {
            let want = self.json["boxes"]
                .as_array()
                .unwrap()
                .iter()
                .find(|b| b["cid"] == cid)
                .unwrap();
            let f = |key: &str| want[key].as_f64().unwrap() as f32;
            Rect {
                x: f("x"),
                y: f("y"),
                width: f("width"),
                height: f("height"),
            }
        }
        /// The layout border-box bounds of `cid` (unscrolled).
        pub(crate) fn bounds(&self, cid: &str) -> Rect {
            super::super::metrics::element_geometry(&self.snapshot, self.id(cid))
                .unwrap()
                .bounds
        }
        /// The readout list `key` (`metrics`, `visibility`, ...).
        pub(crate) fn reads(&self, key: &str) -> &[Value] {
            self.json[key].as_array().unwrap()
        }
    }

    /// `[x, y, width, height]` of a Chrome rect readout.
    pub(crate) fn rect(value: &Value) -> [f32; 4] {
        std::array::from_fn(|i| value[i].as_f64().unwrap() as f32)
    }

    pub(crate) fn close(a: f32, b: f32) -> bool {
        (a - b).abs() <= 0.01
    }
}

#[cfg(test)]
mod tests {
    use super::chrome::{case, close};
    use super::*;

    /// turbo-surf's device-pixel snap of a scroll offset (device scale 1).
    fn snap(value: f64) -> f64 {
        (value + 0.5).floor()
    }

    fn assert_extents(id: &str) {
        let case = case(id);
        for want in case.reads("metrics") {
            let cid = want["cid"].as_str().unwrap();
            let source = case.id(cid);
            let style = &case.styles[&source].style;
            assert_eq!(
                [overflow(style, "overflow-x"), overflow(style, "overflow-y")],
                [
                    want["overflowX"].as_str().unwrap(),
                    want["overflowY"].as_str().unwrap()
                ],
                "{id}: {cid} computed overflow"
            );
            let got = scroll_metrics(&case.snapshot, &case.styles, source).unwrap();
            // A non-scrollable axis keeps offset 0 (turbo-surf's clamp).
            let max = |max: f64, scrolls: bool| if scrolls { snap(max) as i64 } else { 0 };
            assert_eq!(
                [
                    i64::from(got.width),
                    i64::from(got.height),
                    max(got.max_x, got.scroll_x),
                    max(got.max_y, got.scroll_y)
                ],
                ["scrollWidth", "scrollHeight", "maxX", "maxY"].map(|k| want[k].as_i64().unwrap()),
                "{id}: {cid} scroll extents"
            );
            // An element without a box scrolls in neither axis.
            let boxed = want["scrollWidth"].as_i64() != Some(0);
            assert_eq!(
                [got.scroll_x, got.scroll_y],
                [
                    boxed && permits_scroll(want["overflowX"].as_str().unwrap()),
                    boxed && permits_scroll(want["overflowY"].as_str().unwrap())
                ],
                "{id}: {cid} scrollable axes"
            );
        }
    }

    #[test]
    fn computed_overflow_pairs_match_chrome_156() {
        assert_extents("overflow-computed-pairs");
    }

    #[test]
    fn scroll_extents_add_flow_margins_and_end_padding_like_chrome_156() {
        assert_extents("scroll-extent-flow-and-padding");
        assert_extents("scroll-extent-hidden-and-visible");
    }

    #[test]
    fn positioned_descendants_reach_the_scroller_through_their_containing_block_like_chrome_156() {
        assert_extents("scroll-extent-positioned-children");
    }

    #[test]
    fn nested_clips_own_their_overflow_and_transforms_project_like_chrome_156() {
        assert_extents("scroll-extent-nested-clip-and-transform");
    }

    #[test]
    fn image_boxes_reach_the_scroll_extents_like_chrome_156() {
        assert_extents("scroll-extent-image");
        assert_viewport("scroll-extent-image");
    }

    #[test]
    fn boxless_elements_have_zero_extents_like_chrome_156() {
        assert_extents("scroll-extent-no-box");
    }

    fn assert_viewport(id: &str) {
        let case = case(id);
        let got =
            viewport_metrics(&case.snapshot, &case.styles, case.root(), [1280.0, 800.0]).unwrap();
        let want = &case.json["viewport"];
        assert_eq!(
            [
                i64::from(got.width),
                i64::from(got.height),
                snap(got.max_x) as i64,
                snap(got.max_y) as i64
            ],
            ["scrollWidth", "scrollHeight", "maxX", "maxY"].map(|k| want[k].as_i64().unwrap()),
            "{id}"
        );
        assert!(got.scroll_x && got.scroll_y);
    }

    /// The body's overflow scrolls the viewport unless the root's own overflow is not
    /// visible; the root and display:contents boxes are never element scrollers.
    #[test]
    fn body_overflow_propagation_matches_chrome_156() {
        for (id, propagates) in [
            ("body-overflow-propagates", true),
            ("body-overflow-own-scroller", false),
            ("body-visible-not-a-scroller", false),
        ] {
            let case = case(id);
            for want in case.reads("metrics") {
                let cid = want["cid"].as_str().unwrap();
                let source = if cid == "body" || cid == "html" {
                    crate::live::source_id(
                        crate::live::testing::DOCUMENT,
                        case.tree.query_selector(cid).unwrap(),
                    )
                } else {
                    case.id(cid)
                };
                let scrolls =
                    want["maxX"].as_f64() != Some(0.0) || want["maxY"].as_f64() != Some(0.0);
                assert_eq!(
                    element_scroller(&case.styles, source),
                    scrolls && cid != "html",
                    "{id}: {cid}"
                );
                assert_eq!(is_root(&case.styles, source), cid == "html", "{id}: {cid}");
                assert_eq!(
                    propagated_body(&case.styles, source),
                    propagates && cid == "body",
                    "{id}: {cid}"
                );
            }
        }
        assert_viewport("body-overflow-propagates");
        let case = case("body-overflow-propagates");
        assert!(!element_scroller(&case.styles, SourceId(3)));
        assert!(!propagated_body(&case.styles, SourceId(3)));
        assert!(!is_root(&case.styles, SourceId(3)));
    }

    #[test]
    fn viewport_extents_match_chrome_156() {
        assert_viewport("viewport-extent");
        assert_viewport("viewport-extent-fits");
        assert_viewport("viewport-extent-body-overflow-propagates");
    }

    fn assert_translated(id: &str) {
        let case = case(id);
        let offsets = case.offsets();
        for want in case.json["boxes"].as_array().unwrap() {
            let cid = want["cid"].as_str().unwrap();
            let [dx, dy] =
                scroll_translation(&case.snapshot, &case.styles, case.id(cid), &offsets).unwrap();
            let bounds = case.bounds(cid);
            let chrome = case.chrome_box(cid);
            assert!(
                close(bounds.x - dx, chrome.x) && close(bounds.y - dy, chrome.y),
                "{id}: {cid} at ({}, {}), Chrome ({}, {})",
                bounds.x - dx,
                bounds.y - dy,
                chrome.x,
                chrome.y
            );
        }
    }

    #[test]
    fn scrolled_boxes_move_with_their_scrolling_ancestors_like_chrome_156() {
        assert_translated("translation-nested-scrollers");
        assert_translated("translation-absolute-skips-static-scroller");
        assert_translated("translation-transformed-scroller");
    }

    #[test]
    fn scroll_into_view_offsets_match_chrome_156() {
        let case = case("into-view");
        let scroller = case.id("s");
        let metrics = scroll_metrics(&case.snapshot, &case.styles, scroller).unwrap();
        let port = case.bounds("s");
        for want in case.reads("intoView") {
            let target = want["target"].as_str().unwrap();
            let bounds = case.bounds(target);
            let from = [
                want["from"][0].as_f64().unwrap(),
                want["from"][1].as_f64().unwrap(),
            ];
            // Chrome clamps the starting offsets first, as the embedder does.
            let from = [
                from[0].clamp(0.0, snap(metrics.max_x)),
                from[1].clamp(0.0, snap(metrics.max_y)),
            ];
            let mode = want["mode"].as_u64().unwrap() as u32;
            let behaviors = [
                alignment(want["inline"].as_u64().unwrap() as u32, mode).unwrap(),
                alignment(want["block"].as_u64().unwrap() as u32, mode).unwrap(),
            ];
            let max = [snap(metrics.max_x), snap(metrics.max_y)];
            let start = [bounds.x - port.x, bounds.y - port.y];
            let length = [bounds.width, bounds.height];
            let visible = [port.width, port.height];
            let got: [f64; 2] = std::array::from_fn(|axis| {
                snap(
                    expose(
                        f64::from(start[axis]),
                        f64::from(length[axis]),
                        from[axis],
                        f64::from(visible[axis]),
                        behaviors[axis],
                    )
                    .clamp(0.0, max[axis]),
                )
            });
            let chrome = [
                want["to"][0].as_f64().unwrap(),
                want["to"][1].as_f64().unwrap(),
            ];
            assert_eq!(got, chrome, "{want}");
        }
    }

    #[test]
    fn alignment_maps_options_to_chromium_behaviors() {
        use Align::{Center, Closest, End, None, Start};
        assert_eq!(alignment(0, 0).unwrap(), [Start; 3]);
        assert_eq!(alignment(1, 0).unwrap(), [Center; 3]);
        assert_eq!(alignment(2, 0).unwrap(), [End; 3]);
        assert_eq!(alignment(3, 0).unwrap(), [None, Closest, Closest]);
        assert_eq!(alignment(7, 2).unwrap(), [None, Closest, Closest]);
        assert_eq!(alignment(7, 1).unwrap(), [None, Center, Closest]);
        assert_eq!(
            alignment(4, 0).unwrap_err(),
            "native scroll alignment is invalid"
        );
        assert_eq!(
            alignment(0, 3).unwrap_err(),
            "native scroll alignment is invalid"
        );
    }

    #[test]
    fn rounded_rejects_extents_outside_the_integer_range() {
        assert_eq!(rounded(2.5).unwrap(), 3);
        assert_eq!(rounded(0.0).unwrap(), 0);
        for bad in [-0.5, f32::NAN, f32::INFINITY, 3.0e9] {
            assert_eq!(
                rounded(bad).unwrap_err(),
                "native scroll extent exceeds integer range"
            );
        }
    }
    fn laid_out(html: &str) -> (turbo_dom::Tree, LayoutSnapshot, SourceStyles) {
        crate::live::testing::layout(html, "", (800.0, 600.0), Some(1.0))
    }

    #[test]
    fn unsupported_scroll_geometry_is_an_error_not_a_guess() {
        let (tree, snapshot, styles) = laid_out(
            "<body><div id=r style='direction:rtl;overflow:auto'></div><div id=v style='writing-mode:vertical-rl;overflow:auto'></div><div id=z style='transform:scale(0);overflow:auto'></div><svg><rect id=s width=5 height=5></rect></svg><span id=i>x</span></body>",
        );
        let id = |selector| crate::live::testing::id(&tree, selector);
        let error = |selector| scroll_metrics(&snapshot, &styles, id(selector)).unwrap_err();
        assert_eq!(
            error("#r"),
            "native scroll RTL/vertical writing provenance pending"
        );
        assert_eq!(
            error("#v"),
            "native scroll RTL/vertical writing provenance pending"
        );
        assert_eq!(
            error("#z"),
            "native scroll degenerate-container relative transform provenance pending"
        );
        assert_eq!(
            error("#s"),
            "native SVG descendant scroll geometry is unsupported"
        );
        assert_eq!(
            scroll_metrics(&snapshot, &styles, SourceId(3)).unwrap_err(),
            "native scroll source missing"
        );
        assert_eq!(
            scroll_translation(&snapshot, &styles, SourceId(3), &BTreeMap::new()).unwrap_err(),
            "native scroll source missing"
        );
        assert_eq!(
            viewport_metrics(&snapshot, &styles, SourceId(3), [1.0, 1.0]).unwrap_err(),
            "native scroll source missing"
        );
    }
}
