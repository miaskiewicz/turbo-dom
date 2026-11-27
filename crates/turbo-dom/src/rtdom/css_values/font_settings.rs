//! Shared font-tag grammar; legacy descriptor callers retain their optional-value mode.
use super::css_number;
use cssparser::Parser;
use std::fmt::Write;
/// Parse native font feature/variation settings with an explicit variation-value policy.
/// Returns None for invalid tags/values; caller checks exhaustion of the complete value.
pub fn font_settings(
    feature: bool,
    require_variation_value: bool,
    input: &mut Parser<'_, '_>,
) -> Option<String> {
    if input
        .try_parse(|input| input.expect_ident_matching("normal"))
        .is_ok()
    {
        return Some("normal".to_owned());
    }
    let settings = input
        .parse_comma_separated(|input| {
            let location = input.current_source_location();
            let invalid = || location.new_custom_error::<(), ()>(());
            let tag = input.expect_string_cloned()?;
            if tag.chars().count() != 4 || !tag.chars().all(|c| (' '..='~').contains(&c)) {
                return Err(invalid());
            }
            let mut text = String::new();
            cssparser::serialize_string(&tag, &mut text).map_err(|_| invalid())?;
            if feature {
                let value = feature_value(input);
                if value != 1 {
                    let _ = write!(text, " {value}");
                }
            } else {
                let value = if require_variation_value {
                    input.expect_number()?
                } else {
                    input.try_parse(Parser::expect_number).unwrap_or(1.0)
                };
                let _ = write!(text, " {}", css_number(value));
            }
            Ok(text)
        })
        .ok()?;
    Some(settings.join(", "))
}

fn feature_value(input: &mut Parser<'_, '_>) -> i32 {
    if input
        .try_parse(|input| input.expect_ident_matching("on"))
        .is_ok()
    {
        1
    } else if input
        .try_parse(|input| input.expect_ident_matching("off"))
        .is_ok()
    {
        0
    } else {
        input.try_parse(Parser::expect_integer).unwrap_or(1)
    }
}
