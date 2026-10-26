//! Scrollable overflow from real native border fragments and resolved styles.
//! Clipping and scroll state remain separate from immutable layout provenance.
use crate::live_layout::LiveTreeInput;
use std::collections::BTreeMap;
use turbo_dom_layout::layout_snapshot::{GeometryKind, LayoutSnapshot, SourceId};
use turbo_dom_layout::{ComputedStyle, StyledNode};

pub(crate) struct SourceStyle {
    pub(crate) style: ComputedStyle,
    pub(crate) parent: Option<SourceId>,
    pub(crate) tag: String,
    /// Effective CSS zoom: the product of computed factors along the flat tree.
    pub(crate) zoom: f32,
}
pub(crate) type SourceStyles = BTreeMap<SourceId, SourceStyle>;

pub(crate) fn collect_styles(input: &LiveTreeInput, styled: &[StyledNode]) -> SourceStyles {
    let identities: BTreeMap<_, _> = input
        .identities
        .iter()
        .map(|identity| (identity.path.as_slice(), identity.id))
        .collect();
    fn walk(
        nodes: &[StyledNode],
        path: &mut Vec<usize>,
        parent: (Option<SourceId>, f32),
        ids: &BTreeMap<&[usize], SourceId>,
        out: &mut SourceStyles,
    ) {
        for (i, node) in nodes.iter().enumerate() {
            path.push(i);
            let id = ids.get(path.as_slice()).copied();
            if let StyledNode::Element(element) = node {
                let zoom = parent.1 * crate::live_layout_zoom::factor(&element.style);
                if let Some(id) = id {
                    out.insert(
                        id,
                        SourceStyle {
                            style: element.style.clone(),
                            parent: parent.0,
                            zoom,
                            tag: match &element.tag {
                                turbo_dom_layout::Tag::Html(tag) => tag.clone(),
                                _ => String::new(),
                            },
                        },
                    );
                }
                walk(&element.children, path, (id.or(parent.0), zoom), ids, out);
            }
            path.pop();
        }
    }
    let mut out = BTreeMap::new();
    walk(styled, &mut Vec::new(), (None, 1.0), &identities, &mut out);
    out
}
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub(crate) struct ScrollMetrics {
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) scroll_x: bool,
    pub(crate) scroll_y: bool,
    pub(crate) max_x: f64,
    pub(crate) max_y: f64,
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
pub(crate) fn overflow(style: &ComputedStyle, axis: &str) -> &'static str {
    let value = raw_overflow(style, axis);
    let other = raw_overflow(
        style,
        if axis == "overflow-x" {
            "overflow-y"
        } else {
            "overflow-x"
        },
    );
    if !matches!(other, "visible" | "clip") {
        match value {
            "visible" => "auto",
            "clip" => "hidden",
            _ => value,
        }
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
pub(crate) fn scroll_metrics(
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
    let projected = |fragment: &turbo_dom_layout::layout_snapshot::SnapshotFragment| {
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
            if metrics.position != turbo_dom_layout::layout::value::Position::Absolute
                && metrics.position != turbo_dom_layout::layout::value::Position::Fixed
            {
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
    fragment: &turbo_dom_layout::layout_snapshot::SnapshotFragment,
    id: SourceId,
) -> bool {
    let Some(start) = fragment
        .source_ancestors
        .iter()
        .position(|ancestor| *ancestor == id)
    else {
        return true;
    };
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

pub(crate) fn scroll_translation(
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
        .map(|used| used.position)
        .unwrap_or(turbo_dom_layout::layout::value::Position::Static);
    use turbo_dom_layout::layout::value::Position;
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
pub(crate) const VIEWPORT: SourceId = SourceId(u64::MAX);

/// The root element has no flat-tree parent; its scrolling box is the viewport.
pub(crate) fn is_root(styles: &SourceStyles, id: SourceId) -> bool {
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
pub(crate) fn propagated_body(styles: &SourceStyles, id: SourceId) -> bool {
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
pub(crate) fn element_scroller(styles: &SourceStyles, id: SourceId) -> bool {
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
pub(crate) fn viewport_metrics(
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
        use turbo_dom_layout::layout::value::Position;
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

/// Chromium ScrollAlignment behaviors for one axis: fully visible, hidden and
/// partially visible exposure.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Align {
    None,
    Start,
    Center,
    End,
    Closest,
}

pub(crate) fn alignment(block: u32, mode: u32) -> Result<[Align; 3], String> {
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
pub(crate) fn expose(
    start: f64,
    length: f64,
    current: f64,
    visible: f64,
    behaviors: [Align; 3],
) -> f64 {
    let (start, length, current, visible) = (
        layout_unit(start),
        layout_unit(length),
        layout_unit(current),
        layout_unit(visible),
    );
    let end = start + length;
    let intersection = (end.min(current + visible) - start.max(current)).max(0.0);
    let mut behavior = if intersection == length {
        behaviors[0]
    } else if intersection == visible {
        match behaviors[0] {
            Align::Center => Align::None,
            other => other,
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::live_layout::{capture_tree, layout_rules_resolved, source_id};
    use turbo_dom_layout::layout_snapshot::SnapshotInputs;
    use turbo_dom_layout::{Diagnostics, FontRegistry, ImageCtx, NoImages};
    use turbo_dom_parser::rtdom::Tree;
    #[test]
    fn scroll_extent_uses_actual_overflow_padding_margins_transforms_and_nested_clips() {
        let oracle: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../harness/detection/fixtures/scroll-layout-chrome155.json"
        ))
        .unwrap();
        let cases = [
            (
                "padded",
                "padding:5px;border:2px solid;overflow:auto",
                "width:180px;height:140px",
            ),
            (
                "visible",
                "padding:5px;border:2px solid;overflow:visible",
                "width:180px;height:140px",
            ),
            (
                "hidden",
                "padding:5px;border:2px solid;overflow:hidden",
                "width:180px;height:140px",
            ),
            (
                "negative",
                "padding:5px;border:2px solid;position:relative;overflow:auto",
                "position:absolute;left:-50px;top:-40px;width:20px;height:20px",
            ),
            (
                "absolute",
                "padding:5px;border:2px solid;position:relative;overflow:auto",
                "position:absolute;left:120px;top:90px;width:30px;height:20px",
            ),
            (
                "transform",
                "padding:5px;border:2px solid;overflow:auto",
                "width:50px;height:30px;transform:translate(120px,90px)",
            ),
            (
                "margin",
                "padding:5px;border:2px solid;overflow:auto",
                "width:180px;height:140px;margin:10px",
            ),
            ("scaledContainer","padding:5px;border:2px solid;overflow:auto;transform:scale(2);transform-origin:0 0","width:180px;height:140px"),
            ("transformedNegative","padding:5px;border:2px solid;overflow:auto","width:180px;height:140px;transform:translate(-100px,-80px)"),
            (
                "nestedclip",
                "padding:5px;border:2px solid;overflow:auto",
                "width:50px;height:30px;overflow:hidden",
            ),
        ];
        let mut mismatches = Vec::new();
        for (index, (name, outer, inner)) in cases.into_iter().enumerate() {
            let nested = if name == "nestedclip" {
                "<div style='width:250px;height:220px'></div>"
            } else {
                ""
            };
            let tree=Tree::parse(&format!("<html><body style='margin:0'><div id=box style='width:100px;height:80px;{outer}'><div style='{inner}'>{nested}</div></div></body></html>"));
            let input = capture_tree(&tree, 1).unwrap();
            let fonts = FontRegistry::new();
            let images = ImageCtx {
                resolver: &NoImages,
                body_height: None,
            };
            let (snapshot, styles) = layout_rules_resolved(
                &input,
                &[],
                2.0,
                &SnapshotInputs {
                    width: 800.0,
                    height: 600.0,
                    fonts: &fonts,
                    images: &images,
                },
                &mut Diagnostics::default(),
            )
            .unwrap();
            let actual = scroll_metrics(
                &snapshot,
                &styles,
                source_id(1, tree.query_selector("#box").unwrap()),
            )
            .unwrap();
            let expected = &oracle["result"][index]["before"];
            let expected = (
                expected["width"].as_i64().unwrap() as i32,
                expected["height"].as_i64().unwrap() as i32,
            );
            if (actual.width, actual.height) != expected {
                mismatches.push(format!(
                    "{name}: native={:?} Chrome={expected:?}",
                    (actual.width, actual.height)
                ));
            }
        }
        assert!(mismatches.is_empty(), "{}", mismatches.join("; "));
    }
}
