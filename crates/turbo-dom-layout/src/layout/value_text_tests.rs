use super::*;

fn from_box_style(style: &ComputedStyle, parent: f32) -> TextRunStyle {
    let used = resolve_box_style(
        style,
        ResolveCtx {
            parent_font_size: parent,
            cb_width: 347.0,
        },
    );
    TextRunStyle {
        font_families: used.font_families,
        font_weight: used.font_weight,
        italic: used.italic,
        font_size: used.font_size,
        line_height: used.line_height,
        letter_spacing: used.letter_spacing,
        color: used.color,
        vertical_align: used.vertical_align,
    }
}

#[test]
fn text_values_match_complete_box_resolution_for_relative_fonts_and_alignment() {
    let sizes = ["18px", "1.25em", "125%", "1.5rem", "invalid"];
    let alignments = [
        "baseline", "middle", "top", "bottom", "super", "sub", "20%", "3px",
    ];
    let _root_font = crate::root_font_scope(24.0);
    for size in sizes {
        for alignment in alignments {
            let style = ComputedStyle::from_pairs([
                ("font-family", "'Go', sans-serif"),
                ("font-size", size),
                ("font-style", "oblique"),
                ("font-weight", "700"),
                ("line-height", "1.25"),
                ("letter-spacing", "0.125em"),
                ("color", "#123456"),
                ("vertical-align", alignment),
                ("border", "8px ridge blue"),
                ("background", "linear-gradient(red, blue)"),
                ("padding", "25%"),
                ("transform", "rotate(20deg)"),
            ]);
            assert_eq!(
                resolve_text_run_style(&style, 20.0),
                from_box_style(&style, 20.0)
            );
        }
    }
}

#[test]
fn text_values_preserve_explicit_font_spacing_color_and_defaults() {
    let style = ComputedStyle::from_pairs([
        ("font-family", "'Go', sans-serif"),
        ("font-size", "150%"),
        ("font-style", "italic"),
        ("font-weight", "bold"),
        ("line-height", "2em"),
        ("letter-spacing", "0.125em"),
        ("color", "#123456"),
        ("vertical-align", "super"),
    ]);
    let actual = resolve_text_run_style(&style, 20.0);
    assert_eq!(actual.font_families, vec!["Go", "sans-serif"]);
    assert_eq!(
        (actual.font_weight, actual.italic, actual.font_size),
        (700, true, 30.0)
    );
    assert_eq!(
        (actual.line_height, actual.letter_spacing),
        (Some(60.0), 3.75)
    );
    assert_eq!(
        actual.color,
        Rgba {
            r: 0x12,
            g: 0x34,
            b: 0x56,
            a: 255
        }
    );
    assert_eq!(actual.vertical_align, VAlign::Super);
    let empty = ComputedStyle::default();
    assert_eq!(
        resolve_text_run_style(&empty, 17.0),
        from_box_style(&empty, 17.0)
    );
}

#[test]
fn text_values_preserve_document_scopes_without_a_new_cache() {
    let style = ComputedStyle::from_pairs([
        ("font-size", "1rem"),
        ("line-height", "2vh"),
        ("letter-spacing", "1vw"),
        ("color", "cmyk(0, 1, 1, 0)"),
    ]);
    let previous_viewport = crate::style::viewport_px();
    for (root, cmyk, width, height) in [(18.0, false, 600.0, 400.0), (32.0, true, 1200.0, 800.0)] {
        crate::style::set_media_viewport_width(width);
        crate::style::set_media_viewport_height(height);
        let _root = crate::root_font_scope(root);
        let _cmyk = cmyk_scope(cmyk);
        assert_eq!(
            resolve_text_run_style(&style, 20.0),
            from_box_style(&style, 20.0)
        );
        let actual = resolve_text_run_style(&style, 20.0);
        assert_eq!(actual.font_size, root);
        assert_eq!(actual.line_height, Some(height * 0.02));
        assert_eq!(actual.letter_spacing, width * 0.01);
        assert_eq!(
            actual.color,
            if cmyk {
                Rgba {
                    r: 255,
                    g: 0,
                    b: 0,
                    a: 255,
                }
            } else {
                Rgba::BLACK
            }
        );
    }
    crate::style::set_media_viewport_width(previous_viewport.0);
    crate::style::set_media_viewport_height(previous_viewport.1);
}

#[test]
fn relative_font_sizes_and_text_defaults_have_independent_expected_values() {
    let _root = crate::root_font_scope(24.0);
    for (raw, size) in [
        ("1.5em", 30.0),
        ("150%", 30.0),
        ("2rem", 48.0),
        ("invalid", 20.0),
    ] {
        let style = ComputedStyle::from_pairs([
            ("font-size", raw),
            ("line-height", "2"),
            ("letter-spacing", "0.25em"),
        ]);
        let actual = resolve_text_run_style(&style, 20.0);
        assert_eq!(actual.font_size, size);
        assert_eq!(actual.line_height, Some(2.0 * size));
        assert_eq!(actual.letter_spacing, 0.25 * size);
    }
    assert_eq!(
        resolve_text_run_style(&ComputedStyle::default(), 17.0),
        TextRunStyle {
            font_families: vec!["serif".to_owned()],
            font_weight: 400,
            italic: false,
            font_size: 17.0,
            line_height: None,
            letter_spacing: 0.0,
            color: Rgba::BLACK,
            vertical_align: VAlign::Baseline,
        }
    );
}

#[test]
fn unsupported_spacing_and_color_keep_the_existing_fallbacks() {
    for line_height in ["normal", "120%"] {
        let style = ComputedStyle::from_pairs([
            ("font-size", "20px"),
            ("line-height", line_height),
            ("letter-spacing", "normal"),
            ("color", "currentColor"),
        ]);
        let actual = resolve_text_run_style(&style, 16.0);
        assert_eq!(actual.line_height, None);
        assert_eq!(actual.letter_spacing, 0.0);
        assert_eq!(actual.color, Rgba::BLACK);
        assert_eq!(actual, from_box_style(&style, 16.0));
    }
}
