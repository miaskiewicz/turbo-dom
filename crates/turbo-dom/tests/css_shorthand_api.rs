//! `rtdom::css_shorthand`'s table queries and serializer contracts beyond the Chrome 156
//! replay in `css_shorthand_chrome.rs`: `is_shorthand` / `shorthands_of` against the control's
//! shorthand table, `shorthand_value` of a longhand, and values an embedder admits for a
//! longhand that Chrome's own grammar never produces.
use serde_json::Value;
use turbo_dom::rtdom::css_shorthand::{
    canonical, is_shorthand, longhand_value, longhands, shorthand_value, shorthands_of,
    Declaration, DeclarationBlock, Spelled,
};

const CONTROL: &str = include_str!("fixtures/css_shorthand_chrome156.json");
const CASES: &str = include_str!("fixtures/css-shorthand-cases.json");

fn table() -> serde_json::Map<String, Value> {
    let control: Value = serde_json::from_str(CONTROL).unwrap();
    control["result"][0]["result"]["table"]
        .as_object()
        .unwrap()
        .clone()
}

/// The names the probe measured: Chrome 156's shorthands, their aliases and longhand aliases.
fn probed_names() -> Vec<String> {
    let cases: Value = serde_json::from_str(CASES).unwrap();
    let html = cases[0]["html"].as_str().unwrap();
    let start = html.find("id=\"names\">").unwrap() + "id=\"names\">".len();
    let end = html[start..].find("</script>").unwrap() + start;
    serde_json::from_str(&html[start..end]).unwrap()
}

#[test]
fn is_shorthand_is_chromes_table() {
    let table = table();
    let names = probed_names();
    assert_eq!(names.len(), 230);
    let mut shorthands = 0;
    for name in &names {
        assert_eq!(is_shorthand(name), table.contains_key(name), "{name}");
        shorthands += usize::from(is_shorthand(name));
    }
    assert_eq!(shorthands, table.len());
    assert_eq!(shorthands, 129);
    for name in ["color", "margin-top", "word-wrap", "", "MARGIN"] {
        assert!(!is_shorthand(name), "{name}");
    }
}

#[test]
fn shorthands_of_lists_the_canonical_shorthands_most_longhands_first() {
    let table = table();
    let mut longhand_names: Vec<&str> = table
        .values()
        .flat_map(|list| list.as_array().unwrap().iter().map(|v| v.as_str().unwrap()))
        .collect();
    longhand_names.sort_unstable();
    longhand_names.dedup();
    for longhand in longhand_names {
        let ours: Vec<&str> = shorthands_of(longhand).collect();
        // Chrome's shorthands holding the longhand, aliases (`-webkit-flex`) left out.
        let mut expected: Vec<&str> = table
            .iter()
            .filter(|(name, list)| {
                canonical(name) == Some(name.as_str())
                    && list.as_array().unwrap().iter().any(|v| v == longhand)
            })
            .map(|(name, _)| name.as_str())
            .collect();
        expected.sort_by_key(|name| (std::cmp::Reverse(longhands(name).unwrap().len()), *name));
        assert_eq!(ours, expected, "{longhand}");
    }
    assert_eq!(
        shorthands_of("border-top-width").collect::<Vec<_>>(),
        ["border", "border-width", "border-top"]
    );
    assert_eq!(shorthands_of("color").count(), 0);
}

fn declaration<'a>(name: &'a str, value: &'a str) -> Declaration<'a> {
    Declaration {
        name,
        value,
        important: false,
        pending: None,
    }
}

#[test]
fn shorthand_value_of_a_longhand_is_empty() {
    let sides = ["margin-top", "margin-right", "margin-bottom", "margin-left"];
    let block: Vec<Declaration<'_>> = sides.iter().map(|name| declaration(name, "1px")).collect();
    assert_eq!(shorthand_value("margin", &block), "1px");
    assert_eq!(shorthand_value("margin-top", &block), "");
    assert_eq!(shorthand_value("color", &[declaration("color", "red")]), "");
}

/// An embedder that admits every longhand value as written (a `LonghandSpelling`).
#[allow(clippy::unnecessary_wraps)]
fn admit(_: &str, value: &str) -> Option<String> {
    Some(value.trim().to_string())
}

#[test]
fn font_synthesis_serializes_only_auto_and_none() {
    let mut block = DeclarationBlock::new();
    for (name, value) in [
        ("font-synthesis-weight", "auto"),
        ("font-synthesis-style", "none"),
        ("font-synthesis-small-caps", "auto"),
    ] {
        block.set_property(name, value, "", &admit);
    }
    assert_eq!(block.property_value("font-synthesis"), "weight small-caps");
    block.set_property("font-synthesis-style", "bogus", "", &admit);
    assert_eq!(block.property_value("font-synthesis"), "");
    assert_eq!(
        block.css_text(),
        "font-synthesis-weight: auto; font-synthesis-style: bogus; font-synthesis-small-caps: auto;"
    );
}

#[test]
fn longhand_value_of_whitespace_is_invalid() {
    // `setProperty(name, "  ")` removes the property in Chrome: there is no value to spell.
    for value in ["", "  ", "\t\n"] {
        assert_eq!(
            longhand_value("width", value),
            Spelled::Invalid,
            "{value:?}"
        );
    }
    assert_eq!(
        longhand_value("width", " 1PX "),
        Spelled::Valid("1px".into())
    );
}

#[test]
fn background_reads_an_empty_image_layer_as_none() {
    // Chrome's grammar has no empty layer; an embedder that admits one gets `none` for it.
    let mut block = DeclarationBlock::new();
    block.set_property("background", "initial", "", &admit);
    block.set_property("background-image", "url(a.png), , url(b.png)", "", &admit);
    assert_eq!(
        block.property_value("background"),
        "url(a.png), none, url(b.png)"
    );
}

#[test]
fn display_knows_every_two_keyword_form() {
    // `display` is lenient (its grammar leaves unknown forms to the embedder); the
    // two-keyword forms Chrome 156 admits are this grammar's own, spelled like Chrome
    // (`css_longhand_chrome156.json`).
    for (value, chrome) in [
        ("block flow", "block"),
        ("inline flow", "inline"),
        ("flow block", "block"),
        ("inline flow-root", "inline-block"),
        ("flow-root inline", "inline-block"),
        ("block flow-root", "flow-root"),
        ("flex block", "flex"),
        ("inline flex", "inline-flex"),
        ("block grid", "grid"),
        ("inline grid", "inline-grid"),
        ("block table", "table"),
        ("table inline", "inline-table"),
        ("block ruby", "block ruby"),
        ("ruby block", "block ruby"),
        ("inline ruby", "ruby"),
        ("ruby inline", "ruby"),
    ] {
        assert_eq!(
            longhand_value("display", value),
            Spelled::Valid(chrome.into()),
            "{value}"
        );
    }
    assert_eq!(
        longhand_value("display", "flow-root flex"),
        Spelled::Unchecked("flow-root flex".into())
    );
}
