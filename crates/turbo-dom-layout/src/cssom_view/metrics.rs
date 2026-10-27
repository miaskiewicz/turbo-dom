//! CSSOM measurements obtained from actual native source fragments. Missing
//! inline/caret provenance is an explicit error, never a synthetic zero box.
use crate::layout_snapshot::{GeometryKind, LayoutSnapshot, Rect, SourceId, SourceKind};

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ClientMetrics {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
}
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct OffsetSize {
    pub width: i32,
    pub height: i32,
}
#[derive(Debug, PartialEq)]
pub struct ElementGeometry {
    pub bounds: Rect,
    pub rects: Vec<Rect>,
    pub client: ClientMetrics,
    pub offset: OffsetSize,
}

pub fn element_geometry(
    snapshot: &LayoutSnapshot,
    id: SourceId,
) -> Result<ElementGeometry, String> {
    let source = snapshot
        .source(id)
        .ok_or("native layout source is missing")?;
    if source.kind != SourceKind::Element {
        return Err("native element geometry requires an Element receiver".into());
    }
    if source.excluded_from_layout {
        return Ok(empty_geometry());
    }
    supported_source(snapshot, id)?;

    let boxes: Vec<_> = source
        .fragment_indices
        .iter()
        .map(|&index| &snapshot.fragments()[index])
        .filter(|fragment| fragment.kind == GeometryKind::BorderBox)
        .collect();
    let first = boxes
        .first()
        .ok_or("native live layout requires inline border/cluster provenance for this element")?;
    let used = first
        .used_box
        .ok_or("native live layout used box metrics are missing")?;
    if used.inline_fragment {
        let rects = boxes
            .iter()
            .map(|fragment| fragment.transformed_bounds)
            .collect::<Vec<_>>();
        let local = all_fragment_bounds(
            &boxes
                .iter()
                .map(|fragment| fragment.rect)
                .collect::<Vec<_>>(),
        );
        return Ok(ElementGeometry {
            bounds: bounding_box(&rects),
            rects,
            client: ClientMetrics {
                left: 0,
                top: 0,
                width: 0,
                height: 0,
            },
            offset: OffsetSize {
                width: integer(local.width)?,
                height: integer(local.height)?,
            },
        });
    }
    let padding = used.padding_rect(first.rect);
    let rects: Vec<_> = boxes
        .iter()
        .map(|fragment| fragment.transformed_bounds)
        .collect();
    let bounds = bounding_box(&rects);
    Ok(ElementGeometry {
        bounds,
        rects,
        client: ClientMetrics {
            left: integer(used.border.left)?,
            top: integer(used.border.top)?,
            width: integer(padding.width)?,
            height: integer(padding.height)?,
        },
        offset: OffsetSize {
            width: integer(first.rect.width)?,
            height: integer(first.rect.height)?,
        },
    })
}

/// A source whose geometry this snapshot cannot supply fails its reads: an SVG
/// descendant, or a source inside an unsupported native inline construct (per element,
/// miaskiewicz/turbo-surf#31).
fn supported_source(snapshot: &LayoutSnapshot, id: SourceId) -> Result<(), String> {
    if snapshot.unsupported_svg_source_geometry(id) {
        return Err("native SVG descendant geometry is unsupported".into());
    }
    snapshot
        .unsupported_inline_geometry(id)
        .map_or(Ok(()), |error| Err(error.to_string()))
}

fn all_fragment_bounds(rects: &[Rect]) -> Rect {
    // Split inlines retain zero-area continuation points in getClientRects, but
    // those points do not enlarge an offset box that has actual fragments.
    // A zero-width line with height still contributes to the offset extent.
    let mut fragments = rects
        .iter()
        .filter(|rect| rect.width != 0.0 || rect.height != 0.0);
    let Some(first) = fragments.next() else {
        return rects
            .first()
            .copied()
            .unwrap_or_else(|| empty_geometry().bounds);
    };
    let mut left = first.x;
    let mut top = first.y;
    let mut right = first.x + first.width;
    let mut bottom = first.y + first.height;
    for rect in fragments {
        left = left.min(rect.x);
        top = top.min(rect.y);
        right = right.max(rect.x + rect.width);
        bottom = bottom.max(rect.y + rect.height);
    }
    Rect {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    }
}

