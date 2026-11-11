//! miaskiewicz/turbo-dom#238: UAX #9 bidi inline layout against Chrome 156.
//!
//! Reads `tests/fixtures/bidi/chrome156.json` (`scripts/fixture-font-chrome-controls.mjs`
//! running `tests/fixtures/bidi/probe.js`, the fixture fonts loaded through `@font-face`,
//! device scale 2) and lays every case out through the native HTML text snapshot at layout
//! scale 2 with [`fixture_fonts`](turbo_dom_layout::test_support::fixture_fonts): each
//! `[id]` element's border-box fragments, each code unit's Range rect and the caret
//! offsets along the first line must match Chrome's.
use std::collections::HashMap;
use std::sync::OnceLock;

use serde_json::Value;
use turbo_dom_layout::layout_snapshot::{
    snapshot_html_live_tree_with_native_text, GeometryKind, LayoutSnapshot, NativeTextOptions,
    SnapshotInputs, SourceId, SourceIdentity,
};
use turbo_dom_layout::test_support::literal::{self, parse_html, Node};
use turbo_dom_layout::{build_cascade, Diagnostics, FontRegistry, ImageCtx, NoImages};

const BOX: &str =
    "font-family:'Fira Code','Noto Sans Hebrew','Noto Sans Arabic';font-size:16px;line-height:24px";
/// Chrome positions in `LayoutUnit`s; text advances differ in the last bits.
const TOLERANCE: f64 = 0.02;

fn fixture() -> &'static Value {
    static FIXTURE: OnceLock<Value> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/bidi/chrome156.json"
        );
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    })
}

fn fonts() -> &'static FontRegistry {
    static FONTS: OnceLock<FontRegistry> = OnceLock::new();
    FONTS.get_or_init(turbo_dom_layout::test_support::fixture_fonts)
}

/// Every node's identity (paths into the parsed forest), the `[id]` elements by name and
/// the text nodes by their child-index path from `#box`.
#[derive(Default)]
struct Ids {
    all: Vec<SourceIdentity>,
    named: HashMap<String, SourceId>,
    texts: HashMap<Vec<usize>, SourceId>,
}

fn collect(nodes: &[Node], path: &mut Vec<usize>, ids: &mut Ids) {
    for (index, node) in nodes.iter().enumerate() {
        path.push(index);
        let id = SourceId(ids.all.len() as u64 + 1);
        ids.all.push(SourceIdentity {
            id,
            path: path.clone(),
        });
        match node {
            Node::Element(element) => {
                if let Some(name) = element.attr("id") {
                    ids.named.insert(name.to_string(), id);
                }
                collect(&element.children, path, ids);
            }
            Node::Text(_) => {
                // Paths from `#box` (the forest's first node).
                ids.texts.insert(path[1..].to_vec(), id);
            }
        }
        path.pop();
    }
}

fn capture(case: &Value) -> (LayoutSnapshot, Ids) {
    let html = format!(
        "<div id=\"box\" dir=\"{}\" style=\"{BOX};width:{}px\">{}</div>",
        case["dir"].as_str().unwrap(),
        case["width"].as_u64().unwrap(),
        case["html"].as_str().unwrap()
    );
    let nodes = parse_html(&html).unwrap();
    let mut ids = Ids::default();
    collect(&nodes, &mut Vec::new(), &mut ids);
    let images = ImageCtx {
        resolver: &NoImages,
        body_height: None,
    };
    let cascade = build_cascade("", "", turbo_dom_layout::style::TokenSet::default());
    let snapshot = snapshot_html_live_tree_with_native_text(
        &literal::tree(&nodes).input(),
        &ids.all,
        &cascade,
        &NativeTextOptions {
            original_text: &[],
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
    .unwrap_or_else(|error| panic!("{}: {error:?}", case["id"]));
    (snapshot, ids)
}

fn rect(value: &Value) -> [f64; 4] {
    std::array::from_fn(|i| value[i].as_f64().unwrap())
}

fn close(a: [f64; 4], b: [f64; 4]) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() <= TOLERANCE)
}

fn border_boxes(snapshot: &LayoutSnapshot, id: SourceId) -> Vec<[f64; 4]> {
    snapshot
        .source(id)
        .map(|source| {
            source
                .fragment_indices
                .iter()
                .map(|&index| &snapshot.fragments()[index])
                .filter(|fragment| fragment.kind == GeometryKind::BorderBox)
                .map(|fragment| {
                    let r = fragment.rect;
                    [r.x, r.y, r.width, r.height].map(f64::from)
                })
                .collect()
        })
        .unwrap_or_default()
}

