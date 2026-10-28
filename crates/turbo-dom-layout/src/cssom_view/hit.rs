//! Hit testing native border fragments with actual transforms, clipping and
//! per-owner scroll projection. Text hit regions need native cluster provenance.
//! Clipping is an ancestor's `overflow` (its padding box) and the `clip: rect()` of
//! an absolutely positioned box or ancestor (miaskiewicz/turbo-dom#28 item 1).
use super::scroll::{scroll_translation, SourceStyle, SourceStyles};
use crate::layout_snapshot::{GeometryKind, LayoutSnapshot, Rect, SourceId};
use std::collections::{BTreeMap, BTreeSet};
#[must_use]
pub fn contains(rect: Rect, matrix: [f64; 6], translation: [f32; 2], point: [f64; 2]) -> bool {
    let [a, b, c, d, e, f] = matrix;
    let determinant = a * d - b * c;
    if determinant == 0.0 || !determinant.is_finite() {
        return false;
    }
    let x = point[0] + f64::from(translation[0]) - e;
    let y = point[1] + f64::from(translation[1]) - f;
    let (x, y) = (
        (d * x - c * y) / determinant,
        (-b * x + a * y) / determinant,
    );
    x >= f64::from(rect.x)
        && x < f64::from(rect.x + rect.width)
        && y >= f64::from(rect.y)
        && y < f64::from(rect.y + rect.height)
}
#[must_use]
pub fn property<'a>(styles: &'a SourceStyles, mut id: SourceId, name: &str) -> Option<&'a str> {
    loop {
        let source = styles.get(&id)?;
        if let Some(value) = source.style.get(name) {
            return Some(value);
        }
        id = source.parent?;
    }
}
fn clip_axis(style: &crate::ComputedStyle, name: &str) -> bool {
    super::scroll::overflow(style, name) != "visible"
}
pub fn elements_at_point(
    snapshot: &LayoutSnapshot,
    styles: &SourceStyles,
    offsets: &BTreeMap<SourceId, [f64; 2]>,
    point: [f64; 2],
) -> Result<Vec<SourceId>, String> {
    let (width, height) = snapshot.viewport();
    if !point.iter().all(|v| v.is_finite())
        || point[0] < 0.0
        || point[1] < 0.0
        || point[0] >= f64::from(width)
        || point[1] >= f64::from(height)
    {
        return Ok(Vec::new());
    }
    let mut result = Vec::new();
    let mut seen = BTreeSet::new();
    let runs: BTreeMap<_, _> = snapshot
        .text_runs()
        .iter()
        .map(|run| (run.fragment_index, run))
        .collect();
    for fragment in snapshot.fragments().iter().rev() {
        if let Some(run) = runs.get(&fragment.index) {
            let Some(source) = run.source_id else {
                continue;
            };
            let Some(rect) = super::text::run_rect(snapshot, run) else {
                continue;
            };
            let translation = scroll_translation(snapshot, styles, source, offsets)?;
            if contains(
                rect,
                snapshot
                    .native_hit_transform(fragment.index)
                    .ok_or("native text hit transform unavailable")?,
                translation,
                point,
            ) {
                for (index, id) in run.source_ancestors.iter().rev().enumerate() {
                    if index != 0
                        && styles
                            .get(id)
                            .is_none_or(|style| style.style.get("display") != Some("inline"))
                    {
                        continue;
                    }
                    if seen.contains(id)
                        || property(styles, *id, "pointer-events") == Some("none")
                        || property(styles, *id, "visibility")
                            .is_some_and(|value| value != "visible")
                    {
                        continue;
                    }
                    if !hidden_by_clip(
                        snapshot,
                        styles,
                        (&run.source_ancestors, None),
                        offsets,
                        point,
                    )? {
                        seen.insert(*id);
                        result.push(*id);
                    }
                }
            }
        }
        if fragment.kind != GeometryKind::BorderBox {
            continue;
        }
        let Some(id) = fragment.source_id else {
            continue;
        };
        if seen.contains(&id)
            || property(styles, id, "pointer-events") == Some("none")
            || property(styles, id, "visibility").is_some_and(|value| value != "visible")
        {
            continue;
        }
        let translation = scroll_translation(snapshot, styles, id, offsets)?;
        if !contains(
            fragment.rect,
            snapshot
                .native_hit_transform(fragment.index)
                .ok_or("native hit transform unavailable")?,
            translation,
            point,
        ) {
            continue;
        }
        let clipped = hidden_by_clip(
            snapshot,
            styles,
            (&fragment.source_ancestors, Some(id)),
            offsets,
            point,
        )?;
        if !clipped {
            seen.insert(id);
            result.push(id);
        }
    }
    if let Some((&root, _)) = styles
        .iter()
        .find(|(_, style)| style.parent.is_none() && style.tag == "html")
    {
        if seen.insert(root) {
            result.push(root);
        }
    }
    Ok(result)
}
/// Whether `point` is clipped away for a box with `ancestors` (and `own` box): by an
/// ancestor's `overflow` ([`clipped`]) or the `clip: rect()` of an absolutely
/// positioned ancestor or of the box itself (miaskiewicz/turbo-dom#28 item 1).
pub fn hidden_by_clip(
    snapshot: &LayoutSnapshot,
    styles: &SourceStyles,
    (ancestors, own): (&[SourceId], Option<SourceId>),
    offsets: &BTreeMap<SourceId, [f64; 2]>,
    point: [f64; 2],
) -> Result<bool, String> {
    for &id in ancestors.iter().chain(own.as_ref()) {
        if clip_rect_excludes(snapshot, styles, id, offsets, point)? {
            return Ok(true);
        }
    }
    clipped(snapshot, styles, ancestors, offsets, point)
}

