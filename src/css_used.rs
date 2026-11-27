//! Requested-property projection over canonical Core policy and explicit metrics.
use super::{css_backend::ComputationPolicy, css_input};
use serde::{Deserialize, Serialize};
use turbo_dom_core::rtdom::computed_css::used_values::{
    project_used_property, Axis, AxisMeasurement, BoxBasis, Projection, UsedContext,
    UsedMeasurement,
};
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Request {
    abi_version: u32,
    target: Target,
    property: String,
    measurement: Option<Measurement>,
    limits: Limits,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Target {
    id: u32,
    world: Vec<u16>,
    namespace: Vec<u16>,
    pseudo: Option<String>,
    ordinary: Vec<(String, Vec<u16>)>,
    public_keys: Vec<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Limits {
    input_units: usize,
    retained_units: usize,
    computation: ComputationPolicy,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Measurement {
    target: u32,
    world: Vec<u16>,
    horizontal: Metric,
    vertical: Metric,
}
#[derive(Clone, Copy, Deserialize)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
enum Metric {
    Unknown,
    NoBox,
    NonAtomicInline,
    Pixels {
        content: Option<f64>,
        border: Option<f64>,
    },
}
#[derive(Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
enum ResultValue {
    Unchanged,
    NeedsMeasurement {
        axis: &'static str,
        basis: &'static str,
    },
    Value {
        value: Vec<u16>,
    },
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Response {
    abi_version: u32,
    id: u32,
    property: String,
    result: ResultValue,
}
fn fail(reason: impl std::fmt::Debug) -> String {
    format!("CSSBackendUsed:{reason:?}")
}
fn charge(input: &mut usize, amount: usize) -> Result<(), String> {
    css_input::charge(input, amount).map_err(fail)
}
fn pixel(value: Option<f64>) -> Result<Option<f32>, String> {
    value
        .map(|value| {
            if !value.is_finite() || value < 0.0 || !(value as f32).is_finite() {
                return Err("CSSBackendUsed:InvalidMeasurement".into());
            }
            Ok(value as f32)
        })
        .transpose()
}
fn metric(metric: Metric, basis: BoxBasis) -> Result<AxisMeasurement, String> {
    Ok(match metric {
        Metric::Unknown => AxisMeasurement::Unknown,
        Metric::NoBox => AxisMeasurement::NoBox,
        Metric::NonAtomicInline => AxisMeasurement::NonAtomicInline,
        Metric::Pixels { content, border } => match basis {
            BoxBasis::Content => AxisMeasurement::Pixels {
                content: pixel(content)?,
                border: None,
            },
            BoxBasis::Border => AxisMeasurement::Pixels {
                content: None,
                border: pixel(border)?,
            },
        },
    })
}
fn converted<'a>(
    source: &Measurement,
    target: &'a Target,
    axis: Axis,
    basis: BoxBasis,
) -> Result<UsedMeasurement<'a>, String> {
    if source.target != target.id || source.world != target.world {
        return Err("CSSBackendUsed:ForeignMeasurement".into());
    }
    // World is borrowed from the target at the call site; no allocated identity is retained.
    let (horizontal, vertical) = match axis {
        Axis::Horizontal => (metric(source.horizontal, basis)?, AxisMeasurement::Unknown),
        Axis::Vertical => (AxisMeasurement::Unknown, metric(source.vertical, basis)?),
    };
    Ok(UsedMeasurement {
        target: target.id,
        world: &target.world,
        horizontal,
        vertical,
    })
}
pub(super) fn project_property(source: &str) -> Result<String, String> {
    let mut request: Request = serde_json::from_str(source).map_err(fail)?;
    if request.abi_version != 1 {
        return Err("CSSBackendUsed:UnsupportedAbi".into());
    }
    let mut input = charge_request(&request)?;
    let ordinary = std::mem::take(&mut request.target.ordinary);
    let public_keys = std::mem::take(&mut request.target.public_keys);
    let ordinary = css_input::ordinary(ordinary, &mut input).map_err(fail)?;
    let public_keys = css_input::provenance(public_keys, &mut input).map_err(fail)?;
    let context = UsedContext {
        target: request.target.id,
        world: &request.target.world,
        namespace: &request.target.namespace,
        pseudo: request.target.pseudo.as_deref(),
        ordinary: &ordinary,
        public_keys: &public_keys,
        measurement: None,
    };
    let projected = evaluate(&request, &context)?;
    let result = result_value(projected, request.limits.retained_units)?;
    serde_json::to_string(&Response {
        abi_version: 1,
        id: request.target.id,
        property: request.property,
        result,
    })
    .map_err(fail)
}

fn charge_request(request: &Request) -> Result<usize, String> {
    let mut input = request.limits.input_units;

    charge(&mut input, request.property.encode_utf16().count())?;
    charge(&mut input, request.target.world.len())?;
    charge(&mut input, request.target.namespace.len())?;
    if let Some(pseudo) = &request.target.pseudo {
        charge(&mut input, pseudo.encode_utf16().count())?;
    }
    if let Some(measurement) = &request.measurement {
        charge(&mut input, measurement.world.len())?;
    }
    Ok(input)
}
fn evaluate(request: &Request, context: &UsedContext<'_>) -> Result<Projection, String> {
    let mut projected = project_used_property(
        &request.property,
        context,
        request.limits.computation.into(),
    )
    .map_err(fail)?;
    if let (Projection::NeedsMeasurement { axis, basis }, Some(source)) =
        (&projected, request.measurement.as_ref())
    {
        let measurement = converted(source, &request.target, *axis, *basis)?;
        let measured_context = UsedContext {
            measurement: Some(&measurement),
            ..*context
        };
        projected = project_used_property(
            &request.property,
            &measured_context,
            request.limits.computation.into(),
        )
        .map_err(fail)?;
    }
    Ok(projected)
}
fn result_value(projected: Projection, retained: usize) -> Result<ResultValue, String> {
    let result = match projected {
        Projection::Unchanged => ResultValue::Unchanged,
        Projection::NeedsMeasurement { axis, basis } => ResultValue::NeedsMeasurement {
            axis: match axis {
                Axis::Horizontal => "horizontal",
                Axis::Vertical => "vertical",
            },
            basis: match basis {
                BoxBasis::Content => "content",
                BoxBasis::Border => "border",
            },
        },
        Projection::Value(value) => {
            let mut retained = retained;
            charge(&mut retained, value.as_units().len())?;
            ResultValue::Value {
                value: value.as_units().to_vec(),
            }
        }
    };
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    fn units(value: &str) -> Vec<u16> {
        value.encode_utf16().collect()
    }
    fn request() -> Value {
        json!({"abiVersion":1,"target":{"id":7,"world":[55296,2],"namespace":units("http://www.w3.org/1999/xhtml"),"ordinary":[["display",units("block")]],"publicKeys":["width","height"]},"property":"width","limits":{"inputUnits":1024,"retainedUnits":64,"computation":{"maxNesting":32,"maxOutputUnits":128,"maxTotalUnits":128}}})
    }
    fn run(value: &Value) -> Result<Value, String> {
        serde_json::from_str(&project_property(&value.to_string())?).map_err(fail)
    }
    fn measure(value: &mut Value, horizontal: Value, vertical: Value) {
        value["measurement"] =
            json!({"target":7,"world":[55296,2],"horizontal":horizontal,"vertical":vertical});
    }
    #[test]
    fn cold_selected_axis_and_basis_only_are_required() {
        let mut input = request();
        assert_eq!(
            run(&input).unwrap()["result"],
            json!({"status":"needsMeasurement","axis":"horizontal","basis":"content"})
        );
        measure(
            &mut input,
            json!({"status":"pixels","content":0,"border":-1}),
            json!({"status":"pixels","content":-1}),
        );
        assert_eq!(
            run(&input).unwrap()["result"],
            json!({"status":"value","value":units("0px")})
        );
        input["target"]["ordinary"]
            .as_array_mut()
            .unwrap()
            .push(json!(["box-sizing", units("border-box")]));
        assert!(run(&input).unwrap_err().contains("InvalidMeasurement"));
        input["measurement"]["horizontal"] = json!({"status":"pixels","border":12,"content":-1});
        assert_eq!(
            run(&input).unwrap()["result"]["value"],
            json!(units("12px"))
        );
        input["property"] = json!("height");
        assert_eq!(
            run(&input).unwrap()["result"],
            json!({"status":"needsMeasurement","axis":"vertical","basis":"border"})
        );
        input["measurement"]["vertical"] = json!({"status":"pixels","border":-1});
        assert!(run(&input).unwrap_err().contains("InvalidMeasurement"));
    }
    #[test]
    fn identity_precedes_selected_float_conversion_and_no_box_is_not_zero() {
        for status in ["noBox", "nonAtomicInline"] {
            let mut input = request();
            measure(
                &mut input,
                json!({"status":status}),
                json!({"status":"unknown"}),
            );
            assert_eq!(run(&input).unwrap()["result"]["status"], "unchanged");
        }
        for value in [-1.0, -1.0e-300, 1.0e300] {
            let mut input = request();
            measure(
                &mut input,
                json!({"status":"pixels","content":value}),
                json!({"status":"unknown"}),
            );
            assert!(run(&input).unwrap_err().contains("InvalidMeasurement"));
            input["measurement"]["world"] = json!([1]);
            assert!(run(&input).unwrap_err().contains("ForeignMeasurement"));
        }
        let mut input = request();
        measure(
            &mut input,
            json!({"status":"unknown"}),
            json!({"status":"unknown"}),
        );
        input["measurement"]["target"] = json!(8);
        assert!(run(&input).unwrap_err().contains("ForeignMeasurement"));
    }
    #[test]
    fn input_and_output_quotas_preserve_exact_boundaries() {
        let mut input = request();
        // property + world + namespace + display/value + two public keys.
        let exact = 5 + 2 + units("http://www.w3.org/1999/xhtml").len() + 7 + 5 + 5 + 6;
        input["limits"]["inputUnits"] = json!(exact);
        assert!(run(&input).is_ok());
        input["limits"]["inputUnits"] = json!(exact - 1);
        assert!(run(&input).unwrap_err().contains("ResourceLimit"));
        input["limits"]["inputUnits"] = json!(1024);
        measure(
            &mut input,
            json!({"status":"pixels","content":2}),
            json!({"status":"unknown"}),
        );
        input["limits"]["retainedUnits"] = json!(3);
        assert!(run(&input).is_ok());
        input["limits"]["retainedUnits"] = json!(2);
        assert!(run(&input).unwrap_err().contains("ResourceLimit"));
        input["limits"]["computation"]["maxOutputUnits"] = json!(1);
        assert!(run(&input).unwrap_err().contains("ResourceLimit"));
    }
    #[test]
    fn provenance_duplicates_namespace_and_pseudo_are_explicit() {
        let mut input = request();
        input["target"]["ordinary"]
            .as_array_mut()
            .unwrap()
            .push(json!(["display", units("none")]));
        assert!(run(&input).unwrap_err().contains("DuplicateProperty"));
        let mut input = request();
        input["target"]["publicKeys"] = json!(["width", "width"]);
        assert!(run(&input).unwrap_err().contains("DuplicatePublicKey"));
        let mut input = request();
        input["target"]["pseudo"] = json!("before");
        assert!(run(&input).unwrap_err().contains("UnsupportedPseudo"));
        let mut input = request();
        input["target"]["namespace"] = json!(units("http://www.w3.org/2000/svg"));
        assert!(run(&input).unwrap_err().contains("UnsupportedNamespace"));
        input["property"] = json!("color");
        assert_eq!(run(&input).unwrap()["result"]["status"], "unchanged");
        input["abiVersion"] = json!(2);
        assert!(run(&input).unwrap_err().contains("UnsupportedAbi"));
    }
}
