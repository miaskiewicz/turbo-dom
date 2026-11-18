use super::*;
use crate::layout::boxgen::build_box_tree;
use crate::style::{build_cascade, StyledNode, TokenSet};
use crate::test_support::literal::{parse_html, style_nodes};

fn styled_table(body: &str) -> Vec<StyledNode> {
    let html = format!("<table>{body}</table>");
    let nodes = parse_html(&html).unwrap();
    let css = "*{font-family:Evolventa;font-size:12px}\
               table{border-collapse:collapse;width:600px}\
               td{padding:2px;border:1px solid black}";
    style_nodes(&nodes, &build_cascade(css, "", TokenSet::default()))
}

fn table(body: &str) -> LayoutBox {
    build_box_tree(&styled_table(body))
}

fn items(root: &LayoutBox) -> &[LayoutBox] {
    let BoxKind::Block(children) = &root.kind else {
        panic!("anonymous root must be a block");
    };
    let BoxKind::Table(items) = &children[0].kind else {
        panic!("first root child must be the table");
    };
    items
}

fn tracks(body: &str, paired: bool) -> (Vec<f32>, Vec<f32>) {
    let root = table(body);
    let table_items = items(&root);
    let rows = collect_rows(table_items);
    let (placed, ncols) = build_grid(&rows);
    let BoxKind::Block(children) = &root.kind else {
        unreachable!()
    };
    let seg = vsegments(&placed, (rows.len(), ncols), &children[0].style);
    let fonts = crate::test_support::registry();
    if paired {
        let values = paired_cell_intrinsics(&placed, &fonts);
        return (
            collapse_column_values(&placed, &seg, |i, _| values[i].0),
            collapse_column_values(&placed, &seg, |i, _| values[i].1),
        );
    }
    (
        collapse_columns(&placed, &seg, &fonts, natural_width),
        collapse_columns(&placed, &seg, &fonts, super::super::flex::min_content_width),
    )
}

#[test]
fn paired_tracks_preserve_explicit_percentage_spans_and_asymmetric_borders() {
    let cases = [
        "<tr><td>first alpha</td><td>second beta</td></tr>",
        "<tr><td style='width:40px'>first</td><td style='width:40%;border-right:5px solid'>second beta</td></tr>",
        "<tr><td colspan='2'>wide spanning phrase</td><td>tail</td></tr><tr><td>a</td><td>beta</td><td>gamma</td></tr>",
        "<tr><td rowspan='2' style='border-left:7px solid'>tall alpha</td><td>beta</td></tr><tr><td>gamma words</td></tr>",
        "<tr><td style='width:50px;box-sizing:border-box;padding:4px;border:3px solid'>box</td><td>tail words</td></tr>",
    ];
    for body in cases {
        assert_eq!(tracks(body, true), tracks(body, false), "{body}");
    }
}

#[test]
fn paired_tracks_count_collapsed_borders_once_for_specified_widths() {
    let body = "<tr><td style='width:40px'>first</td>\
                <td style='width:50px;box-sizing:border-box;padding:4px;border:3px solid'>second</td></tr>";
    assert_eq!(tracks(body, true), (vec![46.0, 50.0], vec![46.0, 50.0]));
}

#[cfg(feature = "perf")]
fn unique_rows() -> String {
    use std::fmt::Write;
    let mut html = String::new();
    for row in 0..200 {
        write!(
            html,
            "<tr><td>Label Word{row}</td><td>Entry Item{row}</td></tr>"
        )
        .unwrap();
    }
    html
}

#[cfg(feature = "perf")]
fn shape_work(body: &str, paired: bool) -> u64 {
    let root = table(body);
    let rows = collect_rows(items(&root));
    let (placed, _) = build_grid(&rows);
    let mut fonts = FontRegistry::new();
    fonts.add(crate::test_support::evolventa());
    crate::perf::reset();
    if paired {
        std::hint::black_box(paired_cell_intrinsics(&placed, &fonts));
    } else {
        for cell in &placed {
            std::hint::black_box(natural_width(cell.lb, &fonts));
        }
        for cell in &placed {
            std::hint::black_box(super::super::flex::min_content_width(cell.lb, &fonts));
        }
    }
    crate::perf::count("font.shape")
}

