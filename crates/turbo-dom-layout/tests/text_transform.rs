//! turbo-dom#208: `text-transform` changes the rendered (and extracted) text.
//! Oracle: Chrome 156.0.8078.47 headless, `innerText` of each probe element (which
//! applies `text-transform`), and widths with the Inter fixture loaded through
//! `@font-face` from localhost. Inter-word spaces are pen advances, not glyphs,
//! so the expected strings below are compared without white space.

use turbo_dom_layout::layout::fragment::{Fragment, FragmentContent};
use turbo_dom_layout::layout::inline::{layout_paragraph_in_with_sources, InlineRun, Piece};
use turbo_dom_layout::layout::value::{Rgba, VAlign};
use turbo_dom_layout::test_support::fixture_fonts;
use turbo_dom_layout::text::{Align, CaseLocale, CaseTransform, TextTransform};
use turbo_dom_layout::{layout_html, Diagnostics, NodeId};

fn walk<'a>(f: &'a Fragment, out: &mut Vec<&'a Fragment>) {
    out.push(f);
    for c in &f.children {
        walk(c, out);
    }
}

/// `(glyph text, width)` of every text line, in document order.
fn lines(html: &str) -> Vec<(String, f32)> {
    let mut diags = Diagnostics::default();
    let galley = layout_html(html, "", 1280.0, &fixture_fonts(), &mut diags).expect("layout");
    let mut frags = Vec::new();
    walk(&galley, &mut frags);
    frags
        .iter()
        .filter_map(|f| match &f.content {
            FragmentContent::TextLine { glyphs, text, .. } => Some((
                glyphs.iter().map(|g| g.text(text)).collect::<String>(),
                f.width,
            )),
            _ => None,
        })
        .collect()
}

/// The element's glyph text, all lines joined.
fn text(html: &str) -> String {
    lines(html).into_iter().map(|line| line.0).collect()
}

fn squash(chrome: &str) -> String {
    chrome.chars().filter(|c| !c.is_whitespace()).collect()
}

fn case(style: &str, lang: Option<&str>, source: &str, chrome: &str) {
    let lang = lang.map_or_else(String::new, |lang| format!(" lang={lang}"));
    let html = format!("<p style=\"font-family:Inter;text-transform:{style}\"{lang}>{source}</p>");
    assert_eq!(text(&html), squash(chrome), "{html}");
}

#[test]
fn uppercase_uses_full_case_mapping() {
    case(
        "uppercase",
        None,
        "don’t stop ß ﬁ ŉ ǰ istanbul ı",
        "DON’T STOP SS FI ʼN J̌ ISTANBUL I",
    );
    case("uppercase", Some("de"), "straße", "STRASSE");
    case(
        "uppercase",
        None,
        "άλφα ήταν ΐ",
        "ΆΛΦΑ ΉΤΑΝ \u{399}\u{308}\u{301}",
    );
}

#[test]
fn lowercase_uses_full_case_mapping_and_final_sigma() {
    case(
        "lowercase",
        None,
        "ISTANBUL İ I ΣΑΣ ΟΔΟΣ.",
        "istanbul i\u{307} i σας οδος.",
    );
    case(
        "lowercase",
        None,
        "ΑΣ ΑΣ1 ΑΣ\u{301} Σ ΑΣΑ",
        "ας ας1 ας\u{301} σ ασα",
    );
}

#[test]
fn turkic_languages_map_dotted_and_dotless_i() {
    case("uppercase", Some("tr"), "istanbul ı i", "İSTANBUL I İ");
    case("lowercase", Some("tr"), "ISTANBUL İ I", "ıstanbul i ı");
    case("uppercase", Some("az"), "i ı", "İ I");
    case("lowercase", Some("az"), "I İ", "ı i");
    case("uppercase", Some("tr-TR"), "i", "İ");
    case("uppercase", Some("TR"), "i", "İ");
    // Chrome's capitalize ignores the language.
    case("capitalize", Some("tr"), "istanbul", "Istanbul");
}

#[test]
fn lithuanian_keeps_the_dot_above_explicit() {
    case("uppercase", Some("lt"), "i\u{307}\u{300}", "I\u{300}");
    case("lowercase", Some("lt"), "Ì I J Į", "i\u{307}\u{300} i j į");
}

