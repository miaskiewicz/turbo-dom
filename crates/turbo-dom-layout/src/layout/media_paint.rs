//! Chrome 156's `<audio controls>` panel (miaskiewicz/turbo-dom#292), drawn with the
//! galley's box and text primitives over the audio box's content box, as Chrome paints
//! it for an element with no playable source: a `#f1f3f4` pill 54px tall at the bottom of
//! the content box holding, from the left, a greyed-out play button, the `0:00 / 0:00`
//! time display, the timeline (a 4px rounded `#c0c2c3` bar), a greyed-out volume button
//! and the black three-dot menu button. The play button and time display keep their
//! place from the left, the volume and menu buttons theirs from the right, and the
//! timeline stretches between them; a panel narrower than Chrome's default 300px leaves
//! the time display and timeline out (Chrome reflows them), and a right-hand button
//! that would reach the play button is left out. A content box under 54px tall paints
//! no panel (Chrome clips it). The time display is set in
//! the registry's `system-ui` / `sans-serif` face (Chrome's is the host's system font).
//! A vertical audio box paints no panel. Measured against the form-native Chrome 156
//! control's `audio-*` cases (`scripts/form-native-chrome-control.mjs`).
use super::fragment::{
    BorderRadii, ClipRect, Fragment, FragmentContent, NodeId, PositionedGlyph, Transform2D,
};
use super::value::{BorderEdges, BorderSide, Rgba};
use crate::FontRegistry;

const PANEL: Rgba = rgb(0xf1, 0xf3, 0xf4);
/// A disabled button's glyph.
const DISABLED: Rgba = rgb(0xa8, 0xaa, 0xaa);
const TIMELINE: Rgba = rgb(0xc0, 0xc2, 0xc3);
/// The time display's text: black at 87%.
const TEXT: Rgba = Rgba {
    r: 0,
    g: 0,
    b: 0,
    a: 222,
};
const CLEAR: Rgba = Rgba {
    r: 0,
    g: 0,
    b: 0,
    a: 0,
};
/// The panel's height.
const HEIGHT: f32 = 54.0;

const fn rgb(r: u8, g: u8, b: u8) -> Rgba {
    Rgba { r, g, b, a: 255 }
}

/// Chrome 156's panel-width thresholds (miaskiewicz/turbo-dom#346): the play button,
/// current-time, time-remaining, volume and menu are each dropped below the width
/// measured on the `-webkit-media-controls-*` DOM boxes.
const SHOW_MENU: f32 = 68.0;
const SHOW_PLAY: f32 = 100.0;
const SHOW_VOLUME: f32 = 143.0;
const SHOW_CURRENT_TIME: f32 = 175.0;
const SHOW_TIME_REMAINING: f32 = 211.0;

/// Add the panel to the audio box `frag`, whose content box is `(x, y, w, h)`. In a
/// horizontal writing mode the panel spans the content width at the block end; in a
/// vertical writing mode it is a 54 x 54 pill at the content box's physical bottom
/// (Chrome 156's `-webkit-media-controls-enclosure`, measured via
/// `scripts/form-native-chrome-control.mjs`'s `audio-panel-vertical-rl`). A box under
/// 54px tall still paints the pill, clipped to the box (miaskiewicz/turbo-dom#346).
pub(crate) fn audio_panel(
    frag: &mut Fragment,
    (x, y, w, h): (f32, f32, f32, f32),
    vertical: bool,
    fonts: &FontRegistry,
) {
    let ph = h.min(HEIGHT);
    let pw = if vertical { w.min(HEIGHT) } else { w };
    let pill_top = y + h - ph;
    let id = frag.node_id;
    let at = |dx: f32, dy: f32| (x + dx, pill_top + dy);
    let pill_at = |dx: f32, dy: f32| (x + dx, pill_top + dy);
    let mut parts = vec![rounded(id, pill_at(0.0, 0.0), (pw, ph), PANEL, ph / 2.0)];
    // A short panel (vertical mode, a thin box or a short box): just the pill.
    if pw < SHOW_MENU || ph < HEIGHT {
        frag.children.extend(parts);
        return;
    }
    if pw >= SHOW_PLAY {
        parts.push(play(id, at(27.05, 27.0)));
    }
    if pw >= SHOW_TIME_REMAINING {
        parts.extend(time_display(id, at(47.0, 32.0), fonts));
    } else if pw >= SHOW_CURRENT_TIME {
        parts.extend(current_time(id, at(47.0, 32.0), fonts));
    }
    // The timeline: between the left items (play / time) and the right items (volume /
    // menu); its position and width depend on which items are drawn (Chrome 156's
    // `-webkit-media-controls-timeline`, see `audio_scan.mjs`). Below the `SHOW_PLAY`
    // threshold, it starts 10px from the panel's inline start.
    let timeline_start = if pw >= SHOW_TIME_REMAINING {
        128.4
    } else if pw >= SHOW_CURRENT_TIME {
        89.25
    } else if pw >= SHOW_PLAY {
        42.0
    } else {
        10.0
    };
    // The right side: menu is always drawn at `pw - 32`, volume (when shown) at
    // `pw - 74`; the timeline ends 4px before the leftmost right-side item.
    // Where the timeline must end (its inline end, physical left edge of the rightmost
    // right-side item): just before the volume button when it is shown, else just before
    // the menu's dots. The timeline takes at least 32px.
    let right_items_start = if pw >= SHOW_VOLUME {
        pw - 90.4
    } else {
        pw - 32.0
    };
    let timeline_end = right_items_start.max(timeline_start + 32.0);
    parts.push(rounded(
        id,
        at(timeline_start, 25.0),
        (timeline_end - timeline_start, 4.0),
        TIMELINE,
        2.0,
    ));
    if pw >= SHOW_VOLUME {
        parts.extend(volume(id, at(pw - 65.6, 0.0)));
    }
    for dy in [20.0, 25.0, 30.0] {
        parts.push(rounded(id, at(pw - 28.0, dy), (4.0, 4.0), Rgba::BLACK, 2.0));
    }
    frag.children.extend(parts);
}