fn integer(value: f32) -> Result<i32, String> {
    if !value.is_finite() || value < 0.0 || f64::from(value) > f64::from(i32::MAX) {
        return Err("native CSSOM metric is outside its integer range".into());
    }
    Ok(value.round() as i32)
}
#[must_use]
pub fn empty_geometry() -> ElementGeometry {
    ElementGeometry {
        bounds: Rect {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        },
        rects: Vec::new(),
        client: ClientMetrics {
            left: 0,
            top: 0,
            width: 0,
            height: 0,
        },
        offset: OffsetSize {
            width: 0,
            height: 0,
        },
    }
}
#[must_use]
pub fn bounding_box(rects: &[Rect]) -> Rect {
    let mut visible = rects
        .iter()
        .filter(|rect| rect.width != 0.0 && rect.height != 0.0);
    let Some(first) = visible.next() else {
        return rects
            .first()
            .copied()
            .unwrap_or_else(|| empty_geometry().bounds);
    };
    let mut left = first.x;
    let mut top = first.y;
    let mut right = left + first.width;
    let mut bottom = top + first.height;
    for rect in visible {
        left = left.min(rect.x);
        top = top.min(rect.y);
        right = right.max(rect.x + rect.width);
        bottom = bottom.max(rect.y + rect.height);
    }
    Rect {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OffsetPosition {
    pub left: i32,
    pub top: i32,
    pub parent: Option<SourceId>,
}
fn offset_integer(value: f32) -> Result<i32, String> {
    if !value.is_finite()
        || f64::from(value) < f64::from(i32::MIN)
        || f64::from(value) > f64::from(i32::MAX)
    {
        return Err("native offset position exceeds integer range".into());
    }
    Ok((f64::from(value) + 0.5).floor() as i32)
}
fn is_inline_fragment(snapshot: &LayoutSnapshot, id: SourceId) -> bool {
    snapshot.source(id).is_some_and(|source| {
        source.fragment_indices.iter().any(|index| {
            snapshot.fragments()[*index]
                .used_box
                .is_some_and(|used| used.inline_fragment)
        })
    })
}
pub fn offset_position(
    snapshot: &LayoutSnapshot,
    styles: &super::scroll::SourceStyles,
    id: SourceId,
) -> Result<OffsetPosition, String> {
    let source = snapshot.source(id).ok_or("native offset source missing")?;
    let style = styles
        .get(&id)
        .ok_or("native offset computed style missing")?;
    let zero = OffsetPosition {
        left: 0,
        top: 0,
        parent: None,
    };
    if source.excluded_from_layout
        || style.style.get("display") == Some("contents")
        || style.tag == "body"
    {
        return Ok(zero);
    }
    let border = source
        .fragment_indices
        .iter()
        .map(|&index| &snapshot.fragments()[index])
        .find(|fragment| fragment.kind == GeometryKind::BorderBox)
        .ok_or("native offset requires inline border provenance")?;
    let used = border.used_box.ok_or("native offset used box missing")?;
    if used.position == crate::layout::value::Position::Fixed {
        return Ok(OffsetPosition {
            left: offset_integer(border.rect.x)?,
            top: offset_integer(border.rect.y)?,
            parent: None,
        });
    }
    let mut next = style.parent;
    while let Some(parent) = next {
        let parent_style = styles
            .get(&parent)
            .ok_or("native offset parent computed style missing")?;
        if parent_style.style.get("display") == Some("contents") {
            next = parent_style.parent;
            continue;
        }
        // The statically positioned BODY fallback uses document coordinates,
        // while actual positioned/transform/table parents use their padding edge.
        let positioned = parent_style
            .style
            .get("position")
            .is_some_and(|value| value != "static");
        let transformed = parent_style
            .style
            .get("transform")
            .is_some_and(|value| value != "none")
            && !is_inline_fragment(snapshot, parent);
        let table = used.position == crate::layout::value::Position::Static
            && matches!(parent_style.tag.as_str(), "table" | "td" | "th");
        // Blink's WebKit extension: offsetParent also stops at a zoom change.
        let zoomed = parent_style.zoom != style.zoom;
        if positioned || transformed || table || zoomed || parent_style.tag == "body" {
            let (x, y) = if parent_style.tag == "body" && !positioned && !transformed {
                (0.0, 0.0)
            } else {
                let source = snapshot
                    .source(parent)
                    .ok_or("native offset parent source missing")?;
                let box_ = source
                    .fragment_indices
                    .iter()
                    .map(|&index| &snapshot.fragments()[index])
                    .find(|fragment| fragment.kind == GeometryKind::BorderBox)
                    .ok_or("native offset parent border missing")?;
                let metrics = box_
                    .used_box
                    .ok_or("native offset parent used box missing")?;
                (
                    box_.rect.x
                        + if metrics.inline_fragment {
                            0.0
                        } else {
                            metrics.border.left
                        },
                    box_.rect.y
                        + if metrics.inline_fragment {
                            0.0
                        } else {
                            metrics.border.top
                        },
                )
            };
            return Ok(OffsetPosition {
                left: offset_integer(border.rect.x - x)?,
                top: offset_integer(border.rect.y - y)?,
                parent: Some(parent),
            });
        }
        next = parent_style.parent;
    }
    Ok(OffsetPosition {
        left: offset_integer(border.rect.x)?,
        top: offset_integer(border.rect.y)?,
        parent: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::live::testing;
    use turbo_dom::rtdom::Tree;
    // Adapted@S4: the live tree is styled and laid out by the live-DOM input
    // (rules pre-matched with turbo-dom's engine) instead of turbo-surf's copier.
    fn capture(html: &str) -> (Tree, LayoutSnapshot) {
        let (tree, snapshot, _) = testing::layout(html, "", (200.0, 200.0), None);
        (tree, snapshot)
    }
    fn id(tree: &Tree, selector: &str) -> SourceId {
        testing::id(tree, selector)
    }
    #[test]
    fn transformed_border_rect_and_untransformed_client_offset_metrics_are_distinct() {
        let (tree,snapshot)=capture("<html><body style='margin:0'><div id=x style='width:40px;height:20px;padding:5px;border:2px solid;transform:scale(2);transform-origin:0 0'></div></body></html>");
        let value = element_geometry(&snapshot, id(&tree, "#x")).unwrap();
        assert_eq!(
            value.bounds,
            Rect {
                x: 0.0,
                y: 0.0,
                width: 108.0,
                height: 68.0
            }
        );
        assert_eq!(value.rects, [value.bounds]);
        assert_eq!(
            value.client,
            ClientMetrics {
                left: 2,
                top: 2,
                width: 50,
                height: 30
            }
        );
        assert_eq!(
            value.offset,
            OffsetSize {
                width: 54,
                height: 34
            }
        );
    }
    #[test]
    fn actual_exclusion_returns_no_rects_but_visible_empty_inline_is_explicitly_unsupported() {
        let (tree,snapshot)=capture("<html><body><div style='display:none'><span id=hidden></span></div><span id=empty></span></body></html>");
        assert_eq!(
            element_geometry(&snapshot, id(&tree, "#hidden")).unwrap(),
            empty_geometry()
        );
        assert!(element_geometry(&snapshot, id(&tree, "#empty"))
            .unwrap_err()
            .contains("inline border/cluster provenance"));
        assert!(element_geometry(&snapshot, SourceId(u64::MAX))
            .unwrap_err()
            .contains("missing"));
    }
    #[test]
    fn offset_extents_ignore_empty_continuation_points_but_keep_zero_width_lines() {
        let before = Rect {
            x: 8.0,
            y: 18.71875,
            width: 0.0,
            height: 0.0,
        };
        let block = Rect {
            x: 8.0,
            y: 18.71875,
            width: 1264.0,
            height: 21.5,
        };
        let after = Rect {
            x: 8.0,
            y: 58.9375,
            width: 0.0,
            height: 0.0,
        };
        assert_eq!(all_fragment_bounds(&[before, block, after]), block);
        let line = Rect {
            x: 8.0,
            y: 50.0,
            width: 0.0,
            height: 12.0,
        };
        assert_eq!(
            all_fragment_bounds(&[before, block, line, after]),
            Rect {
                x: 8.0,
                y: 18.71875,
                width: 1264.0,
                height: 43.28125,
            }
        );
        assert_eq!(all_fragment_bounds(&[before, after]), before);
        assert_eq!(all_fragment_bounds(&[line]), line);
        assert_eq!(all_fragment_bounds(&[]), empty_geometry().bounds);
    }

    #[test]
    fn cssom_bounding_union_ignores_empty_fragments_unless_all_are_empty() {
        let empty = Rect {
            x: 50.0,
            y: 50.0,
            width: 0.0,
            height: 10.0,
        };
        let first = Rect {
            x: 2.0,
            y: 3.0,
            width: 10.0,
            height: 20.0,
        };
        let second = Rect {
            x: 4.0,
            y: 2.0,
            width: 12.0,
            height: 30.0,
        };
        assert_eq!(
            bounding_box(&[empty, first, second]),
            Rect {
                x: 2.0,
                y: 2.0,
                width: 14.0,
                height: 30.0
            }
        );
        assert_eq!(bounding_box(&[empty]), empty);
        assert!(integer(f32::NAN).is_err());
        assert!(integer(-1.0).is_err());
        assert!(integer(f32::MAX).is_err());
    }
}

#[cfg(test)]
#[path = "metrics_chrome_tests.rs"]
mod chrome_tests;

#[cfg(test)]
mod receiver_tests {
    use super::*;
    use crate::live::{source_id, testing};

    fn lay(html: &str) -> (turbo_dom::Tree, LayoutSnapshot, super::super::scroll::SourceStyles) {
        testing::layout(html, "", (400.0, 300.0), Some(1.0))
    }

    #[test]
    fn non_element_and_svg_descendant_receivers_fail_their_reads() {
        let (tree, snapshot, styles) =
            lay("<body><p id=p>text</p><svg width=20 height=20><rect id=r width=5 height=5/></svg></body>");
        let text = source_id(
            testing::DOCUMENT,
            tree.children(tree.query_selector("#p").unwrap())[0],
        );
        assert_eq!(
            element_geometry(&snapshot, text).unwrap_err(),
            "native element geometry requires an Element receiver"
        );
        assert_eq!(
            element_geometry(&snapshot, testing::id(&tree, "#r")).unwrap_err(),
            "native SVG descendant geometry is unsupported"
        );
        // offsetParent reads: a source without a style, then no source at all.
        assert_eq!(
            offset_position(&snapshot, &styles, text).unwrap_err(),
            "native offset computed style missing"
        );
        assert_eq!(
            offset_position(&snapshot, &styles, SourceId(u64::MAX)).unwrap_err(),
            "native offset source missing"
        );
    }

    #[test]
    fn an_effective_zoom_change_is_an_offset_parent() {
        // Blink: offsetParent also stops at the nearest ancestor whose effective zoom
        // differs (the embedder supplies zoom; layout here is unzoomed).
        let (tree, snapshot, mut styles) = lay(
            "<body style='margin:8px'><div id=z style='padding:6px'><div id=c style='width:5px;height:5px;margin-left:3px'></div></div></body>",
        );
        let (zoomed, child) = (testing::id(&tree, "#z"), testing::id(&tree, "#c"));
        let body = OffsetPosition {
            left: 17,
            top: 14,
            parent: Some(testing::id(&tree, "body")),
        };
        assert_eq!(offset_position(&snapshot, &styles, child).unwrap(), body);
        styles.get_mut(&child).unwrap().zoom = 2.0;
        assert_eq!(
            offset_position(&snapshot, &styles, child).unwrap(),
            OffsetPosition {
                left: 9,
                top: 6,
                parent: Some(zoomed),
            }
        );
    }

    #[test]
    fn offset_positions_round_half_up_within_the_integer_range() {
        assert_eq!(offset_integer(2.5), Ok(3));
        assert_eq!(offset_integer(-2.5), Ok(-2));
        assert_eq!(offset_integer(-2.6), Ok(-3));
        assert_eq!(offset_integer(-2_147_483_648.0), Ok(i32::MIN));
        for bad in [f32::NAN, f32::INFINITY, -3.0e9, 3.0e9] {
            assert_eq!(
                offset_integer(bad).unwrap_err(),
                "native offset position exceeds integer range"
            );
        }
    }
}
