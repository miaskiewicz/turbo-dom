//! Context-independent property metadata shared with the preserved raw core cascade.
use super::CssText;

pub struct InitialContext<'a> {
    pub color: &'a str,
    pub font_family: &'a str,
    pub font_size: &'a str,
}

#[must_use]
pub fn inherited(name: &str) -> bool {
    super::property_data::lookup(name).map_or_else(
        || crate::rtdom::cascade::INHERITED.contains(&name),
        |entry| entry.inherited,
    )
}
#[must_use]
pub fn length_property(name: &str) -> bool {
    crate::rtdom::cascade::LENGTH_PROPS.contains(&name)
}

/// Only explicit known initial values are returned. This does not materialize a
/// guessed complete UA style; unknown initial/used values require a provider.
#[must_use]
pub fn initial(name: &str, context: &InitialContext<'_>) -> Option<CssText> {
    let value = match name {
        "color" => context.color,
        "font-family" => context.font_family,
        "font-size" => context.font_size,
        _ => return context_free_initial(name),
    };
    Some(CssText::from_units(value.encode_utf16().collect()))
}
/// Known context-independent initial tokens. Font/color preferences remain with
/// the explicit `InitialContext` caller; no UA map is materialized here.
#[must_use]
pub fn context_free_initial(name: &str) -> Option<CssText> {
    let value = super::property_data::lookup(name)
        .map(|entry| entry.initial)
        .or_else(|| context_independent_initial(name))?;
    Some(CssText::from_units(value.encode_utf16().collect()))
}
// The existing native defaults table leaves these to its separate font/pseudo
// context code. Keep their CSS initial values explicit rather than guessing a UA map.
fn context_independent_initial(name: &str) -> Option<&'static str> {
    match name {
        "font-style" | "font-variant" | "line-height" | "white-space" | "content"
        | "container-type" => Some("normal"),
        "font-weight" => Some("400"),
        "display" => Some("inline"),
        "position" => Some("static"),
        "width" | "height" => Some("auto"),
        "container-name" => Some("none"),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WideKeyword {
    Initial,
    Inherit,
    Unset,
    Revert,
    RevertLayer,
}

/// Keyword recognition uses the shared CSS tokenizer, so escapes/comments and
/// whitespace cannot create a separate string-based keyword policy.
#[must_use]
pub fn wide_keyword(value: &CssText) -> Option<WideKeyword> {
    let source = super::CssSource::new(value.as_units());
    let mut parser_input = cssparser::ParserInput::new(source.scalar());
    let mut parser = cssparser::Parser::new(&mut parser_input);
    let ident = parser.expect_ident_cloned().ok()?;
    parser.expect_exhausted().ok()?;
    match ident.to_ascii_lowercase().as_str() {
        "initial" => Some(WideKeyword::Initial),
        "inherit" => Some(WideKeyword::Inherit),
        "unset" => Some(WideKeyword::Unset),
        "revert" => Some(WideKeyword::Revert),
        "revert-layer" => Some(WideKeyword::RevertLayer),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_initial_preferences_and_legacy_inheritance_are_shared_without_default_map() {
        let context = InitialContext {
            color: "rgb(0, 0, 0)",
            font_family: "serif",
            font_size: "16px",
        };
        assert!(inherited("color"));
        assert!(!inherited("margin-left"));
        assert!(length_property("margin-left"));
        assert!(!length_property("opacity"));
        assert_eq!(
            String::from_utf16(initial("color", &context).unwrap().as_units()).unwrap(),
            "rgb(0, 0, 0)"
        );
        assert_eq!(
            String::from_utf16(initial("margin-left", &context).unwrap().as_units()).unwrap(),
            "0px"
        );
        assert!(initial("invented-property", &context).is_none());
        assert_eq!(
            String::from_utf16(initial("width", &context).unwrap().as_units()).unwrap(),
            "auto"
        );
    }
}
