#![cfg(feature = "computed-style-grammar")]
use cssparser::{Parser, ParserInput};
use turbo_dom::rtdom::{
    css_shorthand::{longhand_value_css, Spelled},
    css_values::font_settings,
};
fn setting(value: &str, feature: bool, strict: bool) -> Option<String> {
    let mut source = ParserInput::new(value);
    let mut input = Parser::new(&mut source);
    let result = font_settings(feature, strict, &mut input)?;
    input.expect_exhausted().ok()?;
    Some(result)
}
#[test]
fn font_tags_use_same_shared_parser_for_legacy_and_strict_variations() {
    assert_eq!(setting("\"kern\" on", true, true), Some("\"kern\"".into()));
    assert_eq!(
        setting("\"kern\" off", true, true),
        Some("\"kern\" 0".into())
    );
    assert_eq!(setting("\"wght\"", false, false), Some("\"wght\" 1".into()));
    assert_eq!(setting("\"wght\"", false, true), None);
    for value in ["\"abc\" 1", "\"abcde\" 1", "\"aébc\" 1", "\"a\\1 bc\" 1"] {
        assert_eq!(setting(value, true, true), None, "{value}");
    }
}
#[test]
fn font_metric_and_quote_grammar_validates_context_free_values() {
    assert!(matches!(
        longhand_value_css("font-size-adjust", "cap-height 0.5"),
        Spelled::Valid(_)
    ));
    for value in ["cap-height", "unknown 0.5", "none from-font"] {
        assert_eq!(
            longhand_value_css("font-size-adjust", value),
            Spelled::Invalid
        );
    }
    for value in ["auto", "none"] {
        assert_eq!(
            longhand_value_css("quotes", value),
            Spelled::Valid(value.into())
        );
    }
}
