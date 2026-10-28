//! Multi-run inline layout (§5.2, AC-5.4): wrapping across runs, baseline
//! alignment of mixed sizes, vertical-align, alignment, per-glyph fallback, and
//! the `.notdef` lint.

mod common;

use turbo_dom_layout::layout::fragment::NodeId;
use turbo_dom_layout::layout::inline::*;
use turbo_dom_layout::layout::value::{Rgba, VAlign};
use turbo_dom_layout::text::{Align, FontFace, FontRegistry};
use turbo_dom_layout::{Diagnostics, LintCode};

fn run(text: &str, face: &FontFace) -> InlineRun {
    InlineRun {
        node_id: NodeId(0),
        text: text.to_string(),
        face: face.clone(),
        families: vec![face.family().to_string()],
        weight: face.weight(),
        italic: face.is_italic(),
        font_size: 16.0,
        line_height: None,
        letter_spacing: 0.0,
        color: Rgba::BLACK,
        valign: VAlign::Baseline,
        nowrap: false,
        preserve_spaces: false,
        forced_newlines: false,
    }
}

fn lay(runs: &[InlineRun], max_width: f32, align: Align) -> (ParagraphLayout, Diagnostics) {
    let reg = common::registry();
    let mut diags = Diagnostics::default();
    let p = layout_runs(runs, &reg, max_width, align, &mut diags);
    (p, diags)
}

fn first_x(line: &InlineLine) -> f32 {
    line.runs[0].glyphs[0].x
}

#[test]
fn single_run_one_line() {
    let (p, diags) = lay(
        &[run("hello world", &common::evolventa())],
        1000.0,
        Align::Left,
    );
    assert_eq!(p.lines.len(), 1);
    assert!(p.width > 0.0);
    assert_eq!(p.height, p.lines[0].height);
    assert!(diags.is_empty());
}

#[test]
fn empty_and_whitespace_produce_no_lines() {
    assert!(lay(&[run("", &common::evolventa())], 500.0, Align::Left)
        .0
        .lines
        .is_empty());
    let ws = lay(&[run("   ", &common::evolventa())], 500.0, Align::Left).0;
    assert!(ws.lines.is_empty());
    assert_eq!(ws.height, 0.0);
}

#[test]
fn wraps_to_multiple_lines_and_stacks() {
    let (p, _) = lay(
        &[run("aaaa bbbb cccc dddd eeee", &common::evolventa())],
        80.0,
        Align::Left,
    );
    assert!(p.lines.len() >= 2);
    // finalize stacks lines: second line top equals first line height.
    assert_eq!(p.lines[1].top, p.lines[0].height);
    assert_eq!(p.height, p.lines.iter().map(|l| l.height).sum::<f32>());
}

#[test]
fn long_word_takes_its_own_line() {
    let (p, _) = lay(
        &[run("supercalifragilistic short", &common::evolventa())],
        40.0,
        Align::Left,
    );
    assert!(p.lines.len() >= 2);
}

#[test]
fn leading_and_trailing_whitespace_collapse() {
    let (p, _) = lay(&[run("  a b  ", &common::evolventa())], 1000.0, Align::Left);
    assert_eq!(p.lines.len(), 1);
}

#[test]
fn multi_run_word_keeps_distinct_faces() {
    // "bo" + "ld" with no space -> one word, two segments, two glyph runs
    // (different weight, so not merged).
    let runs = [
        run("bo", &common::evolventa()),
        run("ld", &common::evolventa_bold()),
    ];
    let (p, _) = lay(&runs, 1000.0, Align::Left);
    assert_eq!(p.lines.len(), 1);
    assert_eq!(p.lines[0].runs.len(), 2);
}

#[test]
fn adjacent_identical_runs_merge() {
    let runs = [
        run("ab", &common::evolventa()),
        run("cd", &common::evolventa()),
    ];
    let (p, _) = lay(&runs, 1000.0, Align::Left);
    assert_eq!(p.lines[0].runs.len(), 1); // merged into one run
    assert_eq!(p.lines[0].runs[0].glyphs.len(), 4);
}