#[cfg(feature = "perf")]
#[test]
fn adjacent_cell_intrinsics_reuse_shapes_despite_unique_row_churn() {
    let body = unique_rows();
    let separated = shape_work(&body, false);
    let paired = shape_work(&body, true);
    eprintln!("cell intrinsic shapes: separated={separated}, paired={paired}");
    assert!(paired < separated, "separated={separated}, paired={paired}");
}

#[cfg(feature = "perf")]
#[test]
fn full_declared_width_table_layout_reuses_adjacent_intrinsic_shapes() {
    let styled = styled_table(&unique_rows());
    let mut fonts = FontRegistry::new();
    fonts.add(crate::test_support::evolventa());
    crate::perf::reset();
    let galley = crate::layout(&styled, 600.0, &fonts, &mut crate::Diagnostics::default());
    let mut words = Vec::new();
    collect_words(&galley, &mut words);
    let expected: Vec<_> = (0..200)
        .flat_map(|row| [format!("LabelWord{row}"), format!("EntryItem{row}")])
        .collect();
    assert_eq!(words, expected);
    let shapes = crate::perf::count("font.shape");
    eprintln!("full table layout shapes={shapes}");
    assert!(shapes <= 1000, "font.shape={shapes}");
}

#[cfg(feature = "perf")]
fn collect_words(fragment: &Fragment, out: &mut Vec<String>) {
    if let FragmentContent::TextLine { text, .. } = &fragment.content {
        out.extend(text.split_whitespace().map(str::to_owned));
    }
    for child in &fragment.children {
        collect_words(child, out);
    }
}

#[cfg(feature = "perf")]
fn min_natural_calls(mode: &str, prepare: bool) -> u64 {
    let mut root = table("<tr><td>first words</td><td>second words</td></tr>");
    let BoxKind::Block(children) = &mut root.kind else {
        unreachable!()
    };
    let table = &mut children[0];
    if mode == "fixed" {
        table.style.set("table-layout", "fixed".into());
    }
    if mode == "separate" {
        table.style.set("border-collapse", "separate".into());
    }
    let BoxKind::Table(items) = &table.kind else {
        unreachable!()
    };
    let fonts = crate::test_support::registry();
    crate::perf::reset();
    std::hint::black_box(layout_min_content_width(
        items,
        &table.style,
        &fonts,
        prepare,
    ));
    crate::perf::count("layout.natural_width")
}

#[cfg(feature = "perf")]
#[test]
fn min_only_and_fixed_or_separated_layouts_do_not_prepare_preferred_widths() {
    assert_eq!(min_natural_calls("auto", false), 0);
    assert_eq!(min_natural_calls("fixed", true), 0);
    assert_eq!(min_natural_calls("separate", true), 0);
}

#[test]
fn prepared_intrinsics_preserve_wrappers_and_text_policies() {
    for style in [
        "",
        "white-space:nowrap",
        "white-space:pre",
        "letter-spacing:-1.75px",
        "overflow-wrap:anywhere",
        "word-break:break-all",
        "hyphens:auto",
        "font-size:1.25em",
        "width:37.5%;font-size:1.25rem",
        "writing-mode:vertical-rl",
    ] {
        let body = format!(
            "<tr><td><div style='{style}'>office alpha אבג\t beta\n auto\u{ad}matic</div></td>\
             <td><span style='font-size:21.25px'>gamma 中文</span> tail</td></tr>"
        );
        assert_eq!(tracks(&body, true), tracks(&body, false), "{style}");
    }
}

