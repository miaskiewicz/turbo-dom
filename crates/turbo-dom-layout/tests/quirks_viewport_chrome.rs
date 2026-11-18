//! Physical viewport fill through the public live-DOM snapshot, not a CSSOM clamp.
use std::collections::HashMap;

use serde_json::Value;
use sha2::{Digest, Sha256};
use turbo_dom::rtdom::query::{match_for_cascade, CascadeSelector, LiveNavigator};
use turbo_dom::rtdom::tree::NodeType;
use turbo_dom::{Handle, Tree};
use turbo_dom_layout::cssom_view::metrics::{element_geometry, ElementGeometry};
use turbo_dom_layout::layout_snapshot::{LayoutSnapshot, SnapshotInputs};
use turbo_dom_layout::live::{
    self, LiveOptions, LiveStyle, LiveTree, MatchedTarget, OriginRules, PreMatched,
};
use turbo_dom_layout::style::{self, CascadeOrigin};
use turbo_dom_layout::{Diagnostics, FontRegistry, ImageCtx, ImageResolver};

const CASES: &str = include_str!("fixtures/quirks_viewport_cases.json");
const ORACLE: &str = include_str!("fixtures/quirks_viewport_chrome159.json");
const DOCUMENT: u32 = 1;
const LIVE_UA: &str = "html,body,p,div{display:block}body{margin:8px;font-family:serif}head,style,script,title,meta,link,base,noscript{display:none}";

struct NoImages;
impl ImageResolver for NoImages {
    fn resolve(&self, _: &str) -> Option<&[u8]> {
        None
    }
}

fn targets(tree: &Tree, rule: &style::LeveledRule) -> Vec<MatchedTarget> {
    let nav = LiveNavigator(tree);
    tree.handles()
        .filter(|&h| tree.node_type(h) == NodeType::Element)
        .filter_map(|h| {
            let specificity = rule
                .rule
                .selectors
                .selectors()
                .iter()
                .filter(|selector| match_for_cascade(&nav, h, selector))
                .map(CascadeSelector::specificity)
                .max()?;
            Some(MatchedTarget {
                source: live::source_id(DOCUMENT, h),
                specificity,
            })
        })
        .collect()
}

fn prematch(tree: &Tree, css: &str) -> [(CascadeOrigin, Vec<PreMatched>); 2] {
    let mut cascade = style::build_cascade_with_width(css, "", style::TokenSet::default(), 640.0);
    let start = cascade.rules.len();
    cascade.rules.extend(
        style::parse_stylesheet(LIVE_UA)
            .rules
            .into_iter()
            .enumerate()
            .map(|(i, rule)| style::LeveledRule {
                level: 0,
                order: start + i,
                rule,
            }),
    );
    cascade.rules.sort_by_key(|rule| (rule.level, rule.order));
    let mut origins = [
        (CascadeOrigin::UserAgent, Vec::new()),
        (CascadeOrigin::Author, Vec::new()),
    ];
    for rule in &cascade.rules {
        origins[usize::from(rule.level > 0)].1.push(PreMatched {
            depth: 0,
            declarations: rule.rule.declarations.clone(),
            targets: targets(tree, rule),
        });
    }
    origins
}

fn snapshot(html: &str, fonts: &FontRegistry) -> (Tree, LayoutSnapshot) {
    let tree = Tree::parse(html);
    let children: HashMap<Handle, Vec<Handle>> =
        tree.handles().map(|h| (h, tree.children(h))).collect();
    let provider = |h| children.get(&h).map_or(&[][..], Vec::as_slice);
    let live_tree = LiveTree {
        tree: &tree,
        document: DOCUMENT,
        children: &provider,
    };
    let captured = live::capture(&live_tree).unwrap();
    assert_eq!(captured.quirks, !html.starts_with("<!doctype html>"));
    let css = html
        .split_once("<style>")
        .unwrap()
        .1
        .split_once("</style>")
        .unwrap()
        .0;
    let origins = prematch(&tree, css);
    let origins: Vec<_> = origins
        .iter()
        .map(|(origin, rules)| OriginRules {
            origin: *origin,
            rules: rules.iter().map(Some).collect(),
        })
        .collect();
    let input = LiveStyle {
        tree: live_tree,
        origins: &origins,
        scope_depth: &|_| Ok(0),
        inline_override: &|_| None,
        inline_declarations: &|text| Ok(style::parse_declarations(text)),
    };
    let images = ImageCtx {
        resolver: &NoImages,
        body_height: None,
    };
    let inputs = SnapshotInputs {
        width: 640.0,
        height: 480.0,
        fonts,
        images: &images,
    };
    let (snapshot, ()) = live::snapshot(
        &input,
        &captured,
        &LiveOptions { layout_scale: 1.0 },
        |_| Ok(()),
        &inputs,
        &mut Diagnostics::default(),
    )
    .unwrap();
    (tree, snapshot)
}