#[test]
fn mixed_font_sizes_grow_line_box() {
    let small = lay(&[run("a", &common::evolventa())], 1000.0, Align::Left).0;
    let mut big_run = run("A", &common::evolventa());
    big_run.font_size = 40.0;
    let mixed = lay(
        &[run("a", &common::evolventa()), big_run],
        1000.0,
        Align::Left,
    )
    .0;
    assert!(mixed.lines[0].height > small.lines[0].height);
}

#[test]
fn superscript_raises_subscript_lowers() {
    let mut sup = run("y", &common::evolventa());
    sup.valign = VAlign::Super;
    let p = lay(&[run("x", &common::evolventa()), sup], 1000.0, Align::Left).0;
    let glyphs = &p.lines[0].runs[0].glyphs; // merged: baseline glyph then raised glyph
    assert!(glyphs[1].y < glyphs[0].y);

    let mut sub = run("y", &common::evolventa());
    sub.valign = VAlign::Sub;
    let q = lay(&[run("x", &common::evolventa()), sub], 1000.0, Align::Left).0;
    let g = &q.lines[0].runs[0].glyphs;
    assert!(g[1].y > g[0].y);
}

#[test]
fn alignment_shifts_line() {
    let text = "hi";
    let left = lay(&[run(text, &common::evolventa())], 500.0, Align::Left).0;
    let right = lay(&[run(text, &common::evolventa())], 500.0, Align::Right).0;
    let center = lay(&[run(text, &common::evolventa())], 500.0, Align::Center).0;
    let justify = lay(&[run(text, &common::evolventa())], 500.0, Align::Justify).0;
    assert!(first_x(&left.lines[0]) < first_x(&center.lines[0]));
    assert!(first_x(&center.lines[0]) < first_x(&right.lines[0]));
    assert_eq!(first_x(&justify.lines[0]), first_x(&left.lines[0])); // justify lays out left in v1
}

#[test]
fn letter_spacing_widens() {
    let plain = lay(&[run("mmmm", &common::evolventa())], 1000.0, Align::Left).0;
    let mut spaced_run = run("mmmm", &common::evolventa());
    spaced_run.letter_spacing = 5.0;
    let spaced = lay(&[spaced_run], 1000.0, Align::Left).0;
    assert!(spaced.lines[0].width > plain.lines[0].width);
}

#[test]
fn explicit_line_height_sets_box() {
    let mut r = run("a", &common::evolventa());
    r.line_height = Some(60.0);
    let p = lay(&[r], 1000.0, Align::Left).0;
    assert_eq!(p.lines[0].height, 60.0);
}

#[test]
fn missing_glyph_emits_notdef_lint() {
    let (_p, diags) = lay(&[run("中", &common::evolventa())], 500.0, Align::Left);
    assert!(diags.lints.iter().any(|l| l.code == LintCode::NotdefGlyph));
}

#[test]
fn per_glyph_fallback_finds_another_face() {
    let go = common::go();
    let ev = common::evolventa();
    let evb = common::evolventa_bold();
    let probe = (0x80u32..0x10000)
        .filter_map(char::from_u32)
        .find(|&c| !go.has_glyph(c) && (ev.has_glyph(c) || evb.has_glyph(c)));
    let c = probe.expect("expected a char Go lacks but Evolventa has");

    let mut r = run(&c.to_string(), &go);
    r.families = vec!["Go".to_string()];
    let reg = {
        let mut reg = FontRegistry::new();
        reg.add(go.clone());
        reg.add(ev);
        reg
    };
    let mut diags = Diagnostics::default();
    let p = layout_runs(&[r], &reg, 1000.0, Align::Left, &mut diags);
    assert!(diags.is_empty()); // fallback found, no notdef
    assert_eq!(p.lines[0].runs[0].face.family(), "Evolventa");
}

#[test]
fn nowrap_run_stays_on_one_line() {
    // `white-space:nowrap` folds inter-word spaces into the word so the text never
    // wraps, even when the column is far too narrow (menu tabs, buttons).
    let mut r = run("View source history", &common::evolventa());
    r.nowrap = true;
    // A width of 10px would wrap every word without nowrap.
    let (layout, _) = lay(&[r], 10.0, Align::Left);
    assert_eq!(layout.lines.len(), 1, "nowrap text must not wrap");
    let wrapping = run("View source history", &common::evolventa());
    let (wl, _) = lay(&[wrapping], 10.0, Align::Left);
    assert!(
        wl.lines.len() > 1,
        "control: wrapping text does wrap at 10px"
    );
}

