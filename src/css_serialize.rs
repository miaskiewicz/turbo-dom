//! CSSOM projection consumes already finalized ordinary values, never DOM facts.
use super::css_backend::ComputationPolicy;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use turbo_dom_core::rtdom::computed_css::{
    cssom_serialization::{
        serialize_cssom_with_context, CssomSerializationContext, CssomSerializationLimits,
        SelectedTextDecoration, SerializedCssom, TextDecorationSerialization,
    },
    prepared::CascadeOrigin,
    CssText,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Request {
    abi_version: u32,
    targets: Vec<Target>,
    text_decoration: DecorationPolicy,
    #[serde(default)]
    include_used_properties: bool,
    limits: Limits,
}
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
enum DecorationPolicy {
    Computed,
    Declared,
}
impl From<DecorationPolicy> for TextDecorationSerialization {
    fn from(value: DecorationPolicy) -> Self {
        match value {
            DecorationPolicy::Computed => Self::Computed,
            DecorationPolicy::Declared => Self::Declared,
        }
    }
}
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum MarkerOrigin {
    UserAgent,
    User,
    Author,
}
impl MarkerOrigin {
    fn name(self) -> &'static str {
        match self {
            Self::UserAgent => "user-agent",
            Self::User => "user",
            Self::Author => "author",
        }
    }
}
impl From<MarkerOrigin> for CascadeOrigin {
    fn from(value: MarkerOrigin) -> Self {
        match value {
            MarkerOrigin::UserAgent => Self::UserAgent,
            MarkerOrigin::User => Self::User,
            MarkerOrigin::Author => Self::Author,
        }
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Marker {
    value: Vec<u16>,
    origin: MarkerOrigin,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Target {
    id: u32,
    values: Vec<(String, Vec<u16>)>,
    public_keys: Vec<String>,
    font_size_px: Option<f64>,
    declared_text_decoration: Option<Marker>,
}
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Limits {
    targets: usize,
    input_units: usize,
    retained_units: usize,
    computation: ComputationPolicy,
}
struct Units(CssText);
impl Serialize for Units {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.as_units().serialize(serializer)
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Row {
    id: u32,
    values: Vec<(String, Units)>,
    public_keys: BTreeSet<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    used_properties: Option<Vec<String>>,
}
fn fail(reason: &str) -> String {
    format!("CSSBackendSerialize:{reason}")
}
fn charge(remaining: &mut usize, amount: usize) -> Result<(), String> {
    super::css_input::charge(remaining, amount).map_err(|error| fail(&format!("{error:?}")))
}
fn ordinary(
    source: Vec<(String, Vec<u16>)>,
    input: &mut usize,
) -> Result<BTreeMap<String, CssText>, String> {
    super::css_input::ordinary(source, input).map_err(|error| fail(&format!("{error:?}")))
}
fn provenance(source: Vec<String>, input: &mut usize) -> Result<BTreeSet<String>, String> {
    super::css_input::provenance(source, input).map_err(|error| fail(&format!("{error:?}")))
}
fn marker(
    source: Option<Marker>,
    input: &mut usize,
) -> Result<Option<(CssText, CascadeOrigin)>, String> {
    source
        .map(|marker| {
            charge(input, marker.value.len())?;
            charge(input, marker.origin.name().len())?;
            Ok((CssText::from_units(marker.value), marker.origin.into()))
        })
        .transpose()
}
fn charge_result(serialized: &SerializedCssom, retained: &mut usize) -> Result<(), String> {
    for (name, value) in &serialized.values {
        charge(retained, name.encode_utf16().count())?;
        charge(retained, value.as_units().len())?;
    }
    for name in &serialized.public_keys {
        charge(retained, name.encode_utf16().count())?;
    }
    Ok(())
}
fn row(
    target: Target,
    input: &mut usize,
    retained: &mut usize,
    limits: Limits,
    decoration: DecorationPolicy,
    include_used: bool,
) -> Result<Row, String> {
    let marker = marker(target.declared_text_decoration, input)?;
    let values = ordinary(target.values, input)?;
    let public = provenance(target.public_keys, input)?;
    projected_row(
        target.id,
        &values,
        &public,
        &CssomSerializationContext {
            text_decoration: decoration.into(),
            declared_text_decoration: marker.as_ref().map(|(value, origin)| {
                SelectedTextDecoration {
                    value,
                    origin: *origin,
                }
            }),
            font_size_px: target.font_size_px,
        },
        limits,
        retained,
        include_used,
    )
}
fn projected_row(
    id: u32,
    values: &BTreeMap<String, CssText>,
    public: &BTreeSet<String>,
    context: &CssomSerializationContext<'_>,
    limits: Limits,
    retained: &mut usize,
    include_used: bool,
) -> Result<Row, String> {
    let serialized = serialize_cssom_with_context(
        values,
        public,
        context,
        CssomSerializationLimits {
            retained_units: *retained,
            computation: limits.computation.into(),
        },
    )
    .map_err(|error| fail(&format!("{error:?}")))?;
    charge_result(&serialized, retained)?;
    let used_properties = include_used
        .then(|| used_properties(&serialized, retained))
        .transpose()?;
    Ok(Row {
        id,
        values: serialized
            .values
            .into_iter()
            .filter(|(name, _)| serialized.public_keys.contains(name))
            .map(|(name, value)| (name, Units(value)))
            .collect(),
        public_keys: serialized.public_keys,
        used_properties,
    })
}
fn used_properties(
    serialized: &SerializedCssom,
    retained: &mut usize,
) -> Result<Vec<String>, String> {
    let mut used = Vec::new();
    for name in &serialized.public_keys {
        if serialized.values.contains_key(name)
            && turbo_dom_core::rtdom::computed_css::used_values::property_axis(name).is_some()
        {
            charge(retained, name.encode_utf16().count())?;
            used.push(name.clone());
        }
    }
    Ok(used)
}
pub(crate) fn serialize_batch(source: &str) -> Result<String, String> {
    let request: Request =
        serde_json::from_str(source).map_err(|error| fail(&error.to_string()))?;
    if request.abi_version != 1 {
        return Err(fail("AbiVersion"));
    }
    if request.targets.len() > request.limits.targets {
        return Err(fail("ResourceLimit"));
    }
    let mut ids = BTreeSet::new();
    let mut input = request.limits.input_units;
    let mut retained = request.limits.retained_units;
    let mut targets = Vec::with_capacity(request.targets.len());
    for target in request.targets {
        if !ids.insert(target.id) {
            return Err(fail("DuplicateTarget"));
        }
        targets.push(row(
            target,
            &mut input,
            &mut retained,
            request.limits,
            request.text_decoration,
            request.include_used_properties,
        )?);
    }
    serde_json::to_string(&serde_json::json!({"abiVersion":1,"targets":targets}))
        .map_err(|error| fail(&error.to_string()))
}

#[path = "css_native_projection.rs"]
mod native_projection;
#[cfg(test)]
pub(crate) use native_projection::cssom_wire as state_cssom_wire;
#[cfg(test)]
pub(crate) use native_projection::project_state;
#[cfg(test)]
pub(crate) use native_projection::take_projection_calls;
pub(crate) use native_projection::{project_validated_state, validate_projection};

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn target(id: u32) -> Value {
        json!({"id":id,"values":[["color",[114,101,100]]],"publicKeys":["color"]})
    }
    fn request(targets: Vec<Value>) -> Value {
        json!({"abiVersion":1,"targets":targets,"textDecoration":"declared","limits":{"targets":8,"inputUnits":4096,"retainedUnits":4096,"computation":{"maxNesting":32,"maxOutputUnits":4096,"maxTotalUnits":8192}}})
    }
    fn run(request: &Value) -> Result<Value, String> {
        serde_json::from_str(&serialize_batch(&request.to_string())?)
            .map_err(|error| error.to_string())
    }
    #[test]
    fn only_explicit_public_provenance_derives_shorthands_and_aliases() {
        let values: Vec<_> = ["top", "right", "bottom", "left"]
            .map(|edge| json!([format!("margin-{edge}"), [49, 112, 120]]))
            .into();
        let keys = ["margin-top", "margin-right", "margin-bottom", "margin-left"];
        let input = request(vec![json!({"id":1,"values":values,"publicKeys":keys})]);
        let result = run(&input).unwrap();
        assert!(result["targets"][0]["publicKeys"]
            .as_array()
            .unwrap()
            .contains(&json!("margin")));
        let mut ua_only = input;
        ua_only["targets"][0]["publicKeys"] = json!([]);
        assert_eq!(
            run(&ua_only).unwrap()["targets"][0]["publicKeys"],
            json!([])
        );
    }
    #[test]
    fn bulk_quotas_are_cumulative_and_exact_for_input_and_retained_payloads() {
        let mut input = request(vec![target(1), target(2)]);
        // Each row retains and accepts color's five-unit key twice and red once.
        input["limits"]["inputUnits"] = json!(26);
        input["limits"]["retainedUnits"] = json!(26);
        assert_eq!(run(&input).unwrap()["targets"].as_array().unwrap().len(), 2);
        for quota in ["inputUnits", "retainedUnits"] {
            let mut short = input.clone();
            short["limits"][quota] = json!(25);
            assert!(run(&short).unwrap_err().contains("ResourceLimit"));
        }
    }
    #[test]
    fn ambiguous_transport_is_rejected_instead_of_last_wins() {
        let duplicates = [
            (request(vec![target(1), target(1)]), "DuplicateTarget"),
            (
                request(vec![
                    json!({"id":1,"values":[["color",[]],["color",[]]],"publicKeys":[]}),
                ]),
                "DuplicateProperty",
            ),
            (
                request(vec![
                    json!({"id":1,"values":[],"publicKeys":["color","color"]}),
                ]),
                "DuplicatePublicKey",
            ),
        ];
        for (input, error) in duplicates {
            assert!(run(&input).unwrap_err().contains(error));
        }
        let mut input = request(vec![]);
        input["abiVersion"] = json!(2);
        assert!(run(&input).unwrap_err().contains("AbiVersion"));
        input = request(vec![target(1)]);
        input["limits"]["targets"] = json!(0);
        assert!(run(&input).unwrap_err().contains("ResourceLimit"));
    }
    #[test]
    fn declared_marker_origin_and_raw_units_cross_only_the_selected_policy() {
        let mut units = "var(--missing, '".encode_utf16().collect::<Vec<_>>();
        units.push(0xd800);
        units.extend("')".encode_utf16());
        let marker = json!({"value":units,"origin":"author"});
        let mut input = request(vec![
            json!({"id":1,"values":[],"publicKeys":[],"declaredTextDecoration":marker}),
        ]);
        assert_eq!(
            run(&input).unwrap()["targets"][0]["values"],
            json!([["text-decoration", units]])
        );
        input["targets"][0]["declaredTextDecoration"]["origin"] = json!("user-agent");
        assert_eq!(run(&input).unwrap()["targets"][0]["values"], json!([]));
        input["targets"][0]["declaredTextDecoration"]["origin"] = json!("user");
        assert_eq!(
            run(&input).unwrap()["targets"][0]["values"],
            json!([["text-decoration", units]])
        );
        input["textDecoration"] = json!("computed");
        assert_eq!(run(&input).unwrap()["targets"][0]["values"], json!([]));
    }
    #[test]
    fn actual_font_scalar_projects_numeric_line_height_and_hidden_values_stay_private() {
        let mut input = request(vec![
            json!({"id":1,"values":[["line-height",[49,46,53]],["font-weight",[52,48,48]]],"publicKeys":["line-height"],"fontSizePx":20.0}),
        ]);
        let output = run(&input).unwrap();
        assert_eq!(
            output["targets"][0]["values"],
            json!([["line-height", [51, 48, 112, 120]]])
        );
        input["targets"][0]["fontSizePx"] = Value::Null;
        assert!(run(&input).unwrap_err().contains("RequiresFontSize"));
    }
    #[test]
    fn marker_transport_quotas_cover_input_and_owned_declared_output_exactly() {
        let marker = json!({"value":[118,97,114,40,45,45,100,41],"origin":"author"});
        let mut input = request(vec![
            json!({"id":1,"values":[],"publicKeys":[],"declaredTextDecoration":marker}),
        ]);
        input["limits"]["inputUnits"] = json!(14);
        input["limits"]["retainedUnits"] = json!(38);
        assert_eq!(
            run(&input).unwrap()["targets"][0]["publicKeys"],
            json!(["text-decoration"])
        );
        for (name, value) in [("inputUnits", 13), ("retainedUnits", 37)] {
            let mut short = input.clone();
            short["limits"][name] = json!(value);
            assert!(run(&short).unwrap_err().contains("ResourceLimit"));
        }
    }
    #[test]
    fn ordinary_raw_units_survive_and_custom_maps_cannot_cross_this_boundary() {
        let input = request(vec![
            json!({"id":1,"values":[["unknown-property",[55296,0,57343]]],"publicKeys":["unknown-property"]}),
        ]);
        assert_eq!(
            run(&input).unwrap()["targets"][0]["values"][0][1],
            json!([55296, 0, 57343])
        );
        let custom = request(vec![
            json!({"id":1,"values":[["--Case",[]]],"publicKeys":[]}),
        ]);
        assert!(run(&custom)
            .unwrap_err()
            .contains("CustomPropertyInOrdinary"));
        let mut unknown = request(vec![]);
        unknown["geometryOnly"] = json!(true);
        assert!(run(&unknown).is_err());
    }
    #[test]
    fn optional_used_property_metadata_is_canonical_and_owns_its_budget() {
        let mut input = request(vec![
            json!({"id":1,"values":[["width",[53,48,37]]],"publicKeys":["width"]}),
        ]);
        assert!(run(&input).unwrap()["targets"][0]
            .get("usedProperties")
            .is_none());
        input["includeUsedProperties"] = json!(true);
        let result = run(&input).unwrap();
        assert_eq!(result["targets"][0]["usedProperties"], json!(["width"]));
        assert_eq!(
            result["targets"][0]["values"],
            json!([["width", [53, 48, 37]]])
        );
        input["limits"]["retainedUnits"] = json!(18);
        assert!(run(&input).is_ok());
        input["limits"]["retainedUnits"] = json!(17);
        assert!(run(&input).unwrap_err().contains("ResourceLimit"));
        input["limits"]["retainedUnits"] = json!(64);
        input["targets"][0]["values"] = json!([]);
        assert_eq!(
            run(&input).unwrap()["targets"][0]["usedProperties"],
            json!([])
        );
    }
}