/// Whether an ancestor's `overflow` clips `point` away.
pub fn clipped(
    snapshot: &LayoutSnapshot,
    styles: &SourceStyles,
    ancestors: &[SourceId],
    offsets: &BTreeMap<SourceId, [f64; 2]>,
    point: [f64; 2],
) -> Result<bool, String> {
    let mut clipped = false;
    for ancestor in ancestors {
        let Some(style) = styles.get(ancestor) else {
            continue;
        };
        if style.style.get("display") == Some("contents") {
            continue;
        }
        let clip_x = clip_axis(&style.style, "overflow-x");
        let clip_y = clip_axis(&style.style, "overflow-y");
        if !clip_x && !clip_y {
            continue;
        }
        if clip_x != clip_y {
            return Err("native hit testing asymmetric overflow clipping pending".into());
        }
        let source = snapshot
            .source(*ancestor)
            .ok_or("native hit clip source missing")?;
        let border = source
            .fragment_indices
            .iter()
            .map(|&index| &snapshot.fragments()[index])
            .find(|fragment| fragment.kind == GeometryKind::BorderBox)
            .ok_or("native hit clip border missing")?;
        let used = border.used_box.ok_or("native hit clip used box missing")?;
        let translation = scroll_translation(snapshot, styles, *ancestor, offsets)?;
        if !contains(
            used.padding_rect(border.rect),
            snapshot
                .native_hit_transform(border.index)
                .ok_or("native clip transform unavailable")?,
            translation,
            point,
        ) {
            clipped = true;
            break;
        }
    }
    Ok(clipped)
}
/// Whether `id`'s `clip: rect()` (it and its descendants) leaves `point` out.
fn clip_rect_excludes(
    snapshot: &LayoutSnapshot,
    styles: &SourceStyles,
    id: SourceId,
    offsets: &BTreeMap<SourceId, [f64; 2]>,
    point: [f64; 2],
) -> Result<bool, String> {
    let Some(style) = styles.get(&id) else {
        return Ok(false);
    };
    let Some(border) = snapshot.source(id).and_then(|source| {
        source
            .fragment_indices
            .iter()
            .map(|&index| &snapshot.fragments()[index])
            .find(|fragment| fragment.kind == GeometryKind::BorderBox)
    }) else {
        return Ok(false);
    };
    let Some(clip) = clip_rect(style, border.rect) else {
        return Ok(false);
    };
    let translation = scroll_translation(snapshot, styles, id, offsets)?;
    Ok(!contains(
        clip,
        snapshot
            .native_hit_transform(border.index)
            .ok_or("native clip transform unavailable")?,
        translation,
        point,
    ))
}

