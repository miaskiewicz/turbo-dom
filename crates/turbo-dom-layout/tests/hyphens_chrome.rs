//! `hyphens: auto` dictionary hyphenation against Chrome 156 (miaskiewicz/turbo-dom#213):
//! English (`lang=en`, `en-US`, other `en-*` regions) breaks at the en-US Knuth-Liang
//! points under `hyphenate-limit-chars` (default 5 / 2 / 2), draws a hyphen when a break
//! is taken, leaves capitalized words, words with a soft hyphen, `hyphens: manual | none`
//! and unset or unsupported languages alone.
//! Oracle: `tests/fixtures/hyphens/chrome156.json`, captured by
//! `scripts/fixture-font-chrome-controls.mjs` (Chrome 156.0.8078.47 headless, the fixture
//! fonts through `@font-face` from 127.0.0.1). Laid out with the same fixture fonts.
//! Chrome on macOS hyphenates with the system's dictionary, whose points differ from the
//! en-US TeX patterns for some words (`international`, `extraordinary`); the control's
//! words are ones where both agree. Needs the default `hyphenation` feature.
#![cfg(feature = "hyphenation")]

use turbo_dom_layout::layout::fragment::{Fragment, FragmentContent};
use turbo_dom_layout::test_support::fixture_fonts;
use turbo_dom_layout::{layout_html, Diagnostics};

fn walk<'a>(f: &'a Fragment, out: &mut Vec<&'a Fragment>) {
    out.push(f);
    for c in &f.children {
        walk(c, out);
    }
}

struct Line {
    text: String,
    left: f32,
    right: f32,
}

const HYPHENS: [char; 2] = ['\u{2010}', '-'];

/// Each line box of the case, text runs on one line joined, and the box height.
fn lines(lang: &str, style: &str, text: &str) -> (Vec<Line>, f32) {
    let lang = if lang.is_empty() {
        String::new()
    } else {
        format!(" lang={lang}")
    };
    let html = format!(
        "<div{lang} style=\"font-family:Inter;font-size:16px;line-height:20px;{style}\">{text}</div>"
    );
    let mut diags = Diagnostics::default();
    let galley = layout_html(&html, "", 800.0, &fixture_fonts(), &mut diags).expect("layout");
    let mut frags = Vec::new();
    walk(&galley, &mut frags);
    let mut out: Vec<(f32, Line)> = Vec::new();
    let mut height: f32 = 0.0;
    for f in frags {
        if let FragmentContent::TextLine { glyphs, text, .. } = &f.content {
            height = height.max(f.y + f.height);
            let s: String = glyphs.iter().map(|g| g.text(text)).collect();
            match out.iter_mut().find(|line| (line.0 - f.y).abs() < 0.5) {
                Some((_, line)) => {
                    line.text.push_str(&s);
                    line.left = line.left.min(f.x);
                    line.right = line.right.max(f.x + f.width);
                }
                None => out.push((
                    f.y,
                    Line {
                        text: s,
                        left: f.x,
                        right: f.x + f.width,
                    },
                )),
            }
        }
    }
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    let top = out.first().map_or(0.0, |l| l.0);
    (out.into_iter().map(|l| l.1).collect(), height - top)
}

/// Line text without white space or hyphenation marks.
fn squash(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_whitespace() && *c != '\u{ad}' && !HYPHENS.contains(c))
        .collect()
}

fn check(ids: &[&str]) {
    let control: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/hyphens/chrome156.json")).unwrap();
    assert_eq!(control["browser"], "Chrome/156.0.8078.47");
    let cases = control["result"]["cases"].as_array().unwrap();
    let mut seen = 0;
    for case in cases
        .iter()
        .filter(|c| ids.contains(&c["id"].as_str().unwrap()))
    {
        seen += 1;
        let id = case["id"].as_str().unwrap();
        let style = case["style"].as_str().unwrap();
        let (got, height) = lines(
            case["lang"].as_str().unwrap(),
            style,
            case["text"].as_str().unwrap(),
        );
        let want = case["lines"].as_array().unwrap();
        let want_text: Vec<&str> = want.iter().map(|l| l["text"].as_str().unwrap()).collect();
        assert_eq!(
            got.iter().map(|l| squash(&l.text)).collect::<Vec<_>>(),
            want_text.iter().map(|t| squash(t)).collect::<Vec<_>>(),
            "{id}: line texts"
        );
        assert!(
            (height - case["height"].as_f64().unwrap() as f32).abs() < 0.5,
            "{id}: height {height}"
        );
        for (index, (line, chrome)) in got.iter().zip(&want_text).enumerate() {
            // A line Chrome ends inside a word (the next line goes on with a letter)
            // ends at a hyphenation point and draws a hyphen; `overflow-wrap` breaks
            // draw none, so that case checks widths only.
            let mid_word = chrome.chars().last().is_some_and(char::is_alphabetic)
                && want_text
                    .get(index + 1)
                    .and_then(|next| next.chars().next())
                    .is_some_and(char::is_alphabetic);
            let soft = chrome.ends_with('\u{ad}');
            let drawn = line.text.ends_with(HYPHENS);
            if !style.contains("overflow-wrap") {
                assert_eq!(drawn, mid_word || soft, "{id}: line {index} hyphen");
            }
            if chrome.contains(' ') {
                // The control's per-character rects put a collapsible space on the
                // next line, so only space-free lines compare widths.
                continue;
            }
            // Chrome's rect of the character before a break includes the hyphen drawn.
            let width = line.right - line.left;
            let chrome_width = want[index]["width"].as_f64().unwrap() as f32;
            assert!(
                (width - chrome_width).abs() <= 0.75,
                "{id}: line {index} width {width} vs Chrome {chrome_width}"
            );
        }
    }
    assert_eq!(seen, ids.len(), "every case is in the control");
}

#[test]
fn hyphens_auto_breaks_english_words_at_dictionary_points_like_chrome_156() {
    check(&[
        "en-words",
        "en-narrow",
        "en-us",
        "en-us-lower",
        "en-region",
        "en-punctuation",
        "en-span-lang",
        "en-span-auto",
        "en-span-edge",
        "en-break-word",
        "serif",
    ]);
}

#[test]
fn hyphens_auto_leaves_fitting_capitalized_and_soft_hyphenated_words_like_chrome_156() {
    check(&["en-fits", "en-capitalized", "auto-shy"]);
}

#[test]
fn hyphens_auto_needs_a_supported_language_and_hyphens_auto_like_chrome_156() {
    check(&[
        "no-lang",
        "unsupported-lang",
        "unsupported-nn",
        "unsupported-tr",
        "manual",
        "manual-default",
        "none",
    ]);
}

/// The other languages Chrome 156 on macOS hyphenates (miaskiewicz/turbo-dom#289), at
/// the TeX points of the default-on `hyphenation-languages` feature; outside English a
/// capitalized word hyphenates too.
#[cfg(feature = "hyphenation-languages")]
#[test]
fn hyphens_auto_breaks_other_languages_at_dictionary_points_like_chrome_156() {
    check(&[
        "de-words",
        "de-region",
        "de-caps",
        "fr-words",
        "fr-capitalized",
        "es-words",
        "nl-words",
        "pl-words",
        "en-gb",
        "en-gb-capitalized",
        "it-words",
        "ru-words",
        "lang-span-de",
    ]);
}

#[test]
fn hyphenate_limit_chars_bounds_word_prefix_and_suffix_like_chrome_156() {
    check(&["limit-word", "limit-prefix", "limit-suffix", "limit-short"]);
}
