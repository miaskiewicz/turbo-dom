//! An SVG `<img>` whose root has a `viewBox` and no `width`/`height` against Chrome 156
//! (miaskiewicz/turbo-dom#320, turbo-surf#161): the image has a natural aspect ratio and no
//! natural size, so an auto-sized box is as wide as its containing block allows (CSS
//! Images 3 default sizing; Blink derives the height in layout units first, then the width
//! from it), while `naturalWidth`/`naturalHeight` still report the ratio's fit in 300x150.
//! In a shrink-to-fit context (float, inline-block, table) the image contributes no
//! width, so it lays out 0x0.
//! Oracle: `tests/fixtures/svg_image_ratio_chrome156.json`, captured by
//! `scripts/svg-image-ratio-chrome-control.mjs` (Chrome 156.0.8078.47 headless, 1280x800,
//! DSF 1). The live drive sizes each image from the control's natural size and ratio,
//! which is what [`probe_natural`] reports for it (asserted under `svg-rasterize`, the
//! feature that probe needs), as turbo-surf's live image loads do; so the layout rows
//! run in every feature configuration. Since miaskiewicz/turbo-dom#326 that includes
//! flex and grid items.
use std::collections::HashMap;
use std::fmt::Write as _;
#[cfg(feature = "svg-rasterize")]
use turbo_dom_layout::image::probe_natural;
use turbo_dom_layout::image::Intrinsic;
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

/// The case SVGs by id, each with the intrinsic [`probe_natural`] reports for it.
struct Svgs(HashMap<String, (Vec<u8>, Intrinsic)>);

impl ImageResolver for Svgs {
    fn resolve(&self, name: &str) -> Option<&[u8]> {
        self.0.get(name).map(|(bytes, _)| bytes.as_slice())
    }

    fn intrinsic(&self, name: &str) -> Option<Intrinsic> {
        self.0.get(name).map(|&(_, intrinsic)| intrinsic)
    }
}

fn pair(value: &serde_json::Value) -> (f64, f64) {
    (value[0].as_f64().unwrap(), value[1].as_f64().unwrap())
}

/// A case's natural ratio: only an SVG with just a `viewBox` has one (its `viewBox`'s).
fn natural_ratio(id: &str) -> Option<f32> {
    id.starts_with("viewbox-only").then(|| {
        if id.ends_with("7x3") {
            7.0 / 3.0
        } else if id.ends_with("wide") {
            9.0
        } else {
            1.5
        }
    })
}

/// A case's intrinsic: Chrome's `naturalWidth`/`naturalHeight`, plus its natural ratio.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn case_intrinsic(case: &serde_json::Value) -> Intrinsic {
    let (w, h) = pair(&case["natural"]);
    let (w, h) = (w as u32, h as u32);
    match natural_ratio(case["id"].as_str().unwrap()) {
        Some(ratio) => Intrinsic::ratio_only(w, h, ratio, true),
        None => Intrinsic::new(w, h, true),
    }
}

#[cfg(feature = "svg-rasterize")]
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
        assert_eq!(
            intrinsic.natural_ratio,
            natural_ratio(id),
            "{id}: natural ratio"
        );
        assert_eq!(
            intrinsic,
            case_intrinsic(case),
            "{id}: the layout test's intrinsic"
        );
    }
}

#[test]
fn ratio_only_svg_images_lay_out_like_chrome_156() {
    let control = fixture();
    let cases = control["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 46);
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
            (
                case["svg"].as_str().unwrap().as_bytes().to_vec(),
                case_intrinsic(case),
            ),
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
        if !matches {
            failures.push(format!("{id}: {got:?}, Chrome {want:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
