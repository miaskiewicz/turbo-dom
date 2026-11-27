use turbo_dom::rtdom::computed_css::{declarations, CssSource, CssText};

fn units(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

#[test]
fn raw_css_tokens_preserve_lone_units_without_marker_collisions() {
    let mut input = units("--Token:\"");
    input.push(0xd800);
    input.extend(units(
        "\";--literal:'\\d800 ';--marker:'\u{f0000}\u{f0800}';--empty:;",
    ));
    let parsed = declarations(&input);
    assert_eq!(parsed.len(), 4);
    assert_eq!(parsed[0].name, "--Token");
    assert_eq!(parsed[0].value.as_units(), [34, 0xd800, 34]);
    assert_eq!(parsed[1].value.as_units(), units("'\\d800 '"));
    assert_eq!(parsed[2].value.as_units(), units("'\u{f0000}\u{f0800}'"));
    assert!(
        parsed[3].value.as_units().is_empty(),
        "valid empty is not a missing declaration"
    );
}

#[test]
fn canonical_declaration_parser_keeps_quoted_delimiters_and_importance() {
    let input=units("color:red!important; content:'var(--x);!important';--Case:url(\"data:a;b:c\");broken; width:2em");
    let parsed = declarations(&input);
    assert_eq!(parsed.len(), 4);
    assert!(parsed[0].important);
    assert!(!parsed[1].important);
    assert_eq!(parsed[1].value.as_units(), units("'var(--x);!important'"));
    assert_eq!(parsed[2].name, "--Case");
    assert_eq!(parsed[2].value.as_units(), units("url(\"data:a;b:c\")"));
    assert_eq!(parsed[3].value.as_units(), units("2em"));
}

#[test]
fn source_ranges_map_scalar_offsets_back_to_original_units() {
    let raw = [0x61, 0xd800, 0xd83d, 0xde00, 0xe9];
    let source = CssSource::new(&raw);
    assert_eq!(source.scalar(), "a\u{fffd}😀é");
    assert_eq!(source.raw_range(1..4).unwrap().as_units(), [0xd800]);
    assert_eq!(source.raw_range(4..8).unwrap().as_units(), [0xd83d, 0xde00]);
    assert!(source.raw_range(2..4).is_none());
    assert!(source.raw_range(0..100).is_none());
    let reversed_start = 8;
    assert!(source.raw_range(reversed_start..4).is_none());
}

#[test]
fn custom_tokens_trim_css_whitespace_without_erasing_unicode_content() {
    let parsed = declarations(&units("--Space:  \t ;--NBSP: \u{a0} ;--text: x "));
    assert_eq!(parsed[0].value.as_units(), []);
    assert_eq!(parsed[1].value.as_units(), [0xa0]);
    assert_eq!(parsed[2].value.as_units(), units("x"));
}

#[test]
fn css_identifier_normalization_is_separate_from_raw_value_transport() {
    for suffix in [
        vec![0xd800],
        vec![0xdc00],
        vec![0],
        units("\\d800 "),
        units("\\fffd "),
    ] {
        let mut source = units("--a");
        source.extend(suffix);
        source.extend(units(":red"));
        let parsed = declarations(&source);
        assert_eq!(parsed[0].name, "--a\u{fffd}");
        assert_eq!(parsed[0].value.as_units(), units("red"));
    }
    let parsed = declarations(&units("--a\u{f0000}\u{f0800}:blue"));
    assert_eq!(parsed[0].name, "--a\u{f0000}\u{f0800}");
}

#[test]
fn raw_custom_payloads_preserve_nul_crlf_and_escaped_newline() {
    for payload in [
        vec![0],
        units("a\r\nb"),
        units("\"a\\\r\nb\""),
        vec![0xa0],
        vec![0x85],
        vec![0x2003],
    ] {
        let mut source = units("--Token: ");
        source.extend(&payload);
        source.extend(units(" ;"));
        let parsed = declarations(&source);
        assert_eq!(parsed[0].value.as_units(), payload);
    }
}

#[test]
fn recovered_payloads_own_their_units_after_source_storage_is_dropped() {
    let mut input = vec![0x61, 0xd800, 0x62];
    let text = CssText::from_units(input.clone());
    let recovered = {
        let source = CssSource::new(&input);
        input.clear();
        source.raw_range(1..4).unwrap()
    };
    assert_eq!(text.as_units(), [0x61, 0xd800, 0x62]);
    assert_eq!(recovered.as_units(), [0xd800]);
    assert_eq!(input, []);
}