#[test]
fn prepared_intrinsics_preserve_empty_cells_and_percentage_wrappers() {
    for body in [
        "<tr><td></td><td>tail</td></tr>",
        "<tr><td><div></div></td><td>tail</td></tr>",
        "<tr><td style='width:37.5%;font-size:1.25em'><div style='width:23%;font-size:1.25em'>alpha beta</div></td><td>tail</td></tr>",
    ] {
        assert_eq!(tracks(body, true), tracks(body, false), "{body}");
    }
}

#[test]
fn prepared_leaf_widths_match_independent_queries_at_each_native_scale() {
    let body = "<tr><td style='font-size:1.25em;padding:2.25px'>office alpha אבג</td></tr>";
    let fonts = crate::test_support::registry();
    for policy in [
        (false, None),
        (true, None),
        (true, Some(1.0)),
        (true, Some(1.25)),
        (true, Some(2.0)),
    ] {
        let root = table(body);
        root.set_html_layout_policy(policy);
        let rows = collect_rows(items(&root));
        let (placed, _) = build_grid(&rows);
        super::super::flex::prepare_text_intrinsics(placed[0].lb, &fonts);
        let actual = (
            natural_width(placed[0].lb, &fonts),
            super::super::flex::min_content_width(placed[0].lb, &fonts),
        );
        let fresh = table(body);
        fresh.set_html_layout_policy(policy);
        let rows = collect_rows(items(&fresh));
        let (placed, _) = build_grid(&rows);
        let expected = (
            natural_width(placed[0].lb, &fonts),
            super::super::flex::min_content_width(placed[0].lb, &fonts),
        );
        assert_eq!(actual.0.to_bits(), expected.0.to_bits(), "max {policy:?}");
        assert_eq!(actual.1.to_bits(), expected.1.to_bits(), "min {policy:?}");
    }
}

#[cfg(feature = "perf")]
fn text_leaf(lb: &LayoutBox) -> &LayoutBox {
    match &lb.kind {
        BoxKind::Block(children) if children.len() == 1 => text_leaf(&children[0]),
        BoxKind::Lines(_) => lb,
        _ => panic!("expected a single text leaf"),
    }
}

#[cfg(feature = "perf")]
#[test]
fn prepared_intrinsics_reuse_once_and_respect_partial_cache_and_policy_changes() {
    let root = table("<tr><td>first alpha beta</td></tr>");
    let rows = collect_rows(items(&root));
    let (placed, _) = build_grid(&rows);
    let fonts = crate::test_support::registry();
    let leaf = text_leaf(placed[0].lb);
    crate::perf::reset();
    let first = paired_cell_intrinsics(&placed, &fonts);
    assert_eq!(crate::perf::count("inline.intrinsic_pair_prepare"), 1);
    assert_eq!(paired_cell_intrinsics(&placed, &fonts), first);
    assert_eq!(crate::perf::count("inline.intrinsic_pair_prepare"), 1);
    leaf.set_html_layout_policy((true, Some(1.25)));
    super::super::flex::prepare_text_intrinsics(leaf, &fonts);
    assert_eq!(crate::perf::count("inline.intrinsic_pair_prepare"), 2);
    let expected = table("<tr><td>first alpha beta</td></tr>");
    let rows = collect_rows(items(&expected));
    let (expected, _) = build_grid(&rows);
    let expected = text_leaf(expected[0].lb);
    expected.set_html_layout_policy((true, Some(1.25)));
    assert_eq!(
        natural_width(leaf, &fonts).to_bits(),
        natural_width(expected, &fonts).to_bits()
    );
    assert_eq!(
        super::super::flex::min_content_width(leaf, &fonts).to_bits(),
        super::super::flex::min_content_width(expected, &fonts).to_bits()
    );

    let fresh = table("<tr><td>first alpha beta</td></tr>");
    let rows = collect_rows(items(&fresh));
    let (placed, _) = build_grid(&rows);
    let leaf = text_leaf(placed[0].lb);
    natural_width(leaf, &fonts);
    super::super::flex::prepare_text_intrinsics(leaf, &fonts);
    assert_eq!(crate::perf::count("inline.intrinsic_pair_prepare"), 2);
}