fn compare_elements(
    case: &Value,
    id: &str,
    snapshot: &turbo_dom_layout::layout_snapshot::LayoutSnapshot,
    ids: &Ids,
) -> Vec<String> {
    let mut failures = Vec::new();
    for element in case["elements"].as_array().unwrap() {
        let name = element["id"].as_str().unwrap();
        let want: Vec<_> = element["rects"]
            .as_array()
            .unwrap()
            .iter()
            .map(rect)
            .collect();
        let got = border_boxes(snapshot, ids.named[name]);
        if got.len() != want.len() || !got.iter().zip(&want).all(|(g, w)| close(*g, *w)) {
            failures.push(format!("{id}: #{name} rects {got:?}, Chrome {want:?}"));
        }
    }
    failures
}

fn compare_texts(
    case: &Value,
    id: &str,
    snapshot: &turbo_dom_layout::layout_snapshot::LayoutSnapshot,
    ids: &Ids,
) -> Vec<String> {
    let mut failures = Vec::new();
    for text in case["texts"].as_array().unwrap() {
        let path: Vec<usize> = text["path"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as usize)
            .collect();
        let source = ids.texts[&path];
        for (unit, want) in text["chars"].as_array().unwrap().iter().enumerate() {
            let want: Vec<_> = want.as_array().unwrap().iter().map(rect).collect();
            let got: Vec<_> = snapshot
                .text_range_boxes(source, unit, unit + 1)
                .map(|boxes| {
                    boxes
                        .iter()
                        .map(|b| [b.rect.x, b.rect.y, b.rect.width, b.rect.height].map(f64::from))
                        .collect()
                })
                .unwrap_or_default();
            if got.len() != want.len() || !got.iter().zip(&want).all(|(g, w)| close(*g, *w)) {
                failures.push(format!(
                    "{id}: text {path:?}[{unit}] {got:?}, Chrome {want:?}"
                ));
            }
        }
    }
    failures
}

fn compare_carets(
    case: &Value,
    id: &str,
    snapshot: &turbo_dom_layout::layout_snapshot::LayoutSnapshot,
    ids: &Ids,
) -> Vec<String> {
    let mut failures = Vec::new();
    for caret in case["carets"].as_array().unwrap() {
        let Some(path) = caret[2].as_array() else {
            continue;
        };
        let path: Vec<usize> = path.iter().map(|v| v.as_u64().unwrap() as usize).collect();
        let point = turbo_dom_layout::layout_snapshot::Point {
            x: caret[0].as_f64().unwrap() as f32,
            y: caret[1].as_f64().unwrap() as f32,
        };
        let got = snapshot.text_caret_offset_from_point(ids.texts[&path], point);
        let want = caret[3].as_u64().unwrap() as usize;
        if got.as_ref().ok() != Some(&want) {
            failures.push(format!("{id}: caret at {point:?} {got:?}, Chrome {want}"));
        }
    }
    failures
}

/// Compare one case with Chrome; the differences, one line each.
fn compare(case: &Value) -> Vec<String> {
    let id = case["id"].as_str().unwrap();
    let (snapshot, ids) = capture(case);
    let mut failures = compare_elements(case, id, &snapshot, &ids);
    failures.extend(compare_texts(case, id, &snapshot, &ids));
    failures.extend(compare_carets(case, id, &snapshot, &ids));
    failures
}

