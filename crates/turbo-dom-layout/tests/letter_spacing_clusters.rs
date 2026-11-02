//! `letter-spacing` is added once per typographic character unit, after its
//! last glyph, not after every glyph. Chrome 156 control (Inter fixture via
//! `@font-face`, 16px, `letter-spacing:10px`): `qqq` and `q\u{301}q\u{301}q\u{301}`
//! both measure 59.390625px; without spacing the marked text is 29.390625px.
//! Per-glyph spacing made the marked text 30px too wide and pushed each
//! combining mark 10px off its base.

use turbo_dom_layout::layout::fragment::{Fragment, FragmentContent};
use turbo_dom_layout::test_support::fixture_fonts;
use turbo_dom_layout::{layout_html, Diagnostics};

const MARKED: &str = "q\u{301}q\u{301}q\u{301}";

fn inter() -> turbo_dom_layout::text::FontFace {
    fixture_fonts()
        .select(&["Inter"], 400, false)
        .expect("Inter fixture")
        .clone()
}

#[test]
fn measure_adds_spacing_once_per_cluster() {
    let face = inter();
    let plain = face.measure("qqq", 16.0, 10.0);
    let marked = face.measure(MARKED, 16.0, 10.0);
    assert!((plain - 59.39).abs() < 0.05, "{plain}");
    assert!((marked - plain).abs() < 0.01, "{marked} vs {plain}");
}

fn walk<'a>(f: &'a Fragment, out: &mut Vec<&'a Fragment>) {
    out.push(f);
    for c in &f.children {
        walk(c, out);
    }
}

/// `(glyph text, x)` of every glyph of the one text line.
fn glyphs(html: &str) -> Vec<(String, f32)> {
    let mut diags = Diagnostics::default();
    let galley = layout_html(html, "", 1280.0, &fixture_fonts(), &mut diags).expect("layout");
    let mut frags = Vec::new();
    walk(&galley, &mut frags);
    frags
        .iter()
        .find_map(|f| match &f.content {
            FragmentContent::TextLine { glyphs, text, .. } => Some(
                glyphs
                    .iter()
                    .map(|g| (g.text(text).to_owned(), f.x + g.x))
                    .collect(),
            ),
            _ => None,
        })
        .expect("a text line")
}

#[test]
fn laid_out_bases_advance_like_unmarked_text_and_marks_stay_on_their_base() {
    let style = "font:16px Inter;letter-spacing:10px";
    let plain = glyphs(&format!("<p style=\"{style}\">qqq</p>"));
    let marked = glyphs(&format!("<p style=\"{style}\">{MARKED}</p>"));
    let bases: Vec<f32> = marked
        .iter()
        .filter(|(text, _)| !text.is_empty())
        .map(|(_, x)| *x)
        .collect();
    let plain_x: Vec<f32> = plain.iter().map(|(_, x)| *x).collect();
    assert_eq!(bases.len(), 3, "{marked:?}");
    for (base, plain) in bases.iter().zip(&plain_x) {
        assert!((base - plain).abs() < 0.01, "{marked:?} vs {plain_x:?}");
    }
    // Each mark sits within its base's 16px glyph box, not 10px past it.
    for pair in marked.chunks(2) {
        let (base, mark) = (pair[0].1, pair[1].1);
        assert!(mark - base < 10.0, "{marked:?}");
    }
}