#[test]
fn greek_uppercase_drops_accents() {
    case("uppercase", Some("el"), "άλφα ήταν", "ΑΛΦΑ ΗΤΑΝ");
    case("uppercase", Some("el"), "ΐ ύ Ά", "Ϊ Υ Α");
    // Chrome 156 (ICU Greek upper): ι/υ after an accented vowel is not a diphthong
    // and gets a dialytika (combining when the accent was combining), which ends the
    // accent context; a lone ή
    // (the disjunctive "or") keeps its tonos.
    case("uppercase", Some("el"), "άι άυ αί αι άϊ", "ΑΪ ΑΫ ΑΙ ΑΙ ΑΪ");
    case("uppercase", Some("el"), "ήίύ", "ΗΪΥ");
    case("uppercase", Some("el"), "α\u{301}ι", "ΑΙ\u{308}");
    case("uppercase", Some("el"), "ή", "\u{389}");
    case("uppercase", Some("el"), "a ή b", "A\u{389}B");
    case("uppercase", Some("el"), "ΐ ά ι", "Ϊ Α Ι");
    case("capitalize", Some("el"), "άλφα", "Άλφα");
}

#[test]
fn capitalize_titlecases_the_first_letter_of_each_word() {
    case(
        "capitalize",
        None,
        "don’t stop hello-world x1y 1st a.b a_b (paren) «quote» ‘tis o'neil ǆemal ßa ﬁx",
        "Don’t Stop Hello-World X1y 1st A.B A_b (Paren) «Quote» ‘Tis O'neil ǅemal ßa ﬁx",
    );
    case(
        "capitalize",
        None,
        "a:b a/b a@b a&amp;b a·b a—b 'ab \"ab 3d a1b éa e\u{301}a a\u{301}b a’b a‘b a\"b a,b a;b a!b a?b a+b a=b a|b a~b a#b a%b a*b",
        "A:B A/B A@B A&B A·b A—B 'Ab \"Ab 3d A1b Éa E\u{301}a A\u{301}b A’b A‘b A\"B A,B A;B A!B A?B A+B A=B A|B A~B A#B A%B A*B",
    );
    case(
        "capitalize",
        None,
        "a\u{a0}b a\u{2009}b a\u{200b}b a\u{3000}b 日本ab ab日本 aＡb a׳b a״b a٫b a：b",
        "A\u{a0}B A\u{2009}B A\u{200b}B A\u{3000}B 日本Ab Ab日本 AＡb A׳b A״b A٫b A：B",
    );
}

#[test]
fn capitalize_sees_the_preceding_text_of_the_inline_context() {
    let t = |body: &str| text(&format!("<div style=\"font-family:Inter\">{body}</div>"));
    let cap = "style=\"text-transform:capitalize\"";
    assert_eq!(
        t(&format!(
            "<span>x</span><span {cap}>ab</span> <i {cap}>cd</i>"
        )),
        "xabCd"
    );
    assert_eq!(t(&format!("x<b {cap}> ab</b>")), "xAb");
    assert_eq!(
        t(&format!("x<img style=\"width:5px\"><b {cap}>ab</b>")),
        "xAb"
    );
    assert_eq!(t(&format!("x<br><b {cap}>ab</b>")), "xAb");
    assert_eq!(
        t("x<span style=\"display:inline-block;text-transform:capitalize\">ab</span>"),
        "xAb"
    );
    assert_eq!(
        t(&format!(
            "<p {cap}>ab<span>cd</span> ef <span>gh</span></p>"
        )),
        "AbcdEfGh"
    );
    assert_eq!(
        text("<pre style=\"font-family:Inter;text-transform:capitalize\">a  b\nc</pre>"),
        "A  BC" // preserved spaces are glyphs; Chrome: `A  B\nC`
    );
}

#[test]
fn none_full_width_and_invalid_values_render_as_authored() {
    // Chrome 156 does not support `full-width`; the two-keyword value is invalid.
    case("full-width", None, "Ab 1! ~", "Ab 1! ~");
    case("uppercase full-width", None, "ab", "ab");
    assert_eq!(
        text(
            "<p style=\"font-family:Inter;text-transform:none\"><span style=\"text-transform:uppercase\">ab</span>cd</p>"
        ),
        "ABcd"
    );
    assert_eq!(
        text("<p style=\"font-family:Inter;text-transform:uppercase\"><span style=\"text-transform:none\">ab</span>cd</p>"),
        "abCD"
    );
}

