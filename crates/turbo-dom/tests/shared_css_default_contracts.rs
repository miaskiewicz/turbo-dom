//! Default-feature contracts shared by native descriptors and computed CSS.
use cssparser::{Parser, ParserInput};
use turbo_dom::rtdom::{
    computed_css::ComputationLimits,
    css_shorthand::{self, LengthContext, LengthError, Spelled},
    css_values::font_settings,
    query::{parse_for_cascade, parse_for_cascade_units},
};

fn font(feature: bool, required: bool, value: &str) -> Option<String> {
    let mut source = ParserInput::new(value);
    let mut parser = Parser::new(&mut source);
    let result = font_settings(feature, required, &mut parser)?;
    parser.expect_exhausted().ok()?;
    Some(result)
}

#[test]
fn font_tag_modes_preserve_descriptor_defaults_and_explicit_values() {
    for (source, expected) in [
        ("normal", "normal"),
        ("\"kern\"", "\"kern\""),
        ("\"kern\" on", "\"kern\""),
        ("\"kern\" off", "\"kern\" 0"),
        ("\"kern\" -1", "\"kern\" -1"),
        ("\"kern\" 2, \"liga\" off", "\"kern\" 2, \"liga\" 0"),
    ] {
        assert_eq!(
            font(true, false, source).as_deref(),
            Some(expected),
            "{source}"
        );
    }
    assert_eq!(
        font(false, false, "\"wght\"").as_deref(),
        Some("\"wght\" 1")
    );
    assert_eq!(font(false, true, "\"wght\""), None);
    assert_eq!(
        font(false, true, "\"wght\" -20.5").as_deref(),
        Some("\"wght\" -20.5")
    );
    for source in [
        "\"abc\"",
        "\"abcde\"",
        "\"éabc\"",
        "\"kern\" garbage",
        "\"kern\",",
        "normal garbage",
    ] {
        assert_eq!(font(true, false, source), None, "{source}");
    }
}

#[test]
fn longhand_families_validate_operands_and_preserve_css_whitespace() {
    for (name, source, expected) in [
        ("quotes", "auto", "auto"),
        ("quotes", "none", "none"),
        ("font-language-override", "normal", "normal"),
        ("font-language-override", "\"TRK\"", "\"TRK\""),
        ("font-size-adjust", "none", "none"),
        ("font-size-adjust", "from-font", "from-font"),
        ("font-size-adjust", "ex-height 0.5", "0.5"),
        (
            "font-size-adjust",
            "cap-height from-font",
            "cap-height from-font",
        ),
        ("position-anchor", "auto", "auto"),
        ("position-anchor", "--a", "--a"),
        ("anchor-scope", "all", "all"),
        ("anchor-scope", "--a, --b", "--a, --b"),
        ("anchor-name", "--", "--"),
        ("view-transition-class", "foo bar", "foo bar"),
    ] {
        assert_eq!(
            css_shorthand::longhand_value_css(name, source),
            Spelled::Valid(expected.into()),
            "{name}:{source}"
        );
    }
    for (name, source) in [
        ("quotes", "\"only-one\""),
        ("font-language-override", "\"too long\""),
        ("font-size-adjust", "-1"),
        ("font-size-adjust", "bad 1"),
        ("font-size-adjust", "cap-height bad"),
        ("font-size-adjust", "1 2 3"),
        ("anchor-name", "foo"),
        ("anchor-scope", "--a --b"),
        ("view-transition-class", "foo none"),
    ] {
        assert_eq!(
            css_shorthand::longhand_value_css(name, source),
            Spelled::Invalid,
            "{name}:{source}"
        );
    }
    for space in ['\u{a0}', '\u{85}', '\u{2003}'] {
        let value = format!("{space}red");
        assert_eq!(
            css_shorthand::longhand_value("color", &value),
            Spelled::Valid("red".into())
        );
        assert_eq!(
            css_shorthand::longhand_value_css("color", &value),
            Spelled::Invalid
        );
    }
    assert!(css_shorthand::expand_css("margin", " 1px\t2px ").is_some());
    assert!(css_shorthand::expand("margin", "\u{a0}1px").is_some());
    assert!(css_shorthand::expand_css("margin", "\u{a0}1px").is_none());
}

#[test]
fn length_errors_keep_unsupported_math_distinct_from_missing_context() {
    let limits = ComputationLimits {
        max_nesting: 32,
        max_output_units: 1024,
        max_total_units: 0,
    };
    let context = LengthContext { unit_bases: &[] };
    for source in ["round(up, 1px, 2px)", "calc-size(auto, size + 1px)"] {
        assert!(
            matches!(
                css_shorthand::computed_length_expression(source, &context, limits),
                Err(LengthError::UnsupportedMath(_))
            ),
            "{source}"
        );
    }
}

#[test]
fn selector_payload_accounts_for_scalar_and_retained_unit_sources() {
    let list = parse_for_cascade("div.example").unwrap();
    let selector = &list.selectors()[0];
    assert_eq!(
        selector.text_units().as_ref(),
        "div.example".encode_utf16().collect::<Vec<_>>()
    );
    assert!(selector.source_payload_bytes() >= selector.text().len());
    let mut source: Vec<u16> = "[data-value=\"".encode_utf16().collect();
    source.push(0xd800);
    source.extend("\"]".encode_utf16());
    let list = parse_for_cascade_units(&source).unwrap();
    let selector = &list.selectors()[0];
    assert_eq!(selector.text_units().as_ref(), source);
    assert!(selector.source_payload_bytes() >= selector.text().len() + source.len() * 2);
}
