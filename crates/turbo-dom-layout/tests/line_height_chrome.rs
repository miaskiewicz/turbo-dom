//! A `line-height` smaller than the font's content area against Chrome 156
//! (miaskiewicz/turbo-dom#178): the used line-height is honoured (negative leading), so
//! line boxes are exactly that tall and the glyphs of adjacent lines overlap.
//! Oracle: `tests/fixtures/line-height/chrome156.json`, captured by
//! `scripts/fixture-font-chrome-controls.mjs` (Chrome 156.0.8078.47 headless, the fixture
//! fonts through `@font-face`). Laid out with the same fixture fonts.

use turbo_dom_layout::layout::fragment::{Fragment, FragmentContent};
use turbo_dom_layout::test_support::fixture_fonts;
use turbo_dom_layout::{layout_html, Diagnostics};

/// The 60px-wide test block (fragment coordinates are absolute).
fn block(f: &Fragment) -> Option<&Fragment> {
    if (f.width - 60.0).abs() < 0.01 && !matches!(f.content, FragmentContent::TextLine { .. }) {
        return Some(f);
    }
    f.children.iter().find_map(block)
}

fn text_line_tops(f: &Fragment, out: &mut Vec<f32>) {
    if matches!(f.content, FragmentContent::TextLine { .. })
        && !out.iter().any(|y| (y - f.y).abs() < 0.01)
    {
        out.push(f.y);
    }
    for c in &f.children {
        text_line_tops(c, out);
    }
}

#[test]
fn line_height_below_the_content_area_is_honoured_like_chrome_156() {
    let control: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/line-height/chrome156.json")).unwrap();
    assert_eq!(control["browser"], "Chrome/156.0.8078.47");
    let cases = control["result"].as_array().unwrap();
    assert_eq!(cases.len(), 6);
    for case in cases {
        let id = case["id"].as_str().unwrap();
        let style = case["style"].as_str().unwrap();
        let html = format!(
            "<!doctype html><div style=\"width:60px;{style}\"><span>aa</span> <span>bb</span> <span>cc</span></div>"
        );
        let mut diags = Diagnostics::default();
        let galley = layout_html(&html, "", 400.0, &fixture_fonts(), &mut diags).expect("layout");
        let div = block(&galley).expect("the test block");
        let want_h = case["height"].as_f64().unwrap() as f32;
        assert!(
            (div.height - want_h).abs() <= 0.5,
            "{id}: block height {} vs Chrome {want_h}",
            div.height
        );
        let mut tops = Vec::new();
        text_line_tops(div, &mut tops);
        let want = case["lineTops"].as_array().unwrap();
        assert_eq!(tops.len(), want.len(), "{id}: line count ({tops:?})");
        // The line pitch is the used line-height (Chrome's span tops step by it).
        for (i, pair) in want.windows(2).enumerate() {
            let pitch = (pair[1].as_f64().unwrap() - pair[0].as_f64().unwrap()) as f32;
            let got = tops[i + 1] - tops[i];
            assert!(
                (got - pitch).abs() <= 0.5,
                "{id}: line {i} pitch {got} vs Chrome {pitch}"
            );
        }
    }
}
