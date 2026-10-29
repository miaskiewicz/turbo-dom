//! turbo-dom#205: every shaped glyph carries its source cluster text, so a PDF
//! `/ToUnicode` map (or `/ActualText`) reproduces the source characters instead
//! of reverse-mapping the font's cmap (which picks the lowest codepoint for a
//! shared glyph and nothing for a ligature). Oracle: Chrome 156 `page.pdf()`
//! extracts `don’t` and `a -> b != c => d` verbatim from these fonts.

use turbo_dom_layout::layout::fragment::{Fragment, FragmentContent};
use turbo_dom_layout::test_support::fixture_fonts;
use turbo_dom_layout::{layout_html, Diagnostics};

fn walk<'a>(f: &'a Fragment, out: &mut Vec<&'a Fragment>) {
    out.push(f);
    for c in &f.children {
        walk(c, out);
    }
}

/// Every text line's glyph texts, in glyph order, one entry per glyph.
fn glyph_texts(html: &str) -> Vec<Vec<String>> {
    let mut diags = Diagnostics::default();
    let galley = layout_html(html, "", 1280.0, &fixture_fonts(), &mut diags).expect("layout");
    let mut frags = Vec::new();
    walk(&galley, &mut frags);
    frags
        .iter()
        .filter_map(|f| match &f.content {
            FragmentContent::TextLine { glyphs, text, .. } => Some(
                glyphs
                    .iter()
                    .map(|g| g.text(text).to_owned())
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
        .collect()
}

fn joined(html: &str) -> String {
    glyph_texts(html).concat().concat()
}

#[test]
fn shared_glyph_keeps_its_source_character_not_the_lowest_codepoint() {
    // Inter draws U+2019 and U+02BC (and U+201D / U+02EE) with one glyph each.
    let texts =
        glyph_texts(r#"<p style="font-family:Inter">“quotes” ‘single’ don’t</p>"#).concat();
    assert!(texts.iter().any(|t| t == "’"), "{texts:?}");
    assert!(texts.iter().any(|t| t == "”"), "{texts:?}");
    assert!(!texts.iter().any(|t| t == "ʼ" || t == "ˮ"), "{texts:?}");
    assert_eq!(
        texts.concat(),
        "“quotes” ‘single’ don’t",
        "the line's glyph texts reproduce the source"
    );
}

#[test]
fn ligature_glyphs_carry_their_whole_cluster_once() {
    let html = r#"<p style="font-family:'Fira Code'">a -> b != c => d</p>"#;
    assert_eq!(joined(html), "a -> b != c => d");
}

#[test]
fn merged_runs_and_wrapped_lines_keep_per_line_cluster_text() {
    // Several words of one text node merge into one run per line; a narrow
    // box wraps them, so each line's text is the visible words of that line.
    let html = r#"<div style="width:60px;font-family:Inter">alpha beta gamma delta</div>"#;
    let lines: Vec<String> = glyph_texts(html).into_iter().map(|l| l.concat()).collect();
    assert!(lines.len() >= 2, "{lines:?}");
    assert_eq!(
        lines.concat().split_whitespace().collect::<Vec<_>>(),
        ["alpha", "beta", "gamma", "delta"]
    );
    for line in &lines {
        assert!(!line.trim().is_empty(), "{lines:?}");
    }
}

#[test]
fn text_transform_text_is_what_the_glyphs_show() {
    // Chrome extracts the transformed text.
    let html = r#"<p style="font-family:Inter;text-transform:uppercase">don’t</p>"#;
    assert_eq!(joined(html), "DON’T");
}