fn handle(tree: &Tree, tag: &str) -> Handle {
    tree.handles()
        .find(|&h| tree.local_name(h) == Some(tag))
        .unwrap()
}

fn compare_geometry(id: &str, tag: &str, actual: &ElementGeometry, expected: &Value) {
    // The explicit fixture registry keeps font metrics independent of host fonts.
    let tolerance = 0.01;
    let rect = actual.bounds;
    for (index, value) in [rect.x, rect.y, rect.width, rect.height]
        .into_iter()
        .enumerate()
    {
        let wanted = expected["rect"][index].as_f64().unwrap();
        assert!(
            (f64::from(value) - wanted).abs() <= tolerance,
            "{id} {tag} rect[{index}]={value}, Chrome={wanted}"
        );
    }
    for (index, value) in [actual.offset.width, actual.offset.height]
        .into_iter()
        .enumerate()
    {
        let wanted = expected["offset"][index].as_f64().unwrap();
        assert!(
            (f64::from(value) - wanted).abs() <= tolerance,
            "{id} {tag} offset[{index}]={value}, Chrome={wanted}"
        );
    }
}

fn compare_children(id: &str, tree: &Tree, snapshot: &LayoutSnapshot, expected: &Value) {
    let elements: Vec<_> = tree
        .handles()
        .filter(|&h| matches!(tree.local_name(h), Some("p" | "div")))
        .collect();
    let children = expected["children"].as_array().unwrap();
    assert_eq!(elements.len(), children.len(), "{id}: child count");
    for (child, expected) in elements.into_iter().zip(children) {
        let actual = element_geometry(snapshot, live::source_id(DOCUMENT, child)).unwrap();
        compare_geometry(id, tree.local_name(child).unwrap(), &actual, expected);
    }
}

#[test]
fn public_live_quirks_viewport_matches_chrome_159() {
    let cases: Value = serde_json::from_str(CASES).unwrap();
    let oracle: Value = serde_json::from_str(ORACLE).unwrap();
    assert_eq!(
        oracle["casesSha256"],
        format!("{:x}", Sha256::digest(CASES.as_bytes()))
    );
    assert_eq!(
        oracle["collectorSha256"],
        format!(
            "{:x}",
            Sha256::digest(
                include_str!("../../../scripts/quirks-viewport-chrome-control.mjs").as_bytes()
            )
        )
    );
    assert_eq!(oracle["browser"], "159.0.8154.6");
    assert_eq!(oracle["viewport"], serde_json::json!([640, 480]));
    assert_eq!(oracle["deviceScaleFactor"], 1);
    assert_eq!(oracle["offline"], true);
    assert_eq!(
        cases[0]["html"],
        "<style>html,body{margin:0}</style><p>viewport</p>"
    );
    assert_eq!(
        cases[1]["html"],
        "<!doctype html><style>html,body{margin:0}</style><p>viewport</p>"
    );
    let fonts = turbo_dom_layout::test_support::fixture_fonts();
    for (input, expected) in cases
        .as_array()
        .unwrap()
        .iter()
        .zip(oracle["rows"].as_array().unwrap())
    {
        let id = input["id"].as_str().unwrap();
        assert_eq!(input["id"], expected["id"]);
        assert_eq!(input["html"], expected["markup"]);
        let (tree, snapshot) = snapshot(input["html"].as_str().unwrap(), &fonts);
        compare_children(id, &tree, &snapshot, expected);
        for tag in ["html", "body"] {
            let geometry =
                element_geometry(&snapshot, live::source_id(DOCUMENT, handle(&tree, tag)));
            let geometry = match geometry {
                Err(error) if id == "body-inline" && tag == "body" => {
                    assert!(error.contains("inline border/cluster provenance"));
                    continue;
                }
                Err(error) => panic!("{id} {tag}: {error}"),
                Ok(geometry) => geometry,
            };
            compare_geometry(id, tag, &geometry, &expected[tag]);
            if id == "original-no-doctype" && tag == "html" {
                assert_eq!(
                    geometry.client.height, 480,
                    "original public clientHeight regression"
                );
            }
        }
    }
    assert_eq!(
        cases.as_array().unwrap().len(),
        oracle["rows"].as_array().unwrap().len()
    );
}
