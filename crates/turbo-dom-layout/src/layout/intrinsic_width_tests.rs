use super::*;

fn fixture() -> (FontRegistry, Vec<InlineRun>) {
    let [face, second] = FontFace::collection_fixture_faces();
    let mut fonts = FontRegistry::new();
    fonts.add(face.clone());
    fonts.add(second.clone());
    let make = |node_id, text: &str, face| InlineRun {
        node_id: NodeId(node_id),
        text: text.into(),
        face,
        families: vec!["same CSS alias".into()],
        weight: 400,
        italic: false,
        font_size: 16.0,
        line_height: Some(25.0),
        letter_spacing: 0.0,
        color: Rgba::BLACK,
        valign: VAlign::Baseline,
        nowrap: false,
        preserve_spaces: false,
        forced_newlines: false,
        text_transform: crate::text::TextTransform::default(),
        word_breaking: WordBreaking::default(),
        upright: UprightText::None,
    };
    (
        fonts,
        vec![
            make(1, "office alpha ", face),
            make(2, "beta אבג 中文", second),
        ],
    )
}

fn compare(
    runs: &[InlineRun],
    fonts: &FontRegistry,
    original: Option<&crate::layout_snapshot::text_units::NativeUnitMap>,
) {
    for policy in [
        (false, None),
        (true, None),
        (true, Some(1.0)),
        (true, Some(1.25)),
        (true, Some(2.0)),
    ] {
        for width in [0.0, 1.0, 33.25, f32::MAX] {
            let full = layout_runs_with_units(
                runs,
                fonts,
                width,
                &mut Diagnostics::default(),
                original,
                policy,
            );
            let measured = measure_runs_with_units(
                runs,
                fonts,
                width,
                &mut Diagnostics::default(),
                original,
                policy,
            );
            assert_eq!(
                measured.to_bits(),
                full.width.to_bits(),
                "width={width}, policy={policy:?}"
            );
        }
    }
}

#[test]
fn intrinsic_widths_match_full_mixed_font_bidi_and_negative_spacing_lines() {
    let (fonts, mut runs) = fixture();
    compare(&runs, &fonts, None);
    runs[0].letter_spacing = -1.75;
    runs[1].font_size = 23.25;
    runs[1].valign = VAlign::Super;
    compare(&runs, &fonts, None);
    runs[1].upright = UprightText::All;
    compare(&runs, &fonts, None);
}

#[test]
fn intrinsic_widths_preserve_whitespace_nowrap_newlines_and_break_opportunities() {
    let (fonts, mut runs) = fixture();
    runs[0].text = "  one\t two\nthree  ".into();
    runs[1].text = "auto\u{00ad}matic longword\u{200b}next".into();
    compare(&runs, &fonts, None);
    runs[0].preserve_spaces = true;
    runs[0].forced_newlines = true;
    runs[1].nowrap = true;
    compare(&runs, &fonts, None);
    runs[1].nowrap = false;
    runs[1].word_breaking.break_all = true;
    compare(&runs, &fonts, None);
    runs[1].word_breaking = WordBreaking::from_computed(None, Some("anywhere"), None);
    compare(&runs, &fonts, None);
    runs[1].text = "internationalization automatic".into();
    runs[1].word_breaking =
        WordBreaking::default().with_auto_hyphens(Some("auto"), Some("en"), None);
    compare(&runs, &fonts, None);
    runs[0].text.clear();
    runs[1].text.clear();
    compare(&runs, &fonts, None);
}

#[test]
fn intrinsic_widths_preserve_original_utf16_and_source_rounding() {
    let (fonts, mut runs) = fixture();
    runs.truncate(1);
    runs[0].text = "a\u{fffd}b".into();
    let original = [(NodeId(1), std::sync::Arc::from([0x61, 0xd800, 0x62]))].into();
    compare(&runs, &fonts, Some(&original));
}

#[cfg(feature = "perf")]
#[test]
fn production_min_max_width_queries_emit_no_positioned_glyph_lines() {
    use crate::layout::boxgen::build_box_tree;
    use crate::style::{build_cascade, TokenSet};
    use crate::test_support::literal::{parse_html, style_nodes};
    let nodes = parse_html("<div>alpha beta gamma</div>").unwrap();
    let cascade = build_cascade(
        "*{font-size:16px;font-family:'same CSS alias'}",
        "",
        TokenSet::default(),
    );
    let tree = build_box_tree(&style_nodes(&nodes, &cascade));
    let (fonts, _) = fixture();
    crate::perf::reset();
    let max = crate::layout::flex::natural_width(&tree, &fonts);
    let min = crate::layout::flex::min_content_width(&tree, &fonts);
    assert!(max >= min && min > 0.0);
    assert!(crate::perf::count("layout.intrinsic_line.width_only") >= 2);
    assert_eq!(crate::perf::count("layout.inline_line.emit"), 0);
}
