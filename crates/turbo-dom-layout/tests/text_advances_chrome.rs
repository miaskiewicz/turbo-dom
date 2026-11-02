//! Text run advances against Chrome 156 (miaskiewicz/turbo-dom#191): shaped advances with
//! GPOS kerning, ligatures and mark positioning, each source run's advance admitted to the
//! 1/64 px `LayoutUnit` grid (at the device scale factor), Range rects over every prefix and a
//! middle slice of each text node, and Ranges across mixed-font spans.
//! Oracle: `tests/fixtures/text-advances/chrome156.json`, captured by
//! `scripts/fixture-font-chrome-controls.mjs` (Chrome 156.0.8078.47 headless, DSF 2, the
//! fixture fonts through `@font-face`). Laid out with the same fixture fonts. The block axis
//! is not compared here (line-height has its own control); every inline-axis edge is, exactly.

use serde_json::Value;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::OnceLock;
use turbo_dom_layout::layout_snapshot::{
    snapshot_html_live_tree_with_native_text, GeometryKind, LayoutSnapshot, NativeTextOptions,
    SnapshotInputs, SourceId, SourceIdentity,
};
use turbo_dom_layout::test_support::literal;
use turbo_dom_layout::test_support::literal::{parse_html, Node};
use turbo_dom_layout::{build_cascade, Diagnostics, FontRegistry, ImageCtx, NoImages};

fn identities(
    nodes: &[Node],
    path: &mut Vec<usize>,
    out: &mut Vec<SourceIdentity>,
    names: &mut HashMap<String, SourceId>,
) {
    for (index, node) in nodes.iter().enumerate() {
        path.push(index);
        let id = SourceId(out.len() as u64 + 1);
        out.push(SourceIdentity {
            id,
            path: path.clone(),
        });
        if let Node::Element(element) = node {
            if let Some(name) = element.attr("id") {
                names.insert(name.to_string(), id);
            }
            identities(&element.children, path, out, names);
        }
        path.pop();
    }
}

fn fonts() -> &'static FontRegistry {
    static FIXTURE: OnceLock<FontRegistry> = OnceLock::new();
    FIXTURE.get_or_init(turbo_dom_layout::test_support::fixture_fonts)
}

fn control() -> Value {
    let value: Value =
        serde_json::from_str(include_str!("fixtures/text-advances/chrome156.json")).unwrap();
    assert_eq!(value["browser"], "Chrome/156.0.8078.47");
    assert_eq!(value["errors"], serde_json::json!([]));
    assert_eq!(value["environment"]["deviceScaleFactor"], 2);
    value
}

fn capture(html: &str) -> (LayoutSnapshot, HashMap<String, SourceId>) {
    capture_original(html, None)
}

/// With `original`: the UTF-16 source units of span `s0`'s text node (the embedder's
/// original-text route).
fn capture_original(
    html: &str,
    original: Option<&[u16]>,
) -> (LayoutSnapshot, HashMap<String, SourceId>) {
    let nodes = parse_html(html).unwrap();
    let mut ids = Vec::new();
    let mut names = HashMap::new();
    identities(&nodes, &mut Vec::new(), &mut ids, &mut names);
    let originals: Vec<_> = original
        .map(|units| turbo_dom_layout::layout_snapshot::SourceTextUnits {
            source_id: SourceId(names["s0"].0 + 1),
            units,
        })
        .into_iter()
        .collect();
    let images = ImageCtx {
        resolver: &NoImages,
        body_height: None,
    };
    let cascade = build_cascade("", "", turbo_dom_layout::style::TokenSet::default());
    let snapshot = snapshot_html_live_tree_with_native_text(
        &literal::tree(&nodes).input(),
        &ids,
        &cascade,
        &NativeTextOptions {
            original_text: &originals,
            svg_viewports: &[],
            layout_scale: 2.0,
        },
        &SnapshotInputs {
            width: 800.0,
            height: 600.0,
            fonts: fonts(),
            images: &images,
        },
        &mut Diagnostics::default(),
    )
    .unwrap();
    (snapshot, names)
}

/// `[x, width]` of the source's border box.
fn span_box(snapshot: &LayoutSnapshot, id: SourceId) -> [f64; 2] {
    let r = snapshot
        .source(id)
        .unwrap()
        .fragment_indices
        .iter()
        .map(|index| &snapshot.fragments()[*index])
        .find(|fragment| fragment.kind == GeometryKind::BorderBox)
        .unwrap()
        .rect;
    [f64::from(r.x), f64::from(r.width)]
}

