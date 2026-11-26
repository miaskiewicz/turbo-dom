//! Explicit host preferences; no target-platform inference in WASM or guest reads.
use super::{css_backend::ComputationPolicy, css_input};
use serde::Deserialize;
use serde_json::json;
use std::borrow::Cow;
use turbo_dom_core::rtdom::{
    computed_css::preferences::initial_preference_values,
    computed_preferences::{
        computed_font_preferences, default_computed_font_preferences, FontDefaultsPlatform,
        PREFERENCE_SOURCE_SHA256,
    },
};
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Request {
    abi_version: u32,
    host_platform: Platform,
    font_override: Option<Override>,
    limits: Limits,
}
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Platform {
    Macos,
    Other,
}
impl Platform {
    fn core(self) -> FontDefaultsPlatform {
        match self {
            Self::Macos => FontDefaultsPlatform::MacOs,
            Self::Other => FontDefaultsPlatform::Other,
        }
    }
    fn identity(self) -> &'static str {
        match self {
            Self::Macos => "macos",
            Self::Other => "other",
        }
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Override {
    family: Vec<u16>,
    size_px: f64,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Limits {
    input_units: usize,
    retained_units: usize,
    computation: ComputationPolicy,
}
fn fail(reason: impl std::fmt::Debug) -> String {
    format!("CSSBackendPreferences:{reason:?}")
}
fn charge(remaining: &mut usize, amount: usize) -> Result<(), String> {
    css_input::charge(remaining, amount).map_err(fail)
}
fn ua_units() -> usize {
    use turbo_dom_layout::style::{
        computed_document_user_agent_css, fullscreen_user_agent_css, DOCUMENT_USER_AGENT_CSS,
    };
    [
        computed_document_user_agent_css(),
        DOCUMENT_USER_AGENT_CSS,
        fullscreen_user_agent_css(),
    ]
    .into_iter()
    .map(|value| value.encode_utf16().count())
    .sum()
}
fn font_preferences(
    platform: Platform,
    override_value: Option<Override>,
    input_units: usize,
) -> Result<turbo_dom_core::rtdom::computed_preferences::ComputedFontPreferences<'static>, String> {
    match override_value {
        None => Ok(default_computed_font_preferences(platform.core())),
        Some(value) => {
            let mut input = input_units;
            charge(&mut input, value.family.len())?;
            let family = String::from_utf16(&value.family)
                .map_err(|_| "CSSBackendPreferences:NonScalarFamily".to_owned())?;
            computed_font_preferences(Cow::Owned(family), value.size_px).map_err(fail)
        }
    }
}
fn request(source: &str) -> Result<Request, String> {
    let request: Request = serde_json::from_str(source).map_err(fail)?;
    if request.abi_version != 1 {
        return Err("CSSBackendPreferences:AbiVersion".into());
    }
    Ok(request)
}
pub(super) fn preferences(source: &str) -> Result<String, String> {
    let request = request(source)?;
    let fonts = font_preferences(
        request.host_platform,
        request.font_override,
        request.limits.input_units,
    )?;
    let mut computation: turbo_dom_core::rtdom::computed_css::ComputationLimits =
        request.limits.computation.into();
    computation.max_total_units = computation
        .max_total_units
        .min(request.limits.retained_units);
    let initial = initial_preference_values(&fonts, computation).map_err(fail)?;
    let mut retained = request.limits.retained_units;
    charge_preferences(&initial, &mut retained)?;
    // An exact, owned encoding, not a hash, opaque token, or process registry key.
    let identity = identity_units(
        &(
            request.host_platform.identity(),
            fonts.family.as_ref(),
            fonts.size_px,
            PREFERENCE_SOURCE_SHA256,
        ),
        &mut retained,
    )?;
    let ua = user_agent(&mut retained)?;
    serde_json::to_string(&json!({
        "abiVersion":1,
        "initial":{"fontFamily":initial.font_family.as_units(),"fontSizePx":initial.font_size_px,"computedFontSize":initial.computed_font_size.as_units(),"fontSizeInput":initial.font_size_input.as_units(),"computedFontSizePx":initial.computed_font_size_px,"color":initial.color.as_units()},
        "preferenceIdentity":identity,"userAgentSource":ua
    })).map_err(fail)
}
// This writer is used only by serde_json's scalar tuple serializer: string
// fragments are valid UTF-8 slices and the remaining writes are ASCII syntax.
// It counts canonical wire output before allocating the retained identity copy.
struct IdentityCounter(usize);
impl std::io::Write for IdentityCounter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let text = std::str::from_utf8(bytes).map_err(std::io::Error::other)?;
        self.0 = self
            .0
            .checked_sub(text.encode_utf16().count())
            .ok_or_else(|| std::io::Error::other("ResourceLimit"))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn identity_units(
    identity: &(&str, &str, f64, &str),
    remaining: &mut usize,
) -> Result<Vec<u16>, String> {
    let mut counter = IdentityCounter(*remaining);
    serde_json::to_writer(&mut counter, identity)
        .map_err(|_| fail(css_input::InputError::ResourceLimit))?;
    let encoded = serde_json::to_string(identity).map_err(fail)?;
    *remaining = counter.0;
    Ok(encoded.encode_utf16().collect())
}
fn charge_preferences(
    initial: &turbo_dom_core::rtdom::computed_css::preferences::InitialPreferenceValues,
    retained: &mut usize,
) -> Result<(), String> {
    for value in [
        &initial.font_family,
        &initial.color,
        &initial.computed_font_size,
        &initial.font_size_input,
    ] {
        charge(retained, value.as_units().len())?;
    }
    Ok(())
}
fn user_agent(retained: &mut usize) -> Result<serde_json::Value, String> {
    charge(retained, ua_units())?;
    charge_initials(retained)?;
    Ok(super::css_backend::user_agent_source_value(
        turbo_dom_layout::style::computed_document_user_agent_css(),
    ))
}
fn charge_initials(remaining: &mut usize) -> Result<(), String> {
    for name in super::css_backend::CONTEXT_FREE_INITIAL_NAMES {
        if let Some(value) =
            turbo_dom_core::rtdom::computed_css::properties::context_free_initial(name)
        {
            charge(remaining, name.encode_utf16().count())?;
            charge(remaining, value.as_units().len())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    fn request(platform: &str) -> Value {
        json!({"abiVersion":1,"hostPlatform":platform,"fontOverride":null,"limits":{"inputUnits":1024,"retainedUnits":100_000,"computation":{"maxNesting":0,"maxOutputUnits":1024,"maxTotalUnits":2048}}})
    }
    fn run(input: &Value) -> Result<Value, String> {
        serde_json::from_str(&preferences(&input.to_string())?).map_err(fail)
    }
    fn string(value: &Value) -> String {
        String::from_utf16(&serde_json::from_value::<Vec<u16>>(value.clone()).unwrap()).unwrap()
    }
    #[test]
    fn explicit_platform_and_exact_owned_override_are_independent() {
        let mac = run(&request("macos")).unwrap();
        let other = run(&request("other")).unwrap();
        assert_eq!(string(&mac["initial"]["fontFamily"]), "Times");
        assert_eq!(string(&other["initial"]["fontFamily"]), "Times New Roman");
        assert_ne!(mac["preferenceIdentity"], other["preferenceIdentity"]);
        let mut input = request("other");
        input["fontOverride"] =
            json!({"family":"  Odd 😀  ".encode_utf16().collect::<Vec<_>>(),"sizePx":18.5});
        let result = run(&input).unwrap();
        assert_eq!(string(&result["initial"]["fontFamily"]), "  Odd 😀  ");
        assert_eq!(result["initial"]["fontSizePx"], 18.5);
        assert_eq!(
            run(&input).unwrap()["preferenceIdentity"],
            result["preferenceIdentity"]
        );
    }
    #[test]
    fn raw_huge_preference_identity_is_distinct_from_normalized_computed_basis() {
        let mut input = request("macos");
        input["fontOverride"] = json!({"family":[65],"sizePx":1e300});
        let result = run(&input).unwrap();
        assert_eq!(result["initial"]["fontSizePx"], 1e300);
        assert_eq!(result["initial"]["computedFontSizePx"], 10000.0);
        assert_eq!(string(&result["initial"]["computedFontSize"]), "10000px");
        input["fontOverride"]["sizePx"] = json!(10000);
        let bounded = run(&input).unwrap();
        assert_eq!(
            bounded["initial"]["computedFontSize"],
            result["initial"]["computedFontSize"]
        );
        assert_ne!(bounded["preferenceIdentity"], result["preferenceIdentity"]);
    }
    #[test]
    fn old_renderer_ua_is_unchanged_and_new_document_ua_is_distinct() {
        let old = super::super::css_backend::user_agent_source_value(
            turbo_dom_layout::style::user_agent_css(),
        );
        let new = run(&request("macos")).unwrap();
        assert!(string(&old["text"]).contains("body { font-family: sans-serif }"));
        assert!(
            !string(&new["userAgentSource"]["text"]).contains("body { font-family: sans-serif }")
        );
        assert_eq!(old["documentText"], new["userAgentSource"]["documentText"]);
        assert_eq!(
            old["fullscreenText"],
            new["userAgentSource"]["fullscreenText"]
        );
    }
    #[test]
    fn unicode_control_identity_and_full_payload_obey_exact_retained_quota() {
        let mut input = request("macos");
        input["fontOverride"] =
            json!({"family":"A\0😀\"".encode_utf16().collect::<Vec<_>>(),"sizePx":16.0});
        let output = run(&input).unwrap();
        let initial = &output["initial"];
        let mut retained: usize = ["fontFamily", "computedFontSize", "fontSizeInput", "color"]
            .iter()
            .map(|name| initial[*name].as_array().unwrap().len())
            .sum();
        retained += output["preferenceIdentity"].as_array().unwrap().len();
        let ua = &output["userAgentSource"];
        for name in ["text", "documentText", "fullscreenText"] {
            retained += ua[name].as_array().unwrap().len();
        }
        for (name, value) in ua["contextFreeInitials"].as_object().unwrap() {
            retained += name.encode_utf16().count() + value.as_array().unwrap().len();
        }
        input["limits"]["retainedUnits"] = json!(retained);
        assert_eq!(run(&input).unwrap(), output);
        input["limits"]["retainedUnits"] = json!(retained - 1);
        assert!(run(&input).unwrap_err().contains("ResourceLimit"));
    }
    #[test]
    fn raw_preference_request_preserves_canonical_f64_json_bits() {
        let mut values = vec![
            16.123_456_789,
            0.845_512_408_225_570_1,
            0.572_364_942_924_700_1,
            1.234_567_890_123_456_7,
            f64::from_bits(0x3ff0_0000_0000_0001),
            f64::from_bits(0x3fd5_5555_5555_5555),
            f64::MIN_POSITIVE,
            f64::from_bits(1),
            f64::MAX,
            1e300,
        ];
        // Bounded finite [1,2) fractions exercise nontrivial seventeen-digit
        // round trips without involving CSS dimension rounding or layout.
        let mut state = 0x2960_471b_d5f5_2816_u64;
        for _ in 0..128 {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            values.push(f64::from_bits(
                0x3ff0_0000_0000_0000 | (state & 0x000f_ffff_ffff_ffff),
            ));
        }
        let mut mismatches = Vec::new();
        for expected in values {
            let mut input = request("macos");
            input["fontOverride"] = json!({"family":[65],"sizePx":expected});
            let wire = serde_json::to_string(&input).unwrap();
            match super::request(&wire) {
                Ok(parsed) => {
                    let actual = parsed.font_override.unwrap().size_px;
                    if actual.to_bits() != expected.to_bits() {
                        mismatches.push(format!(
                            "expected={expected:?} bits={:016x}, actual={actual:?} bits={:016x}",
                            expected.to_bits(),
                            actual.to_bits()
                        ));
                    }
                }
                Err(error) => mismatches.push(format!(
                    "expected={expected:?} bits={:016x}, error={error}",
                    expected.to_bits()
                )),
            }
        }
        assert!(mismatches.is_empty(), "{mismatches:#?}");
    }
    #[test]
    fn invalid_raw_preferences_and_explicit_resource_limits_do_not_fallback() {
        let mut input = request("macos");
        input["fontOverride"] = json!({"family":[55296],"sizePx":16});
        assert!(run(&input).unwrap_err().contains("NonScalarFamily"));
        input["fontOverride"] = json!({"family":[32],"sizePx":16});
        assert!(run(&input).unwrap_err().contains("EmptyFamily"));
        input["fontOverride"] = json!({"family":[65],"sizePx":0});
        assert!(run(&input).unwrap_err().contains("InvalidSize"));
        input["fontOverride"] = json!({"family":[65],"sizePx":16});
        input["limits"]["inputUnits"] = json!(0);
        assert!(run(&input).unwrap_err().contains("ResourceLimit"));
        input = request("macos");
        input["limits"]["retainedUnits"] = json!(0);
        assert!(run(&input).unwrap_err().contains("ResourceLimit"));
        input = request("macos");
        input["abiVersion"] = json!(2);
        assert!(run(&input).unwrap_err().contains("AbiVersion"));
        input = request("unknown");
        assert!(run(&input).is_err());
    }
}
