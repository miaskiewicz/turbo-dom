//! Percentage edges resolve against the flex containing block or grid area.
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use turbo_dom_layout::layout_snapshot::{
    snapshot_html_styled_tree_with_svg_viewports, GeometryKind, SnapshotInputs, SourceId,
    SourceIdentity,
};
use turbo_dom_layout::{
    layout_boxes_tree, parse_html_tree, style_html_tree, with_fractional_css_geometry, Diagnostics,
    FontRegistry, HtmlOptions, ImageCtx, NoImages, StyledNode, Tag,
};

const CASES: &str = include_str!("fixtures/percentage_frame_cases.json");
const CONTROL: &str = include_str!("fixtures/percentage_frame_chrome159.json");

fn identify(
    nodes: &[StyledNode],
    parent: &[usize],
    identities: &mut Vec<SourceIdentity>,
    viewports: &mut Vec<SourceId>,
    keys: &mut BTreeMap<String, SourceId>,
) {
    for (index, node) in nodes.iter().enumerate() {
        let mut path = parent.to_vec();
        path.push(index);
        let id = SourceId(identities.len() as u64 + 1);
        identities.push(SourceIdentity {
            id,
            path: path.clone(),
        });
        if let StyledNode::Element(element) = node {
            if matches!(&element.tag, Tag::Html(name) if name == "svg") {
                viewports.push(id);
            }
            for attr in &element.attrs {
                if attr.name == "data-cid" {
                    keys.insert(attr.value.clone(), id);
                }
            }
            identify(&element.children, &path, identities, viewports, keys);
        }
    }
}

fn svg_rects(tree: &turbo_dom::Tree, fonts: &FontRegistry) -> BTreeMap<String, [f32; 4]> {
    let styled = style_html_tree(tree, "", 600.0, &HtmlOptions::default());
    let mut identities = Vec::new();
    let mut viewports = Vec::new();
    let mut keys = BTreeMap::new();
    identify(&styled, &[], &mut identities, &mut viewports, &mut keys);
    let images = ImageCtx {
        resolver: &NoImages,
        body_height: None,
    };
    let snapshot = snapshot_html_styled_tree_with_svg_viewports(
        &styled,
        &identities,
        &viewports,
        1.0,
        &SnapshotInputs {
            width: 600.0,
            height: 1000.0,
            fonts,
            images: &images,
        },
        &mut Diagnostics::default(),
    )
    .unwrap();
    keys.into_iter()
        .map(|(key, id)| {
            let source = snapshot.source(id).unwrap();
            let boxes: Vec<_> = source
                .fragment_indices
                .iter()
                .map(|index| &snapshot.fragments()[*index])
                .filter(|f| f.kind == GeometryKind::BorderBox)
                .collect();
            assert_eq!(boxes.len(), 1, "{key}");
            let r = boxes[0].transformed_bounds;
            (key, [r.x, r.y, r.width, r.height])
        })
        .collect()
}

#[test]
fn percentage_frames_match_chromium_159() {
    let cases: Value = serde_json::from_str(CASES).unwrap();
    let control: Value = serde_json::from_str(CONTROL).unwrap();
    assert_eq!(control["browser"], "159.0.8154.6");
    assert_eq!(control["offline"], true);
    assert_eq!(
        control["casesSha256"],
        format!("{:x}", Sha256::digest(CASES.as_bytes()))
    );
    assert_eq!(
        control["collectorSha256"],
        format!(
            "{:x}",
            Sha256::digest(include_bytes!(
                "../../../scripts/percentage-frame-chrome-control.mjs"
            ))
        )
    );
    let rows = control["rows"].as_array().unwrap();
    assert_eq!(cases.as_array().unwrap().len(), rows.len());
    let fonts = FontRegistry::new();
    let mut failures = Vec::new();
    for (input, expected) in cases.as_array().unwrap().iter().zip(rows) {
        assert_eq!(input["id"], expected["id"]);
        let mut tree = parse_html_tree(input["html"].as_str().unwrap());
        for selector in [".grid", ".item", ".inner", ".next"] {
            let node = tree.query_selector_all(selector)[0];
            tree.set_attribute(node, "data-cid", selector);
        }
        let boxes = with_fractional_css_geometry(|| {
            layout_boxes_tree(
                &tree,
                "",
                600.0,
                1000.0,
                &fonts,
                &HtmlOptions::default(),
                &mut Diagnostics::default(),
            )
        });
        let svg = input["id"]
            .as_str()
            .unwrap()
            .starts_with("svg_")
            .then(|| svg_rects(&tree, &fonts));
        for wanted in expected["boxes"].as_array().unwrap() {
            let key = wanted["key"].as_str().unwrap();
            let actual = svg.as_ref().map_or_else(
                || {
                    let b = boxes.iter().find(|item| item.cid == key).unwrap();
                    [b.x, b.y, b.width, b.height]
                },
                |boxes| boxes[key],
            );
            for (axis, value) in actual.into_iter().enumerate() {
                let expected = wanted["rect"][axis].as_f64().unwrap();
                if (f64::from(value) - expected).abs() > 0.05 {
                    failures.push(format!(
                        "{} {key} axis{axis}: native {value}, Chromium {expected}",
                        input["id"]
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
