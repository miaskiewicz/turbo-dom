//! Inline `position: sticky` boxes against Chrome 156 (miaskiewicz/turbo-dom#218): an
//! inline sticky box shifts only past its threshold, its margin box stays inside its
//! block container's content box, and its content moves with it.
//! Oracle: `tests/fixtures/inline-sticky/chrome156.json`, captured by
//! `scripts/fixture-font-chrome-controls.mjs` (Chrome 156.0.8078.47 headless, 400x300,
//! the Liberation Serif fixture through `@font-face` from 127.0.0.1). Laid out on the
//! native snapshot path (the one that keeps inline boxes) with the same fixture fonts.

use std::collections::HashMap;
use std::sync::OnceLock;
use turbo_dom_layout::layout_snapshot::{
    snapshot_html_live_tree_with_native_text, GeometryKind, LayoutSnapshot, NativeTextOptions,
    SnapshotInputs, SourceId, SourceIdentity,
};
use turbo_dom_layout::test_support::literal;
use turbo_dom_layout::test_support::literal::{parse_html, Node};
use turbo_dom_layout::{build_cascade, Diagnostics, FontRegistry, ImageCtx, NoImages};

const PROBE: &str = include_str!("fixtures/inline-sticky/probe.js");

fn fonts() -> &'static FontRegistry {
    static FIXTURE: OnceLock<FontRegistry> = OnceLock::new();
    FIXTURE.get_or_init(turbo_dom_layout::test_support::fixture_fonts)
}

/// Pre-order source identities, and `data-cid` -> id.
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
            if let Some(cid) = element.attr("data-cid") {
                names.insert(cid.to_string(), id);
            }
            identities(&element.children, path, out, names);
        }
        path.pop();
    }
}

/// The probe's markup: the string literals assigned to `innerHTML`, joined.
fn probe_body() -> String {
    let start = PROBE.find("innerHTML =").unwrap();
    let end = start + PROBE[start..].find(";\n").unwrap();
    PROBE[start..end].split('"').skip(1).step_by(2).collect()
}

fn capture() -> (LayoutSnapshot, HashMap<String, SourceId>) {
    let nodes = parse_html(&format!("<div id=root>{}</div>", probe_body())).unwrap();
    let mut ids = Vec::new();
    let mut names = HashMap::new();
    identities(&nodes, &mut Vec::new(), &mut ids, &mut names);
    let images = ImageCtx {
        resolver: &NoImages,
        body_height: None,
    };
    let cascade = build_cascade(
        "#root{font-family:\"Liberation Serif\";line-height:20px}",
        "",
        turbo_dom_layout::style::TokenSet::default(),
    );
    let snapshot = snapshot_html_live_tree_with_native_text(
        &literal::tree(&nodes).input(),
        &ids,
        &cascade,
        &NativeTextOptions {
            original_text: &[],
            svg_viewports: &[],
            layout_scale: 2.0,
        },
        &SnapshotInputs {
            width: 400.0,
            height: 300.0,
            fonts: fonts(),
            images: &images,
        },
        &mut Diagnostics::default(),
    )
    .unwrap();
    (snapshot, names)
}

/// The union of a source's border-box rects (its `getBoundingClientRect()`), as
/// `[x, y, width, height]`.
fn rect(snapshot: &LayoutSnapshot, id: SourceId) -> [f32; 4] {
    let source = snapshot.source(id).unwrap();
    let (x0, y0, x1, y1) = source
        .fragment_indices
        .iter()
        .map(|index| &snapshot.fragments()[*index])
        .filter(|fragment| fragment.kind == GeometryKind::BorderBox)
        .map(|fragment| fragment.rect)
        .fold(
            (f32::MAX, f32::MAX, f32::MIN, f32::MIN),
            |(x0, y0, x1, y1), r| {
                (
                    x0.min(r.x),
                    y0.min(r.y),
                    x1.max(r.x + r.width),
                    y1.max(r.y + r.height),
                )
            },
        );
    [x0, y0, x1 - x0, y1 - y0]
}

#[test]
#[ignore = "turbo-dom#218: fails until the fix"]
fn inline_sticky_boxes_stick_within_their_containing_block() {
    // `top:10px` in a one-line block moves 1px (to its line's bottom), in a 100px
    // block `top:30px` reaches 30; `left:100px` reaches 100, `left:300px` stops at
    // 192, a nested <b> moves with its sticky parent, and `right` / no inset leave
    // the box in place.
    let control: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/inline-sticky/chrome156.json")).unwrap();
    assert_eq!(control["browser"], "Chrome/156.0.8078.47");
    assert_eq!(control["errors"], serde_json::json!([]));
    let (snapshot, names) = capture();
    let expected = control["result"].as_object().unwrap();
    assert_eq!(expected.len(), 8);
    for (cid, want) in expected {
        let want: Vec<f32> = want
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap() as f32)
            .collect();
        let got = rect(&snapshot, names[cid]);
        assert!(
            got.iter().zip(&want).all(|(g, w)| (g - w).abs() <= 0.02),
            "{cid}: {got:?} vs Chrome {want:?}"
        );
    }
}