fn range_boxes(
    snapshot: &LayoutSnapshot,
    text: SourceId,
    start: usize,
    end: usize,
) -> Vec<[f64; 2]> {
    snapshot
        .text_range_boxes(text, start, end)
        .unwrap()
        .iter()
        .map(|b| {
            let r = b.transformed_bounds;
            [f64::from(r.x), f64::from(r.width)]
        })
        .collect()
}

fn html(case: &Value) -> String {
    let mut spans = String::new();
    for (i, part) in case["html"].as_array().unwrap().iter().enumerate() {
        write!(
            spans,
            "<span id=s{i} style=\"{}\">{}</span>",
            part[1].as_str().unwrap(),
            part[0].as_str().unwrap()
        )
        .unwrap();
    }
    format!(
        "<div style=\"width:700px;line-height:24px;{}\">{spans}</div>",
        case["style"].as_str().unwrap()
    )
}

/// Chrome's `[x, width]`, with x relative to the block (the control page's body margin is
/// 8px; the snapshot lays the block out at the origin).
fn inline_axis(rect: &Value) -> [f64; 2] {
    [rect[0].as_f64().unwrap() - 8.0, rect[2].as_f64().unwrap()]
}

fn compare_case(case: &Value, failures: &mut Vec<String>) {
    let (snapshot, names) = capture(&html(case));
    let name = format!("{} {}", case["style"], case["html"]);
    let mut check = |what: String, got: Vec<[f64; 2]>, want: Vec<[f64; 2]>| {
        if got != want {
            failures.push(format!("{name} {what}: got {got:?}, Chrome {want:?}"));
        }
    };
    for (i, span) in case["spans"].as_array().unwrap().iter().enumerate() {
        let id = names[&format!("s{i}")];
        check(
            format!("span {i}"),
            vec![span_box(&snapshot, id)],
            vec![inline_axis(&span["rect"])],
        );
        // The span's only child, its text node, takes the next identity.
        let text = SourceId(id.0 + 1);
        for range in span["ranges"].as_array().unwrap() {
            let (start, end) = (
                range["start"].as_u64().unwrap(),
                range["end"].as_u64().unwrap(),
            );
            check(
                format!("span {i} range {start}..{end}"),
                range_boxes(&snapshot, text, start as usize, end as usize),
                range["rects"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(inline_axis)
                    .collect(),
            );
        }
    }
}

#[test]
fn text_run_advances_and_range_rects_match_chrome_156() {
    let control = control();
    let cases = control["result"]["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 93);
    let mut failures = Vec::new();
    for case in cases {
        compare_case(case, &mut failures);
    }
    assert!(
        failures.is_empty(),
        "{} mismatches:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn mixed_font_ranges_span_their_runs_like_chrome_156() {
    let control = control();
    for case in control["result"]["cases"].as_array().unwrap() {
        let spans = case["spans"].as_array().unwrap();
        if spans.len() < 2 {
            continue;
        }
        let (snapshot, names) = capture(&html(case));
        let first = SourceId(names["s0"].0 + 1);
        let last_index = spans.len() - 1;
        let last = SourceId(names[&format!("s{last_index}")].0 + 1);
        let last_len = case["html"][last_index][0]
            .as_str()
            .unwrap()
            .encode_utf16()
            .count();
        let left = range_boxes(&snapshot, first, 0, 1)[0][0];
        let right = range_boxes(&snapshot, last, 0, last_len)
            .last()
            .map(|[x, w]| x + w)
            .unwrap();
        assert_eq!(
            [left, right - left],
            inline_axis(&case["range"]),
            "{}",
            case["html"]
        );
    }
}

#[test]
fn original_utf16_text_keeps_the_kerning_across_spaces() {
    let control = control();
    let mut compared = 0;
    for case in control["result"]["cases"].as_array().unwrap() {
        let text = case["html"][0][0].as_str().unwrap();
        if case["spans"].as_array().unwrap().len() != 1 || !text.contains(' ') {
            continue;
        }
        let units: Vec<u16> = text.encode_utf16().collect();
        let (snapshot, names) = capture_original(&html(case), Some(&units));
        assert_eq!(
            span_box(&snapshot, names["s0"]),
            inline_axis(&case["spans"][0]["rect"]),
            "{}",
            case["style"]
        );
        compared += 1;
    }
    assert_eq!(compared, 54);
}