fn compare_cases(select: impl Fn(&str) -> bool) {
    let mut failures = Vec::new();
    let mut compared = 0;
    for case in fixture()["result"].as_array().unwrap() {
        if select(case["id"].as_str().unwrap()) {
            compared += 1;
            failures.extend(compare(case));
        }
    }
    assert!(compared > 0);
    assert!(
        failures.is_empty(),
        "{} differences from Chrome 156:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn mixed_direction_paragraphs_reorder_like_chrome_156() {
    compare_cases(|id| {
        id.starts_with("ltr-")
            || id.starts_with("rtl-") && !id.contains("wrap")
            || id == "mixed-in-word"
            || id == "arabic"
            || id == "arabic-in-ltr"
            || id == "arabic-indic-digits"
    });
}

#[test]
fn inline_unicode_bidi_embeds_isolates_and_overrides_like_chrome_156() {
    compare_cases(|id| id.starts_with("span-") && !id.starts_with("span-edges-"));
}

#[test]
fn inline_box_edges_sit_on_their_visual_sides_like_chrome_156() {
    compare_cases(|id| id.starts_with("span-edges-"));
}

#[test]
fn lines_break_in_logical_order_then_reorder_like_chrome_156() {
    compare_cases(|id| id.contains("-wrap"));
}

#[test]
fn plaintext_paragraphs_take_their_first_strong_direction_like_chrome_156() {
    compare_cases(|id| id.starts_with("block-plaintext-"));
}

/// The visual order of `case`'s characters on its first line from Chrome's per-code-unit
/// Range rects: `(x, char)` sorted left to right, whitespace left out.
fn chrome_visual_order(case: &Value) -> Vec<(f64, char)> {
    let mut chars = Vec::new();
    let mut first_y = None;
    for text in case["texts"].as_array().unwrap() {
        let units: Vec<u16> = text["text"].as_str().unwrap().encode_utf16().collect();
        for (unit, rects) in units.iter().zip(text["chars"].as_array().unwrap()) {
            let ch = char::from_u32(u32::from(*unit)).unwrap();
            let Some(r) = rects.as_array().unwrap().first() else {
                continue;
            };
            let r = rect(r);
            if ch.is_whitespace() || *first_y.get_or_insert(r[1]) != r[1] {
                continue;
            }
            chars.push((r[0], ch));
        }
    }
    chars.sort_by(|a, b| a.0.total_cmp(&b.0));
    chars
}

/// The static HTML drive's (`layout_html`) characters on the first line: every glyph's
/// cluster text at its absolute pen x, sorted left to right, whitespace left out.
fn static_visual_order(case: &Value) -> Vec<(f64, char)> {
    use turbo_dom_layout::layout::fragment::{Fragment, FragmentContent};
    fn walk(f: &Fragment, x: f32, y: f32, top: &mut Option<f32>, out: &mut Vec<(f64, char)>) {
        let (x, y) = (x + f.x, y + f.y);
        if let FragmentContent::TextLine {
            glyphs,
            text,
            face,
            font_size,
            ..
        } = &f.content
        {
            if *top.get_or_insert(y) == y {
                // A glyph paints at its pen position plus its GPOS x offset (Noto Sans
                // Hebrew's right-to-left pair kerning moves the glyph's origin); Chrome's
                // Range rect is the pen position, so take the offset back out. The run's
                // shaping is the one direction whose glyphs are the drawn ones.
                let ids: Vec<u16> = glyphs.iter().map(|g| g.glyph_id).collect();
                let shaped = [false, true]
                    .into_iter()
                    .map(|rtl| face.shape_in(text, rtl))
                    .find(|shaped| shaped.iter().map(|g| g.glyph_id).eq(ids.iter().copied()))
                    .expect("the drawn glyphs are the run shaped in one direction");
                let scale = face.scale(*font_size);
                for (glyph, shaped) in glyphs.iter().zip(&shaped) {
                    let cluster = &text[glyph.text_start as usize..glyph.text_end as usize];
                    let pen = x + glyph.x - scale * shaped.x_offset as f32;
                    for ch in cluster.chars().filter(|c| !c.is_whitespace()) {
                        out.push((f64::from(pen), ch));
                    }
                }
            }
        }
        for child in &f.children {
            walk(child, x, y, top, out);
        }
    }
    let html = format!(
        "<html><body style=\"margin:0\"><div dir=\"{}\" style=\"{BOX};width:{}px\">{}</div></body></html>",
        case["dir"].as_str().unwrap(),
        case["width"].as_u64().unwrap(),
        case["html"].as_str().unwrap()
    );
    let galley =
        turbo_dom_layout::layout_html(&html, "", 800.0, fonts(), &mut Diagnostics::default())
            .unwrap();
    let mut out = Vec::new();
    walk(&galley, 0.0, 0.0, &mut None, &mut out);
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    out
}

/// The static HTML drive (`layout_html`, the PDF path: inline `unicode-bidi` as
/// formatting characters around the flattened span, never drawn) reorders every inline
/// embed, isolate and override like Chrome 156: the same characters in the same visual
/// order at Chrome's x (the static drive positions in floats, not `LayoutUnit`s).
#[test]
fn static_drive_reorders_inline_unicode_bidi_like_chrome_156() {
    let mut compared = 0;
    for case in fixture()["result"].as_array().unwrap() {
        let id = case["id"].as_str().unwrap();
        if !id.starts_with("span-") || id.starts_with("span-edges-") {
            continue;
        }
        compared += 1;
        let chrome = chrome_visual_order(case);
        let got = static_visual_order(case);
        // Range rects name the source characters; the drawn glyphs are text-transformed.
        let order = |chars: &[(f64, char)]| {
            chars
                .iter()
                .flat_map(|c| c.1.to_lowercase())
                .collect::<String>()
        };
        assert_eq!(order(&got), order(&chrome), "{id}");
        for (g, c) in got.iter().zip(&chrome) {
            assert!(
                (g.0 - c.0).abs() <= 0.05,
                "{id}: {} at {}, Chrome {}",
                g.1,
                g.0,
                c.0
            );
        }
    }
    assert!(compared >= 8);
}
