use serde_json::Value;
use sha2::{Digest, Sha256};
use turbo_dom_layout::layout_snapshot::{
    snapshot_html_live_tree_with_native_text, NativeTextOptions, SnapshotInputs, SourceId,
    SourceIdentity,
};
use turbo_dom_layout::test_support::literal::{self, Node};
use turbo_dom_layout::{build_cascade, ImageCtx, NoImages};
use turbo_dom_layout::{layout_boxes, Diagnostics, FontRegistry};

#[test]
fn absolute_percentage_checkbox_fills_positioned_flex_padding_box() {
    let html = r#"<body style="margin:0"><span data-cid="parent" style="position:relative;display:inline-flex;padding:9px">
        <input data-cid="input" type="checkbox" style="position:absolute;top:0;left:0;width:100%;height:100%;padding:0;margin:0;border:0;opacity:0">
        <div style="width:24px;height:24px"></div></span></body>"#;
    let boxes = layout_boxes(
        html,
        "",
        800.0,
        600.0,
        &FontRegistry::new(),
        &mut Diagnostics::default(),
    )
    .unwrap();
    let parent = boxes.iter().find(|b| b.cid == "parent").unwrap();
    let input = boxes.iter().find(|b| b.cid == "input").unwrap();
    assert_eq!([parent.width, parent.height], [42.0, 42.0]);
    assert_eq!(
        [
            input.x - parent.x,
            input.y - parent.y,
            input.width,
            input.height
        ],
        [0.0, 0.0, 42.0, 42.0]
    );
}

fn control() -> Value {
    let control: Value =
        serde_json::from_str(include_str!("fixtures/absolute_padding_box_chrome159.json")).unwrap();
    assert_eq!(control["browser"], "159.0.8154.6");
    assert_eq!(control["cases"].as_array().unwrap().len(), 48);
    assert_eq!(
        control["probe_sha256"],
        format!(
            "{:x}",
            Sha256::digest(include_bytes!(
                "../../../scripts/absolute-padding-box-chrome-control.mjs"
            ))
        )
    );
    control
}

fn assert_boxes(actual: &[(String, [f32; 4])], case: &Value) {
    for expected in case["boxes"].as_array().unwrap() {
        let cid = expected["cid"].as_str().unwrap();
        let got = &actual.iter().find(|(id, _)| id == cid).unwrap().1;
        for (index, field) in ["x", "y", "width", "height"].iter().enumerate() {
            let want = expected[field].as_f64().unwrap() as f32;
            assert!(
                (got[index] - want).abs() <= 0.02,
                "{}/{cid}.{field}: native {} (rect {got:?}), Chromium {want}",
                case["name"],
                got[index]
            );
        }
    }
}

#[test]
fn static_absolute_padding_boxes_match_chromium159() {
    let control = control();
    for case in control["cases"].as_array().unwrap() {
        let boxes = layout_boxes(
            case["html"].as_str().unwrap(),
            "",
            800.0,
            600.0,
            &FontRegistry::new(),
            &mut Diagnostics::default(),
        )
        .unwrap();
        assert_boxes(
            &boxes
                .iter()
                .map(|b| (b.cid.clone(), [b.x, b.y, b.width, b.height]))
                .collect::<Vec<_>>(),
            case,
        );
    }
}

fn identities(nodes: &[Node], path: &mut Vec<usize>, rows: &mut Vec<(String, SourceIdentity)>) {
    for (index, node) in nodes.iter().enumerate() {
        path.push(index);
        if let Node::Element(element) = node {
            if let Some(attr) = element.attrs.iter().find(|attr| attr.name == "data-cid") {
                rows.push((
                    attr.value.clone(),
                    SourceIdentity {
                        id: SourceId(rows.len() as u64 + 1),
                        path: path.clone(),
                    },
                ));
            }
            identities(&element.children, path, rows);
        }
        path.pop();
    }
}

#[test]
fn live_absolute_padding_boxes_match_chromium159() {
    let control = control();
    for case in control["cases"].as_array().unwrap() {
        let nodes = literal::parse_html(case["html"].as_str().unwrap()).unwrap();
        let mut rows = Vec::new();
        identities(&nodes, &mut Vec::new(), &mut rows);
        let sources = rows.iter().map(|(_, id)| id.clone()).collect::<Vec<_>>();
        let fonts = FontRegistry::new();
        let images = ImageCtx {
            resolver: &NoImages,
            body_height: None,
        };
        let snapshot = snapshot_html_live_tree_with_native_text(
            &literal::tree(&nodes).input(),
            &sources,
            &build_cascade("", "", turbo_dom_layout::style::TokenSet::default()),
            &NativeTextOptions {
                original_text: &[],
                svg_viewports: &[],
                layout_scale: 1.0,
            },
            &SnapshotInputs {
                width: 800.0,
                height: 600.0,
                fonts: &fonts,
                images: &images,
            },
            &mut Diagnostics::default(),
        )
        .unwrap();
        let boxes = rows
            .iter()
            .map(|(cid, id)| {
                let source = snapshot.source(id.id).unwrap();
                let rect = snapshot.fragments()[source.fragment_indices[0]].rect;
                (cid.clone(), [rect.x, rect.y, rect.width, rect.height])
            })
            .collect::<Vec<_>>();
        assert_boxes(&boxes, case);
    }
}