fn shape(background: Option<Rgba>, border: BorderEdges, radius: f32) -> FragmentContent {
    FragmentContent::Box {
        background,
        border,
        border_radius: BorderRadii::uniform(radius),
        shadow: None,
        gradient: None,
        transform: None,
    }
}

fn rounded(
    id: NodeId,
    (x, y): (f32, f32),
    (w, h): (f32, f32),
    colour: Rgba,
    radius: f32,
) -> Fragment {
    Fragment::new(
        id,
        x,
        y,
        w,
        h,
        shape(Some(colour), BorderEdges::default(), radius),
    )
}

/// A ring of `width`px about the centre `(cx, cy)`, `radius` to its outer edge.
fn ring(id: NodeId, (cx, cy): (f32, f32), radius: f32, width: u16) -> Fragment {
    let side = BorderSide {
        width,
        color: Some(DISABLED),
    };
    let border = BorderEdges {
        top: side,
        right: side,
        bottom: side,
        left: side,
    };
    let d = 2.0 * radius;
    Fragment::new(
        id,
        cx - radius,
        cy - radius,
        d,
        d,
        shape(None, border, radius),
    )
}

/// A trapezoid: the bottom border band of a `w` x `depth` box whose transparent left and
/// right borders are `inset` wide (its far edge `w - 2 * inset` long; a triangle at
/// `w / 2`), centred at `(cx, cy)` and transformed by `matrix` about its middle.
fn trapezoid(
    id: NodeId,
    (cx, cy): (f32, f32),
    (w, depth): (f32, u16),
    inset: u16,
    matrix: [f32; 4],
) -> Fragment {
    let h = f32::from(depth);
    let clear = BorderSide {
        width: inset,
        color: Some(CLEAR),
    };
    let border = BorderEdges {
        top: BorderSide::default(),
        right: clear,
        bottom: BorderSide {
            width: depth,
            color: Some(DISABLED),
        },
        left: clear,
    };
    let [a, b, c, d] = matrix;
    let content = FragmentContent::Box {
        background: None,
        border,
        border_radius: BorderRadii::ZERO,
        shadow: None,
        gradient: None,
        transform: Some(Transform2D {
            matrix: [a, b, c, d, 0.0, 0.0],
            origin_x: w / 2.0,
            origin_y: h / 2.0,
        }),
    };
    Fragment::new(id, cx - w / 2.0, cy - h / 2.0, w, h, content)
}

/// The play button: a triangle 9px deep and 11px tall pointing right, centred at `at`
/// (a 10 x 9 upward triangle turned a quarter clockwise and stretched 1.1 times along
/// its base).
fn play(id: NodeId, at: (f32, f32)) -> Fragment {
    trapezoid(id, at, (10.0, 9), 5, [0.0, 1.1, -1.0, 0.0])
}

