//! CSSOM View `checkVisibility()` over resolved flat-tree styles. Box presence is
//! decided by the caller from the actual layout snapshot.
use super::scroll::SourceStyles;
use crate::layout_snapshot::{Rect, SourceId};
use crate::ComputedStyle;

pub const OPACITY: u32 = 1;
pub const VISIBILITY: u32 = 2;
pub const CONTENT_VISIBILITY_AUTO: u32 = 4;

fn keyword<'a>(style: &'a ComputedStyle, property: &str) -> Option<&'a str> {
    style.get(property).map(str::trim)
}

fn transparent(style: &ComputedStyle) -> Result<bool, String> {
    let Some(value) = keyword(style, "opacity") else {
        return Ok(false);
    };
    let parsed = match value.strip_suffix('%') {
        Some(percent) => percent.trim().parse::<f64>().map(|value| value / 100.0),
        None => value.parse::<f64>(),
    };
    // Unresolved expressions are rejected rather than guessed visible.
    let opacity = parsed.map_err(|_| "native computed opacity is unresolved".to_string())?;
    Ok(!opacity.is_nan() && opacity <= 0.0)
}

fn invisible(style: &ComputedStyle) -> bool {
    keyword(style, "visibility").is_some_and(|value| {
        value.eq_ignore_ascii_case("hidden") || value.eq_ignore_ascii_case("collapse")
    })
}

/// Chromium decides content-visibility:auto relevance with a 50% viewport margin.
#[must_use]
pub fn near_viewport(bounds: Rect, translation: [f32; 2], viewport: [f32; 2]) -> bool {
    let [width, height] = viewport;
    let x = bounds.x - translation[0];
    let y = bounds.y - translation[1];
    x <= width * 1.5
        && x + bounds.width >= -width * 0.5
        && y <= height * 1.5
        && y + bounds.height >= -height * 0.5
}

