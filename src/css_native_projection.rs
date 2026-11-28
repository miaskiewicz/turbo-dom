use super::{
    charge, fail, projected_row, BTreeMap, CssText, CssomSerializationContext, DecorationPolicy,
    Deserialize, Limits, Row, SelectedTextDecoration,
};
use crate::css_resolve::native_state::NativeStyleState;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Projection {
    abi_version: u32,
    target_id: u32,
    text_decoration: DecorationPolicy,
    include_used_properties: bool,
    limits: Limits,
}
fn input_charge(
    style: &turbo_dom_core::rtdom::computed_css::resolve::ComputedStyle,
    values: &BTreeMap<String, CssText>,
    remaining: &mut usize,
) -> Result<(), String> {
    if let Some(marker) = &style.declared_text_decoration {
        charge(remaining, marker.value.as_units().len())?;
        let origin = crate::css_resolve::origin_name(marker.origin);
        charge(remaining, origin.len())?;
    }
    for (name, value) in values {
        charge(remaining, name.encode_utf16().count())?;
        charge(remaining, value.as_units().len())?;
    }
    for name in &style.public_keys {
        charge(remaining, name.encode_utf16().count())?;
    }
    Ok(())
}
fn projection_row(state: &NativeStyleState, options: &Projection) -> Result<Row, String> {
    let style = state
        .styles
        .get(&options.target_id)
        .ok_or_else(|| fail("StyleStateTargetMissing"))?;
    if options.limits.targets < 1 {
        return Err(fail("ResourceLimit"));
    }
    let values = state
        .renderer(options.target_id)
        .ok_or_else(|| fail("StyleStateTargetMissing"))?;
    let mut input = options.limits.input_units;
    input_charge(style, &values, &mut input)?;
    let font = style.values.font_size_px;
    let context = CssomSerializationContext {
        text_decoration: options.text_decoration.into(),
        declared_text_decoration: style.declared_text_decoration.as_ref().map(|marker| {
            SelectedTextDecoration {
                value: &marker.value,
                origin: marker.origin,
            }
        }),
        font_size_px: font.is_finite().then_some(font),
    };
    let mut retained = options.limits.retained_units;
    projected_row(
        options.target_id,
        &values,
        &style.public_keys,
        &context,
        options.limits,
        &mut retained,
        true,
    )
}
fn public_values(row: Row) -> (Value, Vec<String>) {
    let values: BTreeMap<_, _> = row.values.into_iter().collect();
    (json!(values), row.used_properties.unwrap_or_default())
}
fn minimal(state: &NativeStyleState, id: u32, values: Value, used: Vec<String>) -> Value {
    let custom: BTreeMap<_, _> = state.styles[&id]
        .custom
        .as_map()
        .iter()
        .map(|(name, value)| (name, value.as_units()))
        .collect();
    json!({"id":id,"values":values,"custom":custom,"usedProperties":used})
}
pub(crate) fn validate_projection(
    state: &NativeStyleState,
    wire: &str,
) -> Result<Projection, String> {
    if state.geometry_only {
        return Err(fail("StyleStateGeometryOnly"));
    }
    let options: Projection =
        serde_json::from_str(wire).map_err(|error| fail(&error.to_string()))?;
    if options.abi_version != 1 {
        return Err(fail("AbiVersion"));
    }
    if !options.include_used_properties {
        return Err(fail("StyleStateUsedMetadata"));
    }
    Ok(options)
}
#[cfg(test)]
pub(crate) fn project_state(state: &NativeStyleState, wire: &str) -> Result<String, String> {
    let options = validate_projection(state, wire)?;
    project_validated_state(state, &options)
}
pub(crate) fn project_validated_state(
    state: &NativeStyleState,
    options: &Projection,
) -> Result<String, String> {
    #[cfg(test)]
    PROJECTION_CALLS.with(|count| count.set(count.get() + 1));
    let row = projection_row(state, options)?;
    let (values, used) = public_values(row);
    let targets = if used.is_empty() {
        json!([minimal(state, options.target_id, values, used)])
    } else {
        let mut targets = json!(state.rows());
        for row in targets
            .as_array_mut()
            .expect("native rows serialize as array")
        {
            if row["id"] == options.target_id {
                row["values"] = values.clone();
                row["usedProperties"] = json!(used);
            }
        }
        targets
    };
    serde_json::to_string(&json!({"abiVersion":1,"geometryOnly":false,"targets":targets}))
        .map_err(|error| fail(&error.to_string()))
}

#[cfg(test)]
pub(crate) fn cssom_wire(state: &NativeStyleState, wire: &str) -> Result<String, String> {
    let options: Projection = serde_json::from_str(wire).map_err(|e| fail(&e.to_string()))?;
    let row = projection_row(state, &options)?;
    serde_json::to_string(&json!({"abiVersion":1,"targets":[row]}))
        .map_err(|e| fail(&e.to_string()))
}

#[cfg(test)]
thread_local! { static PROJECTION_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }
#[cfg(test)]
pub(crate) fn take_projection_calls() -> usize {
    PROJECTION_CALLS.with(|count| count.replace(0))
}
