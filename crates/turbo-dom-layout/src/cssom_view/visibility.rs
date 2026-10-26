//! CSSOM View `checkVisibility()` over resolved flat-tree styles. Box presence is
//! decided by the caller from the actual layout snapshot.
use crate::live_layout_scroll::SourceStyles;
use turbo_dom_layout::layout_snapshot::{Rect, SourceId};
use turbo_dom_layout::ComputedStyle;

pub(crate) const OPACITY: u32 = 1;
pub(crate) const VISIBILITY: u32 = 2;
pub(crate) const CONTENT_VISIBILITY_AUTO: u32 = 4;

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
pub(crate) fn near_viewport(bounds: Rect, translation: [f32; 2], viewport: [f32; 2]) -> bool {
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
pub(crate) fn check(
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
    use crate::live_layout::{capture_tree, layout_rules_resolved, source_id};
    use turbo_dom_layout::layout_snapshot::SnapshotInputs;
    use turbo_dom_layout::{Diagnostics, FontRegistry, ImageCtx, NoImages};
    use turbo_dom_parser::rtdom::Tree;

    fn styles(html: &str) -> (SourceStyles, Tree) {
        let tree = Tree::parse(html);
        let input = capture_tree(&tree, 1).unwrap();
        let fonts = FontRegistry::new();
        let images = ImageCtx {
            resolver: &NoImages,
            body_height: None,
        };
        let (_, styles) = layout_rules_resolved(
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
}
