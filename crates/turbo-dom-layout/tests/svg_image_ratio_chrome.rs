//! An SVG `<img>` whose root has a `viewBox` and no `width`/`height` against Chrome 156
//! (miaskiewicz/turbo-dom#320, turbo-surf#161): the image has a natural aspect ratio and no
//! natural size, so an auto-sized box is as wide as its containing block allows (CSS
//! Images 3 default sizing; Blink derives the height in layout units first, then the width
//! from it), while `naturalWidth`/`naturalHeight` still report the ratio's fit in 300x150.
//! In a shrink-to-fit context (float, inline-block, table) the image contributes no
//! width, so it lays out 0x0.
//! Oracle: `tests/fixtures/svg_image_ratio_chrome156.json`, captured by
//! `scripts/svg-image-ratio-chrome-control.mjs` (Chrome 156.0.8078.47 headless, 1280x800,
//! DSF 1). The live drive sizes each image from [`probe_natural`], as turbo-surf's live
//! image loads do.
#![cfg(feature = "svg-rasterize")]
use std::collections::HashMap;
use std::fmt::Write as _;
use turbo_dom_layout::image::{probe_natural, Intrinsic};
use turbo_dom_layout::layout_snapshot::{
    snapshot_html_live_tree_with_native_text, NativeTextOptions, SnapshotInputs, SourceId,
    SourceIdentity,
};
use turbo_dom_layout::test_support::literal;
use turbo_dom_layout::test_support::literal::parse_html;
use turbo_dom_layout::{build_cascade, Diagnostics, FontRegistry, ImageCtx, ImageResolver};

fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!("fixtures/svg_image_ratio_chrome156.json")).unwrap()
}

/// The case SVGs by id; sized like turbo-surf's live loads ([`probe_natural`]).
struct Svgs(HashMap<String, Vec<u8>>);

impl ImageResolver for Svgs {
    fn resolve(&self, name: &str) -> Option<&[u8]> {
        self.0.get(name).map(Vec::as_slice)
    }

    fn intrinsic(&self, name: &str) -> Option<Intrinsic> {
        self.resolve(name).and_then(probe_natural)
    }
}

fn pair(value: &serde_json::Value) -> (f64, f64) {
    (value[0].as_f64().unwrap(), value[1].as_f64().unwrap())
}

#[test]
fn natural_size_is_chromes_and_only_a_viewbox_gives_a_bare_ratio() {
    let control = fixture();
    assert_eq!(control["chrome"], "156.0.8078.47");
    for case in control["cases"].as_array().unwrap() {
        let id = case["id"].as_str().unwrap();
        let intrinsic = probe_natural(case["svg"].as_str().unwrap().as_bytes()).unwrap();
        assert_eq!(
            (f64::from(intrinsic.width), f64::from(intrinsic.height)),
            pair(&case["natural"]),
            "{id}: naturalWidth/naturalHeight"
        );
        let expected = id.starts_with("viewbox-only").then(|| {
            if id.ends_with("7x3") {
                7.0 / 3.0
            } else if id.ends_with("wide") {
                9.0
            } else {
                1.5
            }
        });
        assert_eq!(intrinsic.natural_ratio, expected, "{id}: natural ratio");
    }
}

/// The one control row not matched yet: a ratio-only image as a row flex item
/// (miaskiewicz/turbo-dom#326).
const KNOWN_GAP: &str = "viewbox-only-flex-item";

#[test]
fn ratio_only_svg_images_lay_out_like_chrome_156() {
    let control = fixture();
    let cases = control["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 24);
    let mut html = String::new();
    let mut svgs = HashMap::new();
    for case in cases {
        let id = case["id"].as_str().unwrap();
        write!(
            html,
            "<div style=\"margin:0 0 4px;{}\"><img src=\"{id}\" style=\"{}\"></div>",
            case["wrap"].as_str().unwrap(),
            case["style"].as_str().unwrap(),
        )
        .unwrap();
        svgs.insert(
            id.to_owned(),
            case["svg"].as_str().unwrap().as_bytes().to_vec(),
        );
    }
    let nodes = parse_html(&html).unwrap();
    let identities: Vec<SourceIdentity> = (0..cases.len())
        .map(|i| SourceIdentity {
            id: SourceId(i as u64),
            path: vec![i, 0],
        })
        .collect();
    let resolver = Svgs(svgs);
    let fonts = FontRegistry::new();
    let snapshot = snapshot_html_live_tree_with_native_text(
        &literal::tree(&nodes).input(),
        &identities,
        &build_cascade("", "", turbo_dom_layout::style::TokenSet::default()),
        &NativeTextOptions {
            original_text: &[],
            svg_viewports: &[],
            layout_scale: 1.0,
        },
        &SnapshotInputs {
            // The page's body content box (1280 minus the 8px body margins).
            width: 1264.0,
            height: 800.0,
            fonts: &fonts,
            images: &ImageCtx {
                resolver: &resolver,
                body_height: None,
            },
        },
        &mut Diagnostics::default(),
    )
    .unwrap();
    let mut failures = Vec::new();
    for (i, case) in cases.iter().enumerate() {
        let id = case["id"].as_str().unwrap();
        let source = snapshot.source(SourceId(i as u64)).unwrap();
        let rect = snapshot.fragments()[source.fragment_indices[0]].rect;
        let got = (f64::from(rect.width), f64::from(rect.height));
        let want = pair(&case["size"]);
        let matches = (got.0 - want.0).abs() <= 1e-3 && (got.1 - want.1).abs() <= 1e-3;
        if id == KNOWN_GAP {
            // miaskiewicz/turbo-dom#326: still the natural size, not the container's
            // width; this fails once that is fixed, so the gap is lifted then.
            assert_eq!(got, (225.0, 150.0), "{id}: the #326 gap changed");
            assert!(!matches);
        } else if !matches {
            failures.push(format!("{id}: {got:?}, Chrome {want:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
