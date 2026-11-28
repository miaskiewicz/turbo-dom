use super::*;
use std::cell::Cell;
fn original_keyword(value: &CssText) -> Option<WideKeyword> {
    let source = crate::rtdom::computed_css::CssSource::new(value.as_units());
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

fn text(value: &str) -> CssText {
    CssText::from_units(value.encode_utf16().collect())
}
fn same(units: Vec<u16>) {
    let value = CssText::from_units(units);
    assert_eq!(wide_keyword(&value), original_keyword(&value));
}
#[test]
fn whole_plain_ascii_keywords_avoid_tokenizer_setup() {
    let values = [
        text("initial"),
        text("INHERIT"),
        text("unset"),
        text("revert"),
        text("ReVeRt-LaYeR"),
        text("normal"),
        text("inheritx"),
    ];
    KEYWORD_TOKENIZATIONS.with(|count| count.set(0));
    for value in &values {
        assert_eq!(wide_keyword(value), original_keyword(value));
    }
    assert_eq!(KEYWORD_TOKENIZATIONS.with(Cell::get), 0);
}
#[test]
fn every_keyword_case_and_single_unit_mutation_matches_tokenizer() {
    for word in ["initial", "inherit", "unset", "revert", "revert-layer"] {
        let units: Vec<u16> = word.encode_utf16().collect();
        for mask in 0..(1_u32 << units.len()) {
            let varied = units
                .iter()
                .enumerate()
                .map(|(index, unit)| {
                    if mask & (1 << index) != 0 && *unit != 45 {
                        unit - 32
                    } else {
                        *unit
                    }
                })
                .collect();
            same(varied);
        }
        for index in 0..units.len() {
            for replacement in [
                0, 9, 10, 12, 13, 32, 45, 48, 65, 90, 92, 95, 97, 122, 127, 233, 0xd800, 0xdc00,
                0xffff,
            ] {
                let mut altered = units.clone();
                altered[index] = replacement;
                same(altered);
            }
        }
    }
}
#[test]
fn escapes_comments_spacing_and_non_ascii_keep_shared_parser_fallback() {
    let cases = [
        "",
        "-",
        "--",
        "--initial",
        "_initial",
        " initial",
        "initial ",
        "/*a*/initial",
        "initial/**/",
        "in/**/itial",
        "initial inherit",
        "initial;",
        "initial()",
        "url(initial)",
        "0",
        "1px",
        "\\69 nitial",
        "\\000069nitial",
        "i\\nherit",
        "\\49 NITIAL",
        "revert\\2d layer",
        "initial\r\n",
        "\u{000c}initial",
        "initial\0",
        "éinitial",
        "🙂",
        "auto",
        "revert--layer",
    ];
    for case in cases {
        same(case.encode_utf16().collect());
    }
    for units in [
        vec![0xd800],
        vec![0xdc00],
        vec![0xd800, 0xdc00],
        vec![105, 110, 0xd800, 105, 116, 105, 97, 108],
    ] {
        same(units);
    }
    KEYWORD_TOKENIZATIONS.with(|count| count.set(0));
    for case in [
        "",
        "-",
        "--",
        " initial",
        "initial ",
        "in/**/itial",
        "\\69 nitial",
        "0",
        "éinitial",
    ] {
        let _ = wide_keyword(&text(case));
    }
    assert_eq!(KEYWORD_TOKENIZATIONS.with(Cell::get), 9);
}
#[test]
fn generated_utf16_sequences_preserve_keyword_results() {
    let mut state = 0x1234_5678_u32;
    for length in 0..24 {
        for _ in 0..64 {
            let mut units = Vec::new();
            for _ in 0..length {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                units.push(u16::try_from(state & 0xffff).unwrap());
            }
            same(units);
        }
    }
}