/// Steps after the box check: skipped content, then the optional opacity,
/// visibility and content-visibility:auto filters.
pub fn check(
    styles: &SourceStyles,
    id: SourceId,
    flags: u32,
    mut relevant: impl FnMut(SourceId) -> Result<bool, String>,
) -> Result<bool, String> {
    let own = styles
        .get(&id)
        .ok_or("native visibility computed style missing")?;
    if flags & VISIBILITY != 0 && invisible(&own.style) {
        return Ok(false);
    }
    if flags & OPACITY != 0 && transparent(&own.style)? {
        return Ok(false);
    }
    let mut parent = own.parent;
    let mut depth = 0usize;
    while let Some(ancestor) = parent {
        depth += 1;
        if depth > 4096 {
            return Err("native visibility ancestry exceeds its depth limit".into());
        }
        let entry = styles
            .get(&ancestor)
            .ok_or("native visibility ancestor style missing")?;
        let content = keyword(&entry.style, "content-visibility");
        if content.is_some_and(|value| value.eq_ignore_ascii_case("hidden")) {
            return Ok(false);
        }
        if flags & OPACITY != 0 && transparent(&entry.style)? {
            return Ok(false);
        }
        if flags & CONTENT_VISIBILITY_AUTO != 0
            && content.is_some_and(|value| value.eq_ignore_ascii_case("auto"))
            && !relevant(ancestor)?
        {
            return Ok(false);
        }
        parent = entry.parent;
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::live::{source_id, testing};
    use turbo_dom::rtdom::Tree;

    // Adapted@S4: styled by the live-DOM input instead of turbo-surf's copier.
    fn styles(html: &str) -> (SourceStyles, Tree) {
        let (tree, _, styles) = testing::layout(html, "", (800.0, 600.0), Some(2.0));
        (styles, tree)
    }

    #[test]
    fn opacity_and_visibility_filters_follow_resolved_flat_styles() {
        let (styles, tree) = styles(
            "<body><div id=a style='opacity:0%'><i id=b></i></div><p id=c style='opacity:0.25;visibility:collapse'></p><p id=d style='opacity:var(--missing)'></p></body>",
        );
        let id = |selector| source_id(1, tree.query_selector(selector).unwrap());
        let visible = |selector, flags| check(&styles, id(selector), flags, |_| Ok(true));
        assert!(visible("#b", 0).unwrap());
        assert!(!visible("#b", OPACITY).unwrap());
        assert!(visible("#c", OPACITY).unwrap());
        assert!(!visible("#c", VISIBILITY).unwrap());
        // Unresolved expressions are rejected instead of being guessed visible.
        assert!(visible("#d", 0).unwrap());
        assert!(visible("#d", OPACITY).is_err());
    }

    #[test]
    fn relevance_uses_the_half_viewport_proximity_margin() {
        let rect = |y| Rect {
            x: 0.0,
            y,
            width: 10.0,
            height: 10.0,
        };
        assert!(near_viewport(rect(1199.0), [0.0; 2], [1280.0, 800.0]));
        assert!(!near_viewport(rect(1201.0), [0.0; 2], [1280.0, 800.0]));
        assert!(near_viewport(rect(1201.0), [0.0, 10.0], [1280.0, 800.0]));
        assert!(!near_viewport(rect(-411.0), [0.0; 2], [1280.0, 800.0]));
    }
    /// turbo-surf's `checkVisibility()`: an element with a box, then [`check`] with
    /// content-visibility:auto relevance from the scrolled border box.
    fn chrome_check(case: &crate::cssom_view::scroll::chrome::Case, cid: &str, flags: u32) -> bool {
        use crate::cssom_view::scroll::scroll_translation;
        let id = case.id(cid);
        let offsets = case.offsets();
        let has_box = !case.snapshot.source(id).unwrap().excluded_from_layout
            && case.styles[&id].style.get("display") != Some("contents");
        has_box
            && check(&case.styles, id, flags, |ancestor| {
                let bounds =
                    crate::cssom_view::metrics::element_geometry(&case.snapshot, ancestor)?.bounds;
                let translation =
                    scroll_translation(&case.snapshot, &case.styles, ancestor, &offsets)?;
                Ok(near_viewport(bounds, translation, [1280.0, 800.0]))
            })
            .unwrap()
    }

    #[test]
    fn check_visibility_matches_chrome_156_for_every_flag_combination() {
        for id in ["check-visibility", "check-visibility-scrolled-relevance"] {
            let case = crate::cssom_view::scroll::chrome::case(id);
            for want in case.reads("visibility") {
                let cid = want["cid"].as_str().unwrap();
                let got: Vec<bool> = (0..8)
                    .map(|flags| chrome_check(&case, cid, flags))
                    .collect();
                let chrome: Vec<bool> = want["flags"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_bool().unwrap())
                    .collect();
                assert_eq!(got, chrome, "{id}: {cid}");
            }
        }
    }

    #[test]
    fn missing_styles_and_runaway_ancestry_are_errors() {
        let (mut styles, tree) = styles("<body><div id=a><i id=b></i></div></body>");
        let id = |selector| source_id(1, tree.query_selector(selector).unwrap());
        let a = id("#a");
        assert_eq!(
            check(&styles, SourceId(7), 0, |_| Ok(true)).unwrap_err(),
            "native visibility computed style missing"
        );
        let mut orphan = styles.remove(&a).unwrap();
        assert_eq!(
            check(&styles, id("#b"), 0, |_| Ok(true)).unwrap_err(),
            "native visibility ancestor style missing"
        );
        // A cycle never terminates by itself: the depth limit stops it.
        orphan.parent = Some(a);
        styles.insert(a, orphan);
        assert_eq!(
            check(&styles, a, 0, |_| Ok(true)).unwrap_err(),
            "native visibility ancestry exceeds its depth limit"
        );
        // The relevance callback's error propagates.
        let (styles, tree) = styles_auto();
        let b = source_id(1, tree.query_selector("#b").unwrap());
        assert_eq!(
            check(
                &styles,
                b,
                CONTENT_VISIBILITY_AUTO,
                |_| Err("no box".into())
            )
            .unwrap_err(),
            "no box"
        );
        assert!(check(&styles, b, 0, |_| Err("no box".into())).unwrap());
    }

    fn styles_auto() -> (SourceStyles, Tree) {
        styles("<body><div style='content-visibility:auto'><i id=b></i></div></body>")
    }

    /// The ancestry walk allows 4096 ancestors and fails beyond.
    #[test]
    fn the_ancestry_depth_limit_is_4096() {
        let chain = |ancestors: u64| -> SourceStyles {
            (0..=ancestors)
                .map(|i| {
                    (
                        SourceId(i),
                        crate::cssom_view::scroll::SourceStyle {
                            style: ComputedStyle::default(),
                            parent: i.checked_sub(1).map(SourceId),
                            tag: "div".into(),
                            zoom: 1.0,
                        },
                    )
                })
                .collect()
        };
        assert!(check(&chain(4096), SourceId(4096), 0, |_| Ok(true)).unwrap());
        assert_eq!(
            check(&chain(4097), SourceId(4097), 0, |_| Ok(true)).unwrap_err(),
            "native visibility ancestry exceeds its depth limit"
        );
    }

    #[test]
    fn near_viewport_bounds_are_inclusive_on_every_edge() {
        let rect = |x, y| Rect {
            x,
            y,
            width: 10.0,
            height: 10.0,
        };
        let viewport = [100.0, 200.0];
        assert!(near_viewport(rect(150.0, 0.0), [0.0; 2], viewport));
        assert!(!near_viewport(rect(150.1, 0.0), [0.0; 2], viewport));
        assert!(near_viewport(rect(-60.0, 0.0), [0.0; 2], viewport));
        assert!(!near_viewport(rect(-60.1, 0.0), [0.0; 2], viewport));
        assert!(near_viewport(rect(0.0, 300.0), [0.0; 2], viewport));
        assert!(!near_viewport(rect(0.0, 300.1), [0.0; 2], viewport));
        assert!(near_viewport(rect(0.0, -110.0), [0.0; 2], viewport));
        assert!(!near_viewport(rect(0.0, -110.1), [0.0; 2], viewport));
        assert!(near_viewport(rect(160.0, 0.0), [10.0, 0.0], viewport));
    }
}
