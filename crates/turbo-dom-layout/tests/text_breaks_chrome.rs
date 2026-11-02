//! Line-break opportunities inside words against Chrome 156: `overflow-wrap` /
//! `word-break` (miaskiewicz/turbo-dom#212) and soft hyphens (#213).
//! Oracle: `tests/fixtures/text-breaks/chrome156.json`, captured by
//! `scripts/fixture-font-chrome-controls.mjs` (Chrome 156.0.8078.47 headless, the Inter
//! fixture through `@font-face` from 127.0.0.1). Laid out with the same fixture fonts.

use turbo_dom_layout::layout::fragment::{Fragment, FragmentContent};
use turbo_dom_layout::test_support::fixture_fonts;
use turbo_dom_layout::{layout_html, Diagnostics};

fn walk<'a>(f: &'a Fragment, out: &mut Vec<&'a Fragment>) {
    out.push(f);
    for c in &f.children {
        walk(c, out);
    }
}

/// `(text, width)` of each line box, text runs on one line joined.
fn lines(style: &str, text: &str) -> Vec<(String, f32)> {
    let html = format!(
        "<div lang=en style=\"font-family:Inter;font-size:16px;line-height:20px;{style}\">{text}</div>"
    );
    let mut diags = Diagnostics::default();
    let galley = layout_html(&html, "", 800.0, &fixture_fonts(), &mut diags).expect("layout");
    let mut frags = Vec::new();
    walk(&galley, &mut frags);
    let mut out: Vec<(f32, String, f32, f32)> = Vec::new();
    for f in frags {
        if let FragmentContent::TextLine { glyphs, text, .. } = &f.content {
            let s: String = glyphs.iter().map(|g| g.text(text)).collect();
            match out.iter_mut().find(|line| (line.0 - f.y).abs() < 0.5) {
                Some(line) => {
                    line.1.push_str(&s);
                    line.2 = line.2.min(f.x);
                    line.3 = line.3.max(f.x + f.width);
                }
                None => out.push((f.y, s, f.x, f.x + f.width)),
            }
        }
    }
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    out.into_iter().map(|l| (l.1, l.3 - l.2)).collect()
}

/// Line text without white space or hyphenation marks (U+00AD, and the hyphen a taken
/// soft hyphen draws).
fn squash(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_whitespace() && !matches!(c, '\u{ad}' | '\u{2010}' | '-'))
        .collect()
}

fn check(ids: &[&str]) {
    let control: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/text-breaks/chrome156.json")).unwrap();
    assert_eq!(control["browser"], "Chrome/156.0.8078.47");
    let cases = control["result"]["breaks"].as_array().unwrap();
    let mut seen = 0;
    for case in cases
        .iter()
        .filter(|c| ids.contains(&c["id"].as_str().unwrap()))
    {
        seen += 1;
        let id = case["id"].as_str().unwrap();
        let got = lines(
            case["style"].as_str().unwrap(),
            case["text"].as_str().unwrap(),
        );
        let want = case["lines"].as_array().unwrap();

        assert_eq!(
            got.iter().map(|l| squash(&l.0)).collect::<Vec<_>>(),
            want.iter()
                .map(|l| squash(l["text"].as_str().unwrap()))
                .collect::<Vec<_>>(),
            "{id}: line texts"
        );
        for (index, (line, chrome)) in got.iter().zip(want).enumerate() {
            // The control's per-character rects put a collapsible space on the next
            // line, so only space-free lines (a drawn hyphen included) compare widths.
            let chrome_text = chrome["text"].as_str().unwrap();
            if chrome_text.contains(' ') {
                continue;
            }
            let chrome_width = chrome["width"].as_f64().unwrap() as f32;
            assert!(
                (line.1 - chrome_width).abs() <= 0.75,
                "{id}: line {index} width {} vs Chrome {chrome_width}",
                line.1
            );
        }
    }
    assert_eq!(seen, ids.len(), "every case is in the control");
}

#[test]
fn overflow_wrap_and_word_break_break_inside_words_like_chrome_156() {
    check(&[
        "break-word",
        "anywhere",
        "break-all",
        "break-word-words",
        "break-all-words",
        "normal-overflows",
        "keep-all",
    ]);
}

#[test]
fn soft_hyphens_are_break_opportunities_that_draw_a_hyphen_like_chrome_156() {
    check(&["shy", "shy-fits", "shy-two", "shy-none", "shy-words"]);
}

/// miaskiewicz/turbo-dom#229: the same break opportunities inside and across inline
/// span edges (markup cases): an `overflow-wrap` break plus a soft hyphen inside a span,
/// nested spans, a soft hyphen at either side of a span edge, `word-break: break-all`
/// across span edges, and `hyphens: none` on a span.
#[test]
fn break_opportunities_carry_across_inline_span_edges_like_chrome_156() {
    check(&[
        "span-wrap-shy",
        "span-wrap-shy-mid",
        "span-nested",
        "span-shy-edge-before",
        "span-shy-edge-after",
        "span-break-all-edge",
        "span-hyphens-none",
    ]);
}