fn atom(id: usize, valign: VAlign, height: f32) -> InlineAtom {
    InlineAtom {
        id,
        width: 20.0,
        height,
        valign,
        font_size: 16.0,
        margin_top: 0.0,
        margin_bottom: 0.0,
        margin_left: 0.0,
        margin_right: 0.0,
        placeholder: false,
        baseline: None,
    }
}

#[test]
fn vertical_align_places_inline_block_atoms() {
    // A 60px baseline atom sets the line box height; three 40px atoms then pin
    // differently: `top` to the line top (y=0), `bottom` to the line bottom
    // (60 - 40 = 20), and `middle` centred ((60 - 40)/2 = 10).
    let reg = common::registry();
    let mut diags = Diagnostics::default();
    let pieces = vec![
        Piece::Atom(atom(0, VAlign::Baseline, 60.0)),
        Piece::Atom(atom(1, VAlign::Top, 40.0)),
        Piece::Atom(atom(2, VAlign::Bottom, 40.0)),
        Piece::Atom(atom(3, VAlign::Middle, 40.0)),
    ];
    let p = layout_paragraph(&pieces, &reg, 500.0, Align::Left, &mut diags);
    let line = &p.lines[0];
    assert!(
        (line.height - 60.0).abs() < 0.01,
        "line height, got {}",
        line.height
    );
    let y = |id: usize| line.atoms.iter().find(|a| a.id == id).unwrap().y;
    assert!(
        (y(1) - 0.0).abs() < 0.01,
        "top pins to line top, got {}",
        y(1)
    );
    assert!(
        (y(2) - 20.0).abs() < 0.01,
        "bottom pins to line bottom, got {}",
        y(2)
    );
    assert!((y(3) - 10.0).abs() < 0.01, "middle centres, got {}", y(3));
}

/// `data-cid` boxes of `html` through the HTML drive (bundled fonts, 400x300).
#[cfg(feature = "bundled-fonts")]
fn cid_boxes(html: &str) -> std::collections::HashMap<String, (f32, f32, f32, f32)> {
    let tree = turbo_dom_layout::parse_html_tree(html);
    turbo_dom_layout::layout_boxes_tree(
        &tree,
        "",
        400.0,
        300.0,
        &FontRegistry::new(),
        &turbo_dom_layout::HtmlOptions::default(),
        &mut Diagnostics::default(),
    )
    .into_iter()
    .map(|b| (b.cid, (b.x, b.y, b.width, b.height)))
    .collect()
}

#[cfg(feature = "bundled-fonts")]
#[test]
fn whitespace_only_inline_runs_collapse_unless_white_space_preserves_them() {
    // CSS 2.1 §9.2.2.1: only white space that `white-space` collapses away generates no
    // anonymous inline box. Chrome 156 (one line = 18px there, `a`'s height here):
    //   <p>a</p><span style="white-space:W"> </span><p>b</p><div><span ...> </span></div>
    //   normal/nowrap: b.y 18, div height 0; pre/pre-wrap/break-spaces: b.y 36, div 18.
    //   A pre-line newline between blocks is a line (div 54 = 3 lines), pre-line spaces
    //   collapse (36); a flex container's white-space-only text is never rendered (0).
    let doc = |ws: &str| {
        cid_boxes(&format!(
            "<body style='margin:0'><p data-cid=a style='margin:0'>a</p><span style='white-space:{ws}'> </span>\
             <p data-cid=b style='margin:0'>b</p><div data-cid=d><span style='white-space:{ws}'> </span></div></body>"
        ))
    };
    let normal = doc("normal");
    let line = normal["a"].3;
    assert!(line > 0.0);
    assert_eq!(normal["b"].1, line, "the collapsed span takes no height");
    assert_eq!(normal["d"].3, 0.0, "a whitespace-only block is empty");
    for ws in ["pre", "pre-wrap", "break-spaces"] {
        let kept = doc(ws);
        assert_eq!(
            kept["b"].1,
            2.0 * line,
            "{ws}: the preserved space takes a line"
        );
        assert_eq!(kept["d"].3, line, "{ws}");
    }
    let nowrap = doc("nowrap");
    assert_eq!(
        nowrap["b"].1, line,
        "nowrap: the collapsed span takes no height"
    );
    assert_eq!(
        nowrap["d"].3, 0.0,
        "nowrap: a whitespace-only block is empty"
    );
    let other = cid_boxes(
        "<body style='margin:0'><div data-cid=f style='display:flex;white-space:pre'> </div>\
         <div data-cid=h style='white-space:pre-line'><p style='margin:0'>a</p>  <p style='margin:0'>b</p></div>\
         <div data-cid=n style='white-space:pre-line'><p style='margin:0'>a</p>\n<p style='margin:0'>b</p></div></body>",
    );
    assert_eq!(
        other["f"].3, 0.0,
        "flex: white-space-only text is not rendered"
    );
    assert_eq!(other["h"].3, 2.0 * line, "pre-line collapses spaces");
    assert_eq!(
        other["n"].3,
        3.0 * line,
        "a pre-line newline between blocks is a line"
    );
}

