#![cfg(feature = "computed-style-grammar")]
use turbo_dom::rtdom::computed_css::property_registry;
#[test]
fn generated_positive_keywords_retain_exact_measured_metadata() {
    let source: serde_json::Value = serde_json::from_str(property_registry::REGISTRY_JSON).unwrap();
    for (name, row) in source["properties"].as_object().unwrap() {
        let expected = row
            .get("keywords")
            .and_then(serde_json::Value::as_array)
            .filter(|rows| !rows.is_empty())
            .map(|rows| {
                rows.iter()
                    .map(|value| value.as_str().unwrap())
                    .collect::<Vec<_>>()
            });
        assert_eq!(
            property_registry::keywords(name),
            expected.as_deref(),
            "{name}"
        );
    }
    assert_eq!(property_registry::keywords("future-property"), None);
}