/// The region an absolutely positioned box's `clip: rect(top, right, bottom, left)`
/// keeps (CSS 2.1 §11.1.2): offsets from its border box's top-left corner, `auto` the
/// border-box edge, commas optional, lengths in zoomed pixels. `None` when it does not
/// clip: `clip: auto`, an invalid value, or a box that is not absolutely positioned
/// (`clip` does not apply). The parse is layout's (`value::clip_region`).
fn clip_rect(style: &SourceStyle, border: Rect) -> Option<Rect> {
    let clip =
        crate::layout::value::clip_region(&style.style, border.width, border.height, style.zoom)?;
    Some(Rect {
        x: border.x + clip.x,
        y: border.y + clip.y,
        width: clip.width,
        height: clip.height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_inverse_transform_excludes_axis_bounds_and_degenerate_boxes() {
        let rect = Rect {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 20.0,
        };
        assert!(contains(
            rect,
            [1.0, 0.0, 0.0, 1.0, 3.0, 4.0],
            [0.0; 2],
            [3.0, 4.0]
        ));
        assert!(!contains(
            rect,
            [1.0, 0.0, 0.0, 1.0, 3.0, 4.0],
            [0.0; 2],
            [13.0, 4.0]
        ));
        assert!(contains(
            rect,
            [0.0, 1.0, -1.0, 0.0, 30.0, 0.0],
            [0.0; 2],
            [25.0, 5.0]
        ));
        assert!(!contains(
            rect,
            [0.0, 1.0, -1.0, 0.0, 30.0, 0.0],
            [0.0; 2],
            [10.0, 5.0]
        ));
        assert!(!contains(rect, [0.0; 6], [0.0; 2], [0.0, 0.0]));
    }

    fn styled(pairs: &[(&str, &str)], zoom: f32) -> SourceStyle {
        SourceStyle {
            style: crate::ComputedStyle::from_pairs(pairs.iter().copied()),
            parent: None,
            tag: "span".into(),
            zoom,
        }
    }

    #[test]
    fn clip_rect_resolves_offsets_auto_edges_units_and_zoom() {
        let border = Rect {
            x: 10.0,
            y: 20.0,
            width: 50.0,
            height: 40.0,
        };
        let clip = |pairs: &[(&str, &str)], zoom| clip_rect(&styled(pairs, zoom), border);
        let rect = |x, y, width, height| Rect {
            x,
            y,
            width,
            height,
        };
        let abs = ("position", "absolute");
        assert_eq!(
            clip(&[abs, ("clip", "rect(1px, 30px, 20px, 5px)")], 1.0),
            Some(rect(15.0, 21.0, 25.0, 19.0))
        );
        // `auto` is the border-box edge; commas are optional; `em` uses the font size.
        assert_eq!(
            clip(
                &[("position", "fixed"), ("clip", "rect(auto auto auto auto)")],
                1.0
            ),
            Some(border)
        );
        assert_eq!(
            clip(
                &[abs, ("font-size", "10px"), ("clip", "rect(1em,2em,3em,0)")],
                1.0
            ),
            Some(rect(10.0, 30.0, 20.0, 20.0))
        );
        // A right/bottom edge before the left/top one keeps nothing.
        assert_eq!(
            clip(&[abs, ("clip", "rect(0,0,0,0)")], 1.0),
            Some(rect(10.0, 20.0, 0.0, 0.0))
        );
        assert_eq!(
            clip(&[abs, ("clip", "rect(10px,5px,5px,10px)")], 1.0),
            Some(rect(20.0, 30.0, 0.0, 0.0))
        );
        // Lengths scale with the effective zoom (the layout is in zoomed pixels).
        assert_eq!(
            clip(&[abs, ("clip", "rect(1px,2px,3px,0)")], 2.0),
            Some(rect(10.0, 22.0, 4.0, 4.0))
        );
    }

    #[test]
    fn clip_rect_does_not_clip_when_it_does_not_apply_or_is_invalid() {
        let border = Rect {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
        };
        let clip = |pairs: &[(&str, &str)]| clip_rect(&styled(pairs, 1.0), border);
        let abs = ("position", "absolute");
        for pairs in [
            &[("clip", "rect(0,0,0,0)")][..],
            &[("position", "relative"), ("clip", "rect(0,0,0,0)")],
            &[abs],
            &[abs, ("clip", "auto")],
            &[abs, ("clip", "rect(0,0,0)")],
            &[abs, ("clip", "rect(0,0,0,0,0)")],
            &[abs, ("clip", "rect(0,0,0,0")],
            &[abs, ("clip", "rect(10%,0,0,0)")],
            &[abs, ("clip", "rect(calc(10% + 1px),0,0,0)")],
            &[abs, ("clip", "rect(x,0,0,0)")],
        ] {
            assert_eq!(clip(pairs), None, "{pairs:?}");
        }
        // An unparsable font size falls back to the 16px initial value.
        assert_eq!(
            clip(&[abs, ("font-size", "big"), ("clip", "rect(0,1em,1em,0)")]),
            Some(Rect {
                x: 0.0,
                y: 0.0,
                width: 16.0,
                height: 16.0
            })
        );
    }
}

#[cfg(test)]
mod clip_tests {
    use super::*;
    use crate::live::testing;

    #[test]
    fn containment_inverts_a_scaled_translated_matrix_after_the_scroll_translation() {
        // World x in [10, 30), y in [20, 60); the point is offset by the scroll
        // translation [5, -5] before the inverse transform.
        let rect = Rect {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
        };
        let matrix = [2.0, 0.0, 0.0, 4.0, 10.0, 20.0];
        let at = |x: f64, y: f64| contains(rect, matrix, [5.0, -5.0], [x, y]);
        assert!(at(6.0, 26.0)); // local (0.5, 0.25)
        assert!(at(5.0, 25.0)); // local (0, 0): the start edges are inside
        assert!(!at(4.0, 26.0)); // local x -0.5
        assert!(!at(6.0, 24.0)); // local y -0.25
        assert!(!at(25.0, 26.0)); // local x 10: the end edge is outside
        assert!(!at(6.0, 65.0)); // local y 10
        assert!(at(24.0, 64.0)); // local (9.5, 9.75)
                                 // A quarter turn: [a b c d] = [0 1 -1 0] maps local (x, y) to (-y, x).
        let rotated = [0.0, 1.0, -1.0, 0.0, 0.0, 0.0];
        assert!(contains(rect, rotated, [0.0; 2], [-2.0, 3.0]));
        assert!(!contains(rect, rotated, [0.0; 2], [2.0, 3.0]));
        let sheared = [1.0, 0.5, 0.0, 1.0, 0.0, 0.0];
        // Shear y by x/2: world (4, 2.5) is local (4, 0.5); world (4, 1.5) local (4, -0.5).
        assert!(contains(rect, sheared, [0.0; 2], [4.0, 2.5]));
        assert!(!contains(rect, sheared, [0.0; 2], [4.0, 1.5]));
    }

    #[test]
    fn the_root_element_is_the_parentless_html_element_last_hit() {
        let (tree, snapshot, mut styles) = testing::layout(
            "<body style='margin:0'></body>",
            "",
            (200.0, 200.0),
            Some(1.0),
        );
        let html = testing::id(&tree, "html");
        let body = testing::id(&tree, "body");
        assert_eq!(
            elements_at_point(&snapshot, &styles, &BTreeMap::new(), [5.0, 150.0]),
            Ok(vec![html])
        );
        // Another parentless (non-html) style, ordered first, is not the root.
        let other = SourceStyle {
            style: styles[&body].style.clone(),
            parent: None,
            tag: "div".into(),
            zoom: 1.0,
        };
        styles.insert(SourceId(0), other);
        assert_eq!(
            elements_at_point(&snapshot, &styles, &BTreeMap::new(), [5.0, 150.0]),
            Ok(vec![html])
        );
    }

    #[test]
    fn unknown_ancestors_never_clip_and_one_axis_clipping_is_an_explicit_gap() {
        let (tree, snapshot, styles) = testing::layout(
            "<body style='margin:0'><div id=x style='width:50px;height:50px;overflow-x:clip'><div id=k style='width:90px;height:90px'></div></div></body>",
            "",
            (200.0, 200.0),
            Some(1.0),
        );
        let offsets = BTreeMap::new();
        let unknown = SourceId(u64::MAX - 1);
        // A chain entry the styles do not know (no style, no box) clips nothing.
        assert_eq!(
            hidden_by_clip(
                &snapshot,
                &styles,
                (&[unknown], Some(unknown)),
                &offsets,
                [70.0, 10.0]
            ),
            Ok(false)
        );
        assert_eq!(
            clipped(&snapshot, &styles, &[unknown], &offsets, [70.0, 10.0]),
            Ok(false)
        );
        // `overflow-x: clip` alone clips one axis only (`overflow-y` stays visible):
        // Chrome clips horizontally; native hit testing reports the gap.
        let x = testing::id(&tree, "#x");
        assert_eq!(
            clipped(&snapshot, &styles, &[x], &offsets, [70.0, 10.0]).unwrap_err(),
            "native hit testing asymmetric overflow clipping pending"
        );
        let k = testing::id(&tree, "#k");
        assert_eq!(
            elements_at_point(&snapshot, &styles, &offsets, [70.0, 10.0]).unwrap_err(),
            "native hit testing asymmetric overflow clipping pending"
        );
        // The box itself is not its own clipping ancestor.
        assert_eq!(
            clipped(&snapshot, &styles, &[k], &offsets, [70.0, 10.0]),
            Ok(false)
        );
    }
}