/// The volume button, its left edge at `x` (panel top `y`): the speaker's 3.6 x 5.2 body,
/// its cone widening over 4px to 13.6px (a band turned to face right) and two sound-wave
/// arcs right of the cone.
fn volume(id: NodeId, (x, y): (f32, f32)) -> [Fragment; 3] {
    let cy = y + 27.0;
    let mut waves = Fragment::new(
        id,
        x + 8.4,
        y,
        9.0,
        HEIGHT,
        shape(None, BorderEdges::default(), 0.0),
    );
    waves.clip = Some(ClipRect {
        x: 0.0,
        y: 0.0,
        width: 9.0,
        height: HEIGHT,
        radii: BorderRadii::ZERO,
    });
    let centre = (x + 8.1, cy - 0.5);
    waves.children = vec![ring(id, centre, 3.2, 2), ring(id, centre, 7.2, 2)];
    [
        rounded(id, (x, cy - 2.6), (3.6, 5.2), DISABLED, 0.0),
        trapezoid(id, (x + 5.6, cy), (13.6, 4), 4, [0.0, -1.0, 1.0, 0.0]),
        waves,
    ]
}

/// The current-time-only display (`0:00`), a width below `SHOW_TIME_REMAINING` leaves
/// out the `/ 0:00` suffix (Chrome 156's `-webkit-media-controls-current-time-display`).
fn current_time(id: NodeId, at: (f32, f32), fonts: &FontRegistry) -> Option<Fragment> {
    text_fragment(id, at, "0:00", fonts)
}

/// The `0:00 / 0:00` time display, its text starting at `x` on the baseline `y`.
fn time_display(id: NodeId, (x, baseline): (f32, f32), fonts: &FontRegistry) -> Option<Fragment> {
    text_fragment(id, (x, baseline), "0:00 / 0:00", fonts)
}

/// A text fragment at `(x, baseline)` of the panel's text style
/// (`system-ui`/`sans-serif`, 14px, black at 87%).
fn text_fragment(
    id: NodeId,
    (x, baseline): (f32, f32),
    label: &str,
    fonts: &FontRegistry,
) -> Option<Fragment> {
    let text = label;
    let face = fonts.select(&["system-ui", "sans-serif"], 400, false)?;
    let size = 14.0;
    let scale = face.scale(size);
    let ascent = face.ascent_px(size);
    let mut pen = 0.0;
    let glyphs = face
        .shape(text)
        .into_iter()
        .map(|g| {
            let start = g.cluster;
            let glyph = PositionedGlyph {
                glyph_id: g.glyph_id,
                x: pen + g.x_offset as f32 * scale,
                y: ascent - g.y_offset as f32 * scale,
                text_start: start,
                text_end: start + 1,
            };
            pen += g.x_advance as f32 * scale;
            glyph
        })
        .collect();
    let content = FragmentContent::TextLine {
        glyphs,
        text: text.to_string(),
        face: face.clone(),
        font_size: size,
        color: TEXT,
        vertical: false,
        upright: false,
    };
    Some(Fragment::new(
        id,
        x,
        baseline - ascent,
        pen,
        ascent + face.descent_px(size),
        content,
    ))
}

#[cfg(test)]
mod tests {
    use super::super::fragment::{Fragment, FragmentContent};
    use super::{audio_panel, DISABLED, PANEL, TIMELINE};
    use crate::Rgba;

    /// The audio box's panel parts: `(x, y, w, h, background, turned, clipped)`, and
    /// whether a text line is among them.
    type Part = (f32, f32, f32, f32, Option<Rgba>, bool, bool);

    fn audio(style: &str) -> (Vec<Part>, bool) {
        fn walk(f: &Fragment, out: &mut (Vec<Part>, bool)) {
            match &f.content {
                FragmentContent::Box {
                    background,
                    transform,
                    ..
                } => out.0.push((
                    f.x,
                    f.y,
                    f.width,
                    f.height,
                    *background,
                    transform.is_some(),
                    f.clip.is_some(),
                )),
                FragmentContent::TextLine { text, .. } => out.1 |= text == "0:00 / 0:00",
                _ => {}
            }
            f.children.iter().for_each(|c| walk(c, out));
        }
        let html = format!(
            "<!DOCTYPE html><style>body{{margin:0}}</style><audio controls style='position:absolute;left:0;top:0;{style}'></audio>"
        );
        let fonts = crate::test_support::fixture_fonts();
        let mut diags = crate::Diagnostics::default();
        let galley = crate::layout_html(&html, "", 800.0, &fonts, &mut diags).unwrap();
        let mut out = (Vec::new(), false);
        walk(&galley, &mut out);
        out
    }

