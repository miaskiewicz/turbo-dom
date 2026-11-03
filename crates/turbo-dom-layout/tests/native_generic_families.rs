//! The macOS host-font policy of the CSS generic families (miaskiewicz/turbo-dom#259),
//! against Chrome 156 on the same host (`scripts/native-generic-families-chrome-control.mjs`
//! -> `fixtures/native-generic/chrome156.json`). Host fonts by design: what is tested is
//! the installed-face lookup, so fixture fonts would test nothing (CLAUDE.md).
#![cfg(target_os = "macos")]
use turbo_dom_layout::text::SystemFontPolicy;
use turbo_dom_layout::FontRegistry;

/// Chrome's `line-height: normal` line box height and "Hello world" advance at 16px
/// for `family`.
fn chrome(family: &str) -> (f32, f32) {
    let control: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/native-generic/chrome156.json")).unwrap();
    assert_eq!(control["platform"], "darwin");
    let case = control["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["family"] == family)
        .unwrap();
    #[allow(clippy::cast_possible_truncation)]
    let read = |key: &str| case[key].as_f64().unwrap() as f32;
    (read("lineHeight"), read("width"))
}

/// The same reads for `family` resolved in `fonts`.
fn ours(fonts: &FontRegistry, family: &str) -> (f32, f32) {
    let face = fonts.select(&[family], 400, false).unwrap();
    let metrics = face.html_metrics(16.0);
    (
        metrics.ascent + metrics.descent + metrics.line_gap,
        face.measure("Hello world", 16.0, 0.0),
    )
}

#[test]
fn sans_serif_is_helvetica_like_chrome_under_both_policies() {
    let html = FontRegistry::load_system(SystemFontPolicy::Html);
    let helvetica = html.select(&["Helvetica"], 400, false).unwrap();
    for policy in [SystemFontPolicy::Default, SystemFontPolicy::Html] {
        let fonts = FontRegistry::load_system(policy);
        let sans = fonts.select(&["sans-serif"], 400, false).unwrap();
        assert!(
            sans.data() == helvetica.data(),
            "{policy:?}: sans-serif is not Helvetica"
        );
        assert_eq!(sans.index(), helvetica.index(), "{policy:?}");
        assert_eq!(
            ours(&fonts, "sans-serif"),
            chrome("sans-serif"),
            "{policy:?}"
        );
    }
    assert_eq!(ours(&html, "Helvetica"), chrome("Helvetica"));
}

#[test]
fn serif_and_pdf_monospace_keep_chromes_metrics() {
    // serif: Times (HTML) / Times New Roman (PDF, native_serif_policy), the same
    // metrics in Chrome; monospace under the default (PDF / raster) policy: Courier
    // New, as Chrome's generic resolves.
    for policy in [SystemFontPolicy::Default, SystemFontPolicy::Html] {
        let fonts = FontRegistry::load_system(policy);
        assert_eq!(ours(&fonts, "serif"), chrome("serif"), "{policy:?}");
    }
    // (Its "Hello world" through `FontFace::measure` is 105.617, not Chrome's
    // 105.625: that is the measuring API's advance rounding, not the face; the face is
    // pinned by identity and its line height.)
    let pdf = FontRegistry::load_system(SystemFontPolicy::Default);
    let mono = pdf.select(&["monospace"], 400, false).unwrap();
    let courier = pdf.select(&["Courier New"], 400, false).unwrap();
    assert!(mono.data() == courier.data() && mono.index() == courier.index());
    assert_eq!(ours(&pdf, "monospace").0, chrome("monospace").0);
}

#[test]
fn html_monospace_is_courier_like_chrome() {
    // miaskiewicz/turbo-dom#260: Chrome 156's macOS generic monospace is Courier (its
    // 14 / 4 ascent / descent with the legacy ascent adjustment), not Menlo.
    let html = FontRegistry::load_system(SystemFontPolicy::Html);
    let mono = html.select(&["monospace"], 400, false).unwrap();
    let courier = html.select(&["Courier"], 400, false).unwrap();
    assert!(mono.data() == courier.data() && mono.index() == courier.index());
    assert_eq!(chrome("monospace"), chrome("Courier"));
    // Line height only: `FontFace::measure` rounds this face's "Hello world" to
    // 105.617 (Chrome 105.625), as for Courier New above.
    assert_eq!(ours(&html, "monospace").0, chrome("monospace").0);
}