#[cfg(feature = "bundled-fonts")]
#[test]
fn margins_collapse_through_collapsed_white_space_but_not_preserved_space() {
    // Chrome 156 (18px lines there, `line` here), the same document: a collapsible
    // white-space-only run between two blocks is no line box, so their 20px margins
    // collapse through it (b.y 38 = 18 + 20); a preserved space is a line that keeps
    // them apart (c at 76 = 18 + 20 + 18 + 20 in its div). Text directly in a flex
    // container is an anonymous flex item (one line). An absolutely positioned box
    // between inline runs next to a block stays in their line (z: that line + p).
    let boxes = cid_boxes(
        "<body style='margin:0'><div data-cid=x><p data-cid=a style='margin:0 0 20px'>a</p><span> </span>\
         <p data-cid=b style='margin:20px 0 0'>b</p></div><div data-cid=y><p style='margin:0 0 20px'>a</p>\
         <span style='white-space:pre'> </span><p data-cid=c style='margin:20px 0 0'>b</p></div>\
         <div data-cid=t style='display:flex'>hello</div><div data-cid=z><span>a</span>\
         <i style='position:absolute'>q</i><span>b</span><p style='margin:0'>p</p></div></body>",
    );
    let line = boxes["a"].3;
    assert_eq!(boxes["b"].1, line + 20.0);
    assert_eq!(boxes["x"].3, 2.0 * line + 20.0);
    assert_eq!(boxes["c"].1 - boxes["y"].1, 2.0 * line + 40.0);
    // (The flex item's line rounds to a whole pixel here: 19 vs 19.36.)
    assert!(
        (boxes["t"].3 - line).abs() < 1.0,
        "flex text is an item: {:?}",
        boxes["t"]
    );
    assert_eq!(boxes["z"].3, 2.0 * line);
}

#[cfg(feature = "bundled-fonts")]
#[test]
fn preserved_newlines_and_spaces_inside_edged_inline_boxes() {
    // miaskiewicz/turbo-dom#166 through the inline-edge word builder (a padded span):
    // a preserved newline breaks the line, pre-wrap spaces stay glyphs and the line may
    // break after them.
    let boxes = cid_boxes(
        "<body style='margin:0;line-height:20px'>\
         <div data-cid=n style='white-space:pre'><span style='padding:0 2px'>a\nb</span></div>\
         <div data-cid=w style='white-space:pre-wrap'><span style='padding:0 2px'>ab    cd</span></div>\
         <div data-cid=c style='white-space:normal'><span style='padding:0 2px'>ab    cd</span></div>\
         <div data-cid=x style='white-space:pre-wrap;width:1px'><span style='padding:0 2px'>ab cd</span></div></body>",
    );
    assert_eq!(boxes["n"].3, 40.0, "the newline is a forced break");
    assert_eq!(boxes["w"].3, 20.0);
    assert_eq!(boxes["x"].3, 40.0, "pre-wrap breaks after its spaces");
    assert_eq!(boxes["c"].3, 20.0);
}
