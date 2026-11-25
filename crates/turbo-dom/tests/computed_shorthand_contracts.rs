//! Always-on computed formatter contracts; no optional cascade/layout feature.
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use turbo_dom::rtdom::css_shorthand::{
    computed_shorthand_value, expand_css, longhands, shorthand_value, Declaration,
};
fn render(name: &str, values: &BTreeMap<String, String>) -> String {
    let declarations: Vec<_> = values
        .iter()
        .map(|(name, value)| Declaration {
            name,
            value,
            important: false,
            pending: None,
        })
        .collect();
    computed_shorthand_value(name, &declarations)
}
#[test]
fn actual_chrome159_background_layers_and_computed_font_spelling_are_always_on() {
    let probe = include_bytes!("fixtures/computed-shorthand-159/computed-shorthand-probe.mjs");
    let control: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/computed-shorthand-159/COMPUTED-SHORTHAND-CHROME159.json"
    ))
    .unwrap();
    assert_eq!(control["browser"], "159.0.8154.6");
    assert_eq!(
        control["sourceSha256"],
        format!("{:x}", Sha256::digest(probe))
    );
    assert_eq!(control["result"].as_array().unwrap().len(), 7);
    for row in control["result"].as_array().unwrap() {
        let property = if row["id"].as_str().unwrap().starts_with("background-") {
            "background"
        } else {
            "font"
        };
        let mut values = if property == "font" {
            expand_css(
                "font",
                row["css"].as_str().unwrap().strip_prefix("font:").unwrap(),
            )
            .unwrap()
            .into_iter()
            .map(|value| (value.name.to_owned(), value.value))
            .collect()
        } else {
            BTreeMap::new()
        };
        for name in longhands(property).unwrap() {
            if let Some(value) = row["computed"][*name].as_str() {
                values.insert((*name).to_owned(), value.to_owned());
            }
        }
        assert_eq!(
            render(property, &values),
            row["computed"][property].as_str().unwrap(),
            "{}",
            row["id"]
        );
    }
}
#[test]
fn missing_unknown_and_unresolved_computed_components_do_not_invent_defaults() {
    assert_eq!(render("not-a-shorthand", &BTreeMap::new()), "");
    assert_eq!(
        render(
            "background",
            &BTreeMap::from([("background-color".into(), "red".into())])
        ),
        ""
    );
    let mut values: BTreeMap<_, _> = expand_css("background", "red")
        .unwrap()
        .into_iter()
        .map(|value| (value.name.to_owned(), value.value))
        .collect();
    assert_eq!(render("background", &values), "");
    values.insert("background-image".into(), String::new());
    assert_eq!(render("background", &values), "");
    let values = BTreeMap::from([
        ("margin-top".into(), "1px".into()),
        ("margin-right".into(), "2px".into()),
        ("margin-bottom".into(), "1px".into()),
        ("margin-left".into(), "2px".into()),
    ]);
    assert_eq!(render("margin", &values), "1px 2px");
    let declarations: Vec<_> = values
        .iter()
        .map(|(name, value)| Declaration {
            name,
            value,
            important: false,
            pending: None,
        })
        .collect();
    assert_eq!(
        computed_shorthand_value("margin", &declarations),
        shorthand_value("margin", &declarations)
    );
}