#[test]
fn transformed_text_is_measured_like_chrome() {
    // Chrome 156 widths with the Inter fixture at 16px.
    for (style, source, chrome) in [
        ("uppercase", "don’t stop", 97.328_125),
        ("uppercase", "straße", 72.0),
        ("capitalize", "hello world", 87.5625),
    ] {
        let html = format!("<div style=\"font:16px Inter;text-transform:{style}\">{source}</div>");
        let found = lines(&html);
        assert_eq!(found.len(), 1, "{html}: {found:?}");
        assert!((found[0].1 - chrome).abs() < 0.05, "{html}: {found:?}");
    }
}

fn transformed_run(text: &str, case: CaseTransform, locale: CaseLocale) -> Piece {
    let fonts = fixture_fonts();
    Piece::Run(InlineRun {
        node_id: NodeId(5),
        text: text.into(),
        face: fonts.select(&["Inter"], 400, false).unwrap().clone(),
        families: vec!["Inter".into()],
        weight: 400,
        italic: false,
        font_size: 16.0,
        line_height: Some(20.0),
        letter_spacing: 0.0,
        color: Rgba {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        },
        valign: VAlign::Baseline,
        nowrap: false,
        preserve_spaces: false,
        forced_newlines: false,
        text_transform: TextTransform { case, locale },
    })
}

type GlyphSource = (String, (usize, usize), (usize, usize));

/// Per glyph: (glyph text, source byte range, source UTF-16 range).
fn sources(text: &str, case: CaseTransform, locale: CaseLocale) -> Vec<GlyphSource> {
    let fonts = fixture_fonts();
    let native = layout_paragraph_in_with_sources(
        &[transformed_run(text, case, locale)],
        &fonts,
        Align::Left,
        &mut Diagnostics::default(),
        &|_| (0.0, 1000.0),
        20.0,
    );
    let mut out = Vec::new();
    for (line, clusters) in native.layout.lines.iter().zip(&native.clusters) {
        for (run, cells) in line.runs.iter().zip(clusters) {
            assert_eq!(run.glyphs.len(), cells.len());
            for (glyph, cell) in run.glyphs.iter().zip(cells) {
                assert!(text.is_char_boundary(cell.byte_start), "{cell:?}");
                assert!(text.is_char_boundary(cell.byte_end), "{cell:?}");
                out.push((
                    glyph.text(&run.text).to_owned(),
                    (cell.byte_start, cell.byte_end),
                    (cell.utf16_start, cell.utf16_end),
                ));
            }
        }
    }
    out
}

#[test]
fn length_changing_mappings_keep_the_source_cluster_offsets() {
    // `ß` (2 bytes, 1 UTF-16 unit at byte 4) becomes two glyphs `S` `S`; both map
    // back to the one source character, and `E` keeps its own source offsets.
    let found = sources("straße", CaseTransform::Uppercase, CaseLocale::Default);
    let s = |t: &str, b: (usize, usize), u: (usize, usize)| (t.to_owned(), b, u);
    assert_eq!(
        found,
        vec![
            s("S", (0, 1), (0, 1)),
            s("T", (1, 2), (1, 2)),
            s("R", (2, 3), (2, 3)),
            s("A", (3, 4), (3, 4)),
            s("S", (4, 6), (4, 5)),
            s("S", (4, 6), (4, 5)),
            s("E", (6, 7), (5, 6)),
        ]
    );
    // `ı` (2 bytes) uppercases to the 1-byte `I`; later offsets are unshifted.
    let found = sources("ıa", CaseTransform::Uppercase, CaseLocale::Default);
    assert_eq!(found, vec![s("I", (0, 2), (0, 1)), s("A", (2, 3), (1, 2))]);
    // Turkish lowercase `I\u{307}` → `i`: the dot is consumed into the `i` cluster.
    let found = sources("I\u{307}x", CaseTransform::Lowercase, CaseLocale::Turkic);
    assert_eq!(found[0].0, "i");
    assert_eq!(found[0].1 .0, 0);
    assert_eq!(found.last().unwrap(), &s("x", (3, 4), (2, 3)));
}