    fn close(p: &Part, (x, y, w, h): (f32, f32, f32, f32)) -> bool {
        [(p.0, x), (p.1, y), (p.2, w), (p.3, h)]
            .iter()
            .all(|(a, b)| (a - b).abs() < 0.01)
    }

    #[test]
    fn the_default_panel_holds_chromes_buttons_and_timeline() {
        let (parts, text) = audio("");
        assert!(text, "{parts:?}");
        let find =
            |rect, colour: Option<Rgba>| parts.iter().any(|p| close(p, rect) && p.4 == colour);
        assert!(find((0.0, 0.0, 300.0, 54.0), Some(PANEL)), "{parts:?}");
        assert!(find((128.4, 25.0, 81.2, 4.0), Some(TIMELINE)), "{parts:?}");
        // The speaker's body, and the turned play triangle and cone.
        assert!(find((234.4, 24.4, 3.6, 5.2), Some(DISABLED)), "{parts:?}");
        assert!(parts
            .iter()
            .any(|p| close(p, (22.05, 22.5, 10.0, 9.0)) && p.5));
        assert!(parts
            .iter()
            .any(|p| close(p, (233.2, 25.0, 13.6, 4.0)) && p.5));
        // The waves' clip right of the cone, and the menu's three dots.
        assert!(parts
            .iter()
            .any(|p| close(p, (242.8, 0.0, 9.0, 54.0)) && p.6));
        for y in [20.0, 25.0, 30.0] {
            assert!(find((272.0, y, 4.0, 4.0), Some(Rgba::BLACK)), "{parts:?}");
        }
        // A wider panel stretches the timeline and keeps the right-hand buttons right.
        let (wide, _) = audio("width:400px");
        assert!(wide.iter().any(|p| close(p, (128.4, 25.0, 181.2, 4.0))));
        assert!(wide.iter().any(|p| close(p, (372.0, 30.0, 4.0, 4.0))));
        // A taller box: the panel at its bottom.
        let (tall, _) = audio("height:80px");
        assert!(tall
            .iter()
            .any(|p| close(p, (0.0, 26.0, 300.0, 54.0)) && p.4 == Some(PANEL)));
    }

    #[test]
    fn a_small_panel_leaves_out_what_does_not_fit() {
        // Below `SHOW_TIME_REMAINING` (211px): no `0:00 / 0:00`. At 100px: still the
        // play button, a short timeline and the menu dots but no volume or time.
        let (narrow, text) = audio("width:100px");
        assert!(!text);
        assert!(narrow.iter().all(|p| !p.6), "no waves: {narrow:?}");
        assert!(narrow.iter().any(|p| p.4 == Some(Rgba::BLACK)));
        assert!(narrow.iter().any(|p| p.4 == Some(TIMELINE)));
        // Below `SHOW_MENU` (68px): only the pill, no menu dots.
        let (tiny, _) = audio("width:60px");
        assert!(tiny.iter().all(|p| p.4 != Some(Rgba::BLACK)), "{tiny:?}");
        assert!(tiny.iter().any(|p| p.4 == Some(PANEL)));
        assert!(tiny.iter().all(|p| p.4 != Some(TIMELINE)));
        // Under 54px tall (miaskiewicz/turbo-dom#346): the pill paints at the content
        // box's height, clipped.
        let (short, _) = audio("height:40px");
        assert!(
            short
                .iter()
                .any(|p| p.4 == Some(PANEL) && (p.3 - 40.0).abs() < 0.01),
            "{short:?}"
        );
        // Vertical writing mode: no panel at all (follow-up of miaskiewicz/turbo-dom#346).
        let (v, _) = audio("writing-mode:vertical-rl;width:54px;height:300px");
        assert!(v.iter().all(|p| p.4 != Some(PANEL)), "{v:?}");
    }

    #[test]
    fn without_a_face_the_time_display_is_left_out() {
        let mut frag = Fragment::new(
            super::NodeId(0),
            0.0,
            0.0,
            300.0,
            54.0,
            FragmentContent::Box {
                background: None,
                border: super::BorderEdges::default(),
                border_radius: super::BorderRadii::ZERO,
                shadow: None,
                gradient: None,
                transform: None,
            },
        );
        audio_panel(
            &mut frag,
            (0.0, 0.0, 300.0, 54.0),
            false,
            &crate::FontRegistry::default(),
        );
        assert!(frag
            .children
            .iter()
            .all(|c| !matches!(c.content, FragmentContent::TextLine { .. })));
        assert!(frag.children.iter().any(|c| matches!(
            c.content,
            FragmentContent::Box { background: Some(b), .. } if b == TIMELINE
        )));
    }
}
