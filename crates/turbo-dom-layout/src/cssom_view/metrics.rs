//! CSSOM measurements obtained from actual native source fragments. Missing
//! inline/caret provenance is an explicit error, never a synthetic zero box.
use turbo_dom_layout::layout_snapshot::{GeometryKind, LayoutSnapshot, Rect, SourceId, SourceKind};

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub(crate) struct ClientMetrics {
    pub(crate) left: i32,
    pub(crate) top: i32,
    pub(crate) width: i32,
    pub(crate) height: i32,
}
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub(crate) struct OffsetSize {
    pub(crate) width: i32,
    pub(crate) height: i32,
}
#[derive(Debug, PartialEq)]
pub(crate) struct ElementGeometry {
    pub(crate) bounds: Rect,
    pub(crate) rects: Vec<Rect>,
    pub(crate) client: ClientMetrics,
    pub(crate) offset: OffsetSize,
}

pub(crate) fn element_geometry(
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
    if snapshot.unsupported_svg_source_geometry(id) {
        return Err("native SVG descendant geometry is unsupported".into());
    }

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
pub(crate) fn empty_geometry() -> ElementGeometry {
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
pub(crate) fn bounding_box(rects: &[Rect]) -> Rect {
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
pub(crate) struct OffsetPosition {
    pub(crate) left: i32,
    pub(crate) top: i32,
    pub(crate) parent: Option<SourceId>,
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
pub(crate) fn offset_position(
    snapshot: &LayoutSnapshot,
    styles: &crate::live_layout_scroll::SourceStyles,
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
    if used.position == turbo_dom_layout::layout::value::Position::Fixed {
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
        let table = used.position == turbo_dom_layout::layout::value::Position::Static
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
    use crate::live_layout::{capture_tree, layout};
    use turbo_dom_layout::layout_snapshot::SnapshotInputs;
    use turbo_dom_layout::{Diagnostics, FontRegistry, ImageCtx, NoImages};
    use turbo_dom_parser::rtdom::Tree;
    fn capture(html: &str) -> (Tree, LayoutSnapshot) {
        let tree = Tree::parse(html);
        let input = capture_tree(&tree, 1).unwrap();
        let fonts = FontRegistry::new();
        let images = ImageCtx {
            resolver: &NoImages,
            body_height: None,
        };
        let snapshot = layout(
            &input,
            "",
            &SnapshotInputs {
                width: 200.0,
                height: 200.0,
                fonts: &fonts,
                images: &images,
            },
            &mut Diagnostics::default(),
        )
        .unwrap();
        (tree, snapshot)
    }
    fn id(tree: &Tree, selector: &str) -> SourceId {
        crate::live_layout::source_id(1, tree.query_selector(selector).unwrap())
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
