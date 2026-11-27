//! Lazy CSS artifact boundary. This module owns CSS models, never DOM wrappers.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use turbo_dom_core::rtdom::computed_css::{
    conditions::{ConditionEnvironment, FeatureValue},
    model::{CompileLimits, CompiledStyles, SheetSource},
    prepared::{
        CascadeOrigin, PreparationEnvironment, PreparationLimits, PreparedStyles, SheetUse,
        StylePhase,
    },
    ComputationLimits, CssText,
};
use turbo_dom_core::rtdom::css_shorthand::LengthContext;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CompileRequest {
    abi_version: u32,
    sources: Vec<Source>,
    limits: CompilePolicy,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    id: u32,
    text: Vec<u16>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CompilePolicy {
    sources: usize,
    source_units: usize,
    rules: usize,
    source_rule_preludes: usize,
    selectors: usize,
    declarations: usize,
    properties: usize,
    group_depth: usize,
    token_nesting: usize,
}
impl From<CompilePolicy> for CompileLimits {
    fn from(value: CompilePolicy) -> Self {
        Self {
            sources: value.sources,
            source_units: value.source_units,
            rules: value.rules,
            source_rule_preludes: value.source_rule_preludes,
            selectors: value.selectors,
            declarations: value.declarations,
            properties: value.properties,
            group_depth: value.group_depth,
            token_nesting: value.token_nesting,
        }
    }
}
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ComputationPolicy {
    #[serde(rename = "maxNesting")]
    nesting: usize,
    #[serde(rename = "maxOutputUnits")]
    output_units: usize,
    #[serde(rename = "maxTotalUnits")]
    total_units: usize,
}
impl From<ComputationPolicy> for ComputationLimits {
    fn from(value: ComputationPolicy) -> Self {
        Self {
            max_nesting: value.nesting,
            max_output_units: value.output_units,
            max_total_units: value.total_units,
        }
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PrepareRequest {
    abi_version: u32,
    uses: Vec<SheetBinding>,
    environment: Environment,
    limits: PreparePolicy,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PreparePolicy {
    uses: usize,
    rules: usize,
    layers: usize,
    computation: ComputationPolicy,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SheetBinding {
    source_id: u32,
    origin: Origin,
    tree_scope: u32,
    scope_depth: u32,
    disabled: bool,
    media: Option<Vec<u16>>,
    implicit_scope_root: Option<u32>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
enum Origin {
    UserAgent,
    User,
    Author,
}
impl From<Origin> for CascadeOrigin {
    fn from(value: Origin) -> Self {
        match value {
            Origin::UserAgent => Self::UserAgent,
            Origin::User => Self::User,
            Origin::Author => Self::Author,
        }
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Environment {
    media_type: String,
    features: BTreeMap<String, MediaFact>,
    unit_bases: Vec<(String, f64)>,
    selector_support: BTreeMap<String, bool>,
    style_phase: Option<Phase>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
enum Phase {
    Steady,
    Starting,
}
#[derive(Deserialize, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub(super) enum MediaFact {
    Number(f64),
    LengthPx(f64),
    Ratio((f64, f64)),
    ResolutionDppx(f64),
    Ident(String),
    Boolean(bool),
}
impl From<MediaFact> for FeatureValue {
    fn from(value: MediaFact) -> Self {
        match value {
            MediaFact::Number(value) => Self::Number(value),
            MediaFact::LengthPx(value) => Self::LengthPx(value),
            MediaFact::Ratio((x, y)) => Self::Ratio(x, y),
            MediaFact::ResolutionDppx(value) => Self::ResolutionDppx(value),
            MediaFact::Ident(value) => Self::Ident(value),
            MediaFact::Boolean(value) => Self::Boolean(value),
        }
    }
}

impl From<FeatureValue> for MediaFact {
    fn from(value: FeatureValue) -> Self {
        match value {
            FeatureValue::Number(value) => Self::Number(value),
            FeatureValue::LengthPx(value) => Self::LengthPx(value),
            FeatureValue::Ratio(x, y) => Self::Ratio((x, y)),
            FeatureValue::ResolutionDppx(value) => Self::ResolutionDppx(value),
            FeatureValue::Ident(value) => Self::Ident(value),
            FeatureValue::Boolean(value) => Self::Boolean(value),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SceneCall {
    abi_version: u32,
    scene: crate::css_scene::SceneRequest,
}
fn measure_scene_v1(source: &str) -> Result<String, String> {
    let request: SceneCall = transport(source)?;
    abi(request.abi_version)?;
    let result = crate::css_scene::measure_scene(&request.scene)
        .map_err(|error| format!("CSSBackendScene:{error:?}"))?;
    encode(&result)
}
struct CompiledHandle {
    model: Option<CompiledStyles>,
}
struct PreparedHandle {
    #[cfg(any(feature = "napi-bind", test))]
    matching: Option<crate::css_resolve::retained::MatchingSlot>,
    model: Option<PreparedStyles>,
}
fn transport<T: for<'a> Deserialize<'a>>(source: &str) -> Result<T, String> {
    serde_json::from_str(source).map_err(|error| format!("CSSBackendTransport:{error}"))
}
fn encode(value: &impl Serialize) -> Result<String, String> {
    serde_json::to_string(value).map_err(|error| format!("CSSBackendTransport:{error}"))
}
fn abi(version: u32) -> Result<(), String> {
    if version == 1 {
        Ok(())
    } else {
        Err("CSSBackendABI:expected1".to_owned())
    }
}

impl CompiledHandle {
    fn new(source: &str) -> Result<Self, String> {
        let request: CompileRequest = transport(source)?;
        abi(request.abi_version)?;
        let sources: Vec<_> = request
            .sources
            .into_iter()
            .map(|source| SheetSource {
                id: source.id,
                text: CssText::from_units(source.text),
            })
            .collect();
        let model = CompiledStyles::compile(&sources, request.limits.into())
            .map_err(|error| format!("CSSBackendCompile:{error:?}"))?;
        Ok(Self { model: Some(model) })
    }
    fn model(&self) -> Result<&CompiledStyles, String> {
        self.model
            .as_ref()
            .ok_or_else(|| "CSSBackendDisposed:CompiledStylesV1".to_owned())
    }
    fn metadata(&self) -> Result<String, String> {
        compiled_metadata(self.model()?)
    }
    fn prepare(&self, source: &str) -> Result<PreparedHandle, String> {
        let request: PrepareRequest = transport(source)?;
        abi(request.abi_version)?;
        let uses: Vec<_> = request
            .uses
            .into_iter()
            .map(|binding| SheetUse {
                source_id: binding.source_id,
                origin: binding.origin.into(),
                tree_scope: binding.tree_scope,
                scope_depth: binding.scope_depth,
                disabled: binding.disabled,
                media: binding.media.map(CssText::from_units),
                implicit_scope_root: binding.implicit_scope_root,
            })
            .collect();
        let features = request
            .environment
            .features
            .into_iter()
            .map(|(name, value)| (name, value.into()))
            .collect();
        let bases: Vec<_> = request
            .environment
            .unit_bases
            .iter()
            .map(|(name, value)| (name.as_str(), *value))
            .collect();
        let length = LengthContext { unit_bases: &bases };
        let conditions = ConditionEnvironment {
            media_type: &request.environment.media_type,
            features: &features,
            length_context: &length,
            selector_support: &request.environment.selector_support,
        };
        let environment = PreparationEnvironment {
            conditions: &conditions,
            style_phase: request.environment.style_phase.map(|phase| match phase {
                Phase::Steady => StylePhase::Steady,
                Phase::Starting => StylePhase::Starting,
            }),
        };
        let limits = PreparationLimits {
            uses: request.limits.uses,
            rules: request.limits.rules,
            layers: request.limits.layers,
            computation: request.limits.computation.into(),
        };
        let model = self
            .model()?
            .prepare(&uses, &environment, limits)
            .map_err(|error| format!("CSSBackendPrepare:{error:?}"))?;
        Ok(PreparedHandle {
            model: Some(model),
            #[cfg(any(feature = "napi-bind", test))]
            matching: None,
        })
    }
    fn retained_bytes(&self) -> f64 {
        self.model
            .as_ref()
            .map_or(0, CompiledStyles::retained_payload_bytes) as f64
    }
    fn dispose(&mut self) {
        self.model.take();
    }
}
impl PreparedHandle {
    fn model(&self) -> Result<&PreparedStyles, String> {
        self.model
            .as_ref()
            .ok_or_else(|| "CSSBackendDisposed:PreparedStylesV1".to_owned())
    }
    fn metadata(&self) -> Result<String, String> {
        let metadata = self.model()?.metadata();
        let rules:Vec<_>=metadata.rules.iter().enumerate().map(|(id,rule)|json!({"id":id,"ruleId":rule.rule_id,"useId":rule.use_id,"treeScope":rule.tree_scope,"scopeDepth":rule.scope_depth,"implicitScopeRoot":rule.implicit_scope_root})).collect();
        encode(
            &json!({"abiVersion":1,"retainedBytesKind":"shared-css-payload-lower-bound","rules":rules}),
        )
    }
    fn resolve_batch(&self, source: &str) -> Result<String, String> {
        crate::css_resolve::resolve_batch(self.model()?, source)
    }
    fn resolve_container_geometry(&self, source: &str) -> Result<String, String> {
        let plan = self
            .model()?
            .container_geometry_plan()
            .map_err(|error| format!("CSSBackendGeometry:{error:?}"))?;
        crate::css_resolve::resolve_batch(plan.styles_for_measurement(), source)
    }
    #[cfg(any(feature = "napi-bind", test))]
    fn store_matching(&mut self, wire: &str, limits: &str) -> Result<String, String> {
        self.model()?;
        let next = crate::css_resolve::retained::MatchingSlot::new(wire, limits)?;
        let token = next.token().to_owned();
        self.matching = Some(next);
        Ok(token)
    }
    #[cfg(any(feature = "napi-bind", test))]
    fn resolve_matching(&self, wire: &str, token: &str) -> Result<String, String> {
        let model = self.model()?;
        self.matching
            .as_ref()
            .filter(|slot| slot.token() == token)
            .ok_or_else(|| "CSSBackendFacts:StaleMatching".to_owned())?
            .resolve(model, wire)
    }
    fn retained_bytes(&self) -> f64 {
        let bytes = self
            .model
            .as_ref()
            .map_or(0, PreparedStyles::retained_payload_bytes);
        #[cfg(any(feature = "napi-bind", test))]
        let bytes = bytes
            + self
                .matching
                .as_ref()
                .map_or(0, crate::css_resolve::retained::MatchingSlot::payload_bytes);
        bytes as f64
    }
    fn dispose(&mut self) {
        #[cfg(any(feature = "napi-bind", test))]
        self.matching.take();
        self.model.take();
    }
}
fn selector_items(
    compiled: &Result<
        turbo_dom_core::rtdom::query::SelectorList,
        turbo_dom_core::rtdom::query::SelectorError,
    >,
    scoped: bool,
) -> Value {
    match compiled {
        Ok(list) => Value::Array(list.selectors().iter().enumerate().map(|(index, item)| json!({
            "index":index,"units":item.text_units().as_ref(),"specificity":item.specificity(),
            "pseudo":item.pseudo_element(),"referencesScope":item.references_scope(),
            "implicitScopeAnchor":scoped && !item.references_scope(),"seed":subject_seed(item.subject_seed())
        })).collect()),
        Err(_) => Value::Null,
    }
}
fn user_agent_source_v1() -> Result<String, String> {
    encode(&user_agent_source_value(
        turbo_dom_layout::style::user_agent_css(),
    ))
}
pub(super) const CONTEXT_FREE_INITIAL_NAMES: [&str; 7] = [
    "container-name",
    "container-type",
    "writing-mode",
    "box-sizing",
    "float",
    "position",
    "display",
];
pub(super) fn user_agent_source_value(css: &str) -> Value {
    use turbo_dom_layout::style::{fullscreen_user_agent_css, DOCUMENT_USER_AGENT_CSS};
    let context_free_initials: BTreeMap<_, _> = CONTEXT_FREE_INITIAL_NAMES
        .into_iter()
        .filter_map(|name| {
            turbo_dom_core::rtdom::computed_css::properties::context_free_initial(name)
                .map(|value| (name, value.as_units().to_vec()))
        })
        .collect();
    json!({
        "abiVersion":1,
        "contextFreeInitials":context_free_initials,
        "text":css.encode_utf16().collect::<Vec<_>>(),
        "documentText":DOCUMENT_USER_AGENT_CSS.encode_utf16().collect::<Vec<_>>(),
        "fullscreenText":fullscreen_user_agent_css().encode_utf16().collect::<Vec<_>>()
    })
}
fn selector_policy_metadata(
    policy: &turbo_dom_core::rtdom::computed_css::selector_policy::SelectorPolicyInspection,
) -> Value {
    use turbo_dom_core::rtdom::computed_css::selector_policy::{
        GeneratedPseudo, SelectorPolicyInspection, SelectorTarget,
    };
    let pseudo = |value: GeneratedPseudo| match value {
        GeneratedPseudo::Before => "before",
        GeneratedPseudo::After => "after",
    };
    match policy {
        SelectorPolicyInspection::Unchanged => Value::Null,
        SelectorPolicyInspection::Invalid => json!({"invalid":true}),
        SelectorPolicyInspection::Items(items) => Value::Array(items.iter().map(|item| {
            let target = match item.target {
                SelectorTarget::Element => json!({"kind":"element"}),
                SelectorTarget::Generated { pseudo: value } => json!({"kind":"generated","pseudo":pseudo(value)}),
                SelectorTarget::Slotted { pseudo: value } => json!({"kind":"slotted","pseudo":value.map(pseudo)}),
                SelectorTarget::UnsupportedComposed => json!({"kind":"unsupported_composed"}),
            };
            json!({"index":item.index,"hostReference":item.host_reference,"target":target,"specificity":item.specificity})
        }).collect()),
    }
}
fn boundary_items(
    compiled: &Result<
        turbo_dom_core::rtdom::query::SelectorList,
        turbo_dom_core::rtdom::query::SelectorError,
    >,
    policy: Option<&turbo_dom_core::rtdom::computed_css::selector_policy::SelectorPolicyInspection>,
) -> Value {
    let mut items = selector_items(compiled, false);
    if let (Value::Array(items), Some(policy)) = (&mut items, policy) {
        let policies = selector_policy_metadata(policy);
        for (index, item) in items.iter_mut().enumerate() {
            let certificate = match &policies {
                Value::Array(values) => values
                    .get(index)
                    .cloned()
                    .unwrap_or_else(|| json!({"invalid":true})),
                value => value.clone(),
            };
            item["policy"] = certificate;
        }
    }
    items
}
fn compiled_metadata(styles: &CompiledStyles) -> Result<String, String> {
    let data = styles.metadata();
    let selectors: Vec<_> = data.selectors.iter().enumerate().map(|(id, selector)| json!({
        "id":id,"units":selector.source.as_units(),"items":selector_items(&selector.compiled,selector.scoped),"policy":selector_policy_metadata(&selector.policy)
    })).collect();
    let groups: Vec<_> = data.groups.iter().enumerate().map(|(id, group)| json!({
        "id":id,"kind":format!("{:?}",group.kind),"prelude":group.prelude.as_units(),"parent":group.parent,
        "invalidScope":group.invalid_scope,"scope":group.scope.as_ref().map(|scope|json!({
            "start":scope.start.as_ref().map(CssText::as_units),"end":scope.end.as_ref().map(CssText::as_units),
            "startItems":group.scope_start.as_ref().map(|items|boundary_items(items,group.scope_start_policy.as_ref())),
            "endItems":group.scope_end.as_ref().map(|items|boundary_items(items,group.scope_end_policy.as_ref()))
        }))
    })).collect();
    let rules:Vec<_>=data.rules.iter().enumerate().map(|(id,rule)|json!({"id":id,"sourceId":rule.source_id,"selectorId":rule.selector_id,"groups":rule.groups})).collect();
    encode(
        &json!({"abiVersion":1,"retainedBytesKind":"shared-css-payload-lower-bound","selectors":selectors,"groups":groups,"rules":rules}),
    )
}

#[cfg(all(feature = "napi-bind", not(test)))]
mod native {
    use super::{CompiledHandle, PreparedHandle};
    use napi::Result;
    use napi_derive::napi;

    fn error(message: String) -> napi::Error {
        napi::Error::from_reason(message)
    }
    #[napi(js_name = "cssAbiVersion")]
    pub fn css_abi_version() -> u32 {
        1
    }
    #[napi(js_name = "cssBackendVersion")]
    pub fn css_backend_version() -> &'static str {
        env!("CARGO_PKG_VERSION")
    }
    #[napi(js_name = "cssUserAgentSourceV1")]
    pub fn css_user_agent_source_v1() -> Result<String> {
        super::user_agent_source_v1().map_err(error)
    }
    #[napi(js_name = "cssComputedPreferencesV1")]
    pub fn css_computed_preferences_v1(source: String) -> Result<String> {
        crate::css_preferences::preferences(&source).map_err(error)
    }
    #[napi(js_name = "cssRuntimePolicyV1")]
    pub fn css_runtime_policy_v1() -> Result<String> {
        crate::css_runtime_policy::runtime_policy().map_err(error)
    }
    #[napi(js_name = "cssComputedEnvironmentV1")]
    pub fn css_computed_environment_v1(source: String) -> Result<String> {
        crate::css_preferences::environment(&source).map_err(error)
    }
    #[napi(js_name = "serializeCssomV1")]
    pub fn serialize_cssom_v1(source: String) -> Result<String> {
        crate::css_serialize::serialize_batch(&source).map_err(error)
    }
    #[napi(js_name = "projectCssomPropertyV1")]
    pub fn project_cssom_property_v1(source: String) -> Result<String> {
        crate::css_used::project_property(&source).map_err(error)
    }
    #[napi(js_name = "measureSceneV1")]
    pub fn measure_scene_v1(request: String) -> Result<String> {
        super::measure_scene_v1(&request).map_err(error)
    }
    #[napi]
    pub struct CompiledStylesV1 {
        handle: CompiledHandle,
    }
    #[napi]
    impl CompiledStylesV1 {
        #[napi(constructor)]
        pub fn new(request: String) -> Result<Self> {
            Ok(Self {
                handle: CompiledHandle::new(&request).map_err(error)?,
            })
        }
        #[napi]
        pub fn metadata(&self) -> Result<String> {
            self.handle.metadata().map_err(error)
        }
        #[napi]
        pub fn prepare(&self, request: String) -> Result<PreparedStylesV1> {
            Ok(PreparedStylesV1 {
                handle: self.handle.prepare(&request).map_err(error)?,
            })
        }
        #[napi(js_name = "retainedBytes")]
        pub fn retained_bytes(&self) -> f64 {
            self.handle.retained_bytes()
        }
        #[napi]
        pub fn dispose(&mut self) {
            self.handle.dispose();
        }
    }
    #[napi]
    pub struct PreparedStylesV1 {
        handle: PreparedHandle,
    }
    #[napi]
    impl PreparedStylesV1 {
        #[napi]
        pub fn metadata(&self) -> Result<String> {
            self.handle.metadata().map_err(error)
        }
        #[napi(js_name = "storeMatchingV1")]
        pub fn store_matching_v1(&mut self, wire: String, limits: String) -> Result<String> {
            self.handle.store_matching(&wire, &limits).map_err(error)
        }
        #[napi(js_name = "resolveMatchingV1")]
        pub fn resolve_matching_v1(&self, wire: String, token: String) -> Result<String> {
            self.handle.resolve_matching(&wire, &token).map_err(error)
        }
        #[napi(js_name = "resolveBatch")]
        pub fn resolve_batch(&self, request: String) -> Result<String> {
            self.handle.resolve_batch(&request).map_err(error)
        }
        #[napi(js_name = "resolveContainerGeometry")]
        pub fn resolve_container_geometry(&self, request: String) -> Result<String> {
            self.handle
                .resolve_container_geometry(&request)
                .map_err(error)
        }
        #[napi(js_name = "retainedBytes")]
        pub fn retained_bytes(&self) -> f64 {
            self.handle.retained_bytes()
        }
        #[napi]
        pub fn dispose(&mut self) {
            self.handle.dispose();
        }
    }
}

#[cfg(feature = "wasm-bind")]
mod wasm {
    use super::{CompiledHandle, PreparedHandle};
    use wasm_bindgen::prelude::*;

    fn error(message: String) -> JsValue {
        JsValue::from_str(&message)
    }
    #[wasm_bindgen(js_name = cssAbiVersion)]
    pub fn css_abi_version() -> u32 {
        1
    }
    #[wasm_bindgen(js_name = cssBackendVersion)]
    pub fn css_backend_version() -> String {
        env!("CARGO_PKG_VERSION").to_owned()
    }
    #[wasm_bindgen(js_name=cssUserAgentSourceV1)]
    pub fn css_user_agent_source_v1() -> Result<String, JsValue> {
        super::user_agent_source_v1().map_err(|error| JsValue::from_str(&error))
    }
    #[wasm_bindgen(js_name=cssComputedPreferencesV1)]
    pub fn css_computed_preferences_v1(source: &str) -> Result<String, JsValue> {
        crate::css_preferences::preferences(source).map_err(error)
    }
    #[wasm_bindgen(js_name=cssRuntimePolicyV1)]
    pub fn css_runtime_policy_v1() -> Result<String, JsValue> {
        crate::css_runtime_policy::runtime_policy().map_err(error)
    }
    #[wasm_bindgen(js_name=cssComputedEnvironmentV1)]
    pub fn css_computed_environment_v1(source: &str) -> Result<String, JsValue> {
        crate::css_preferences::environment(source).map_err(error)
    }
    #[wasm_bindgen(js_name=serializeCssomV1)]
    pub fn serialize_cssom_v1(source: &str) -> Result<String, JsValue> {
        crate::css_serialize::serialize_batch(source).map_err(|error| JsValue::from_str(&error))
    }
    #[wasm_bindgen(js_name=projectCssomPropertyV1)]
    pub fn project_cssom_property_v1(source: &str) -> Result<String, JsValue> {
        crate::css_used::project_property(source).map_err(|error| JsValue::from_str(&error))
    }
    #[wasm_bindgen(js_name=measureSceneV1)]
    pub fn measure_scene_v1(request: String) -> Result<String, JsValue> {
        super::measure_scene_v1(&request).map_err(error)
    }
    #[wasm_bindgen]
    pub struct CompiledStylesV1 {
        handle: CompiledHandle,
    }
    #[wasm_bindgen]
    impl CompiledStylesV1 {
        #[wasm_bindgen(constructor)]
        pub fn new(request: String) -> Result<Self, JsValue> {
            Ok(Self {
                handle: CompiledHandle::new(&request).map_err(error)?,
            })
        }
        pub fn metadata(&self) -> Result<String, JsValue> {
            self.handle.metadata().map_err(error)
        }
        pub fn prepare(&self, request: String) -> Result<PreparedStylesV1, JsValue> {
            Ok(PreparedStylesV1 {
                handle: self.handle.prepare(&request).map_err(error)?,
            })
        }
        #[wasm_bindgen(js_name=retainedBytes)]
        pub fn retained_bytes(&self) -> f64 {
            self.handle.retained_bytes()
        }
        pub fn dispose(&mut self) {
            self.handle.dispose();
        }
    }
    #[wasm_bindgen]
    pub struct PreparedStylesV1 {
        handle: PreparedHandle,
    }
    #[wasm_bindgen]
    impl PreparedStylesV1 {
        pub fn metadata(&self) -> Result<String, JsValue> {
            self.handle.metadata().map_err(error)
        }
        #[wasm_bindgen(js_name=resolveBatch)]
        pub fn resolve_batch(&self, request: String) -> Result<String, JsValue> {
            self.handle.resolve_batch(&request).map_err(error)
        }
        #[wasm_bindgen(js_name=resolveContainerGeometry)]
        pub fn resolve_container_geometry(&self, request: String) -> Result<String, JsValue> {
            self.handle
                .resolve_container_geometry(&request)
                .map_err(error)
        }
        #[wasm_bindgen(js_name=retainedBytes)]
        pub fn retained_bytes(&self) -> f64 {
            self.handle.retained_bytes()
        }
        pub fn dispose(&mut self) {
            self.handle.dispose();
        }
    }
}

fn subject_seed(seed: Option<turbo_dom_core::rtdom::query::SubjectSeed<'_>>) -> Value {
    use turbo_dom_core::rtdom::query::SubjectSeed;
    let (kind, value) = match seed {
        Some(SubjectSeed::Id(value)) => ("id", value),
        Some(SubjectSeed::Class(value)) => ("class", value),
        Some(SubjectSeed::Type(value)) => ("type", value),
        None => return Value::Null,
    };
    json!({"kind":kind,"valueUnits":value.encode_utf16().collect::<Vec<_>>()})
}

#[cfg(test)]
mod tests {
    use super::*;
    fn computation() -> Value {
        json!({"maxNesting":32,"maxOutputUnits":8192,"maxTotalUnits":16384})
    }
    fn compiled(css: &str) -> CompiledHandle {
        let request = json!({"abiVersion":1,"sources":[{"id":7,"text":css.encode_utf16().collect::<Vec<_>>()}],"limits":{
            "sources":4,"sourceUnits":8192,"rules":128,"sourceRulePreludes":128,"selectors":128,
            "declarations":256,"properties":1024,"groupDepth":16,"tokenNesting":32,
        }});
        CompiledHandle::new(&request.to_string()).unwrap()
    }
    #[test]
    fn boundary_metadata_has_explicit_ordinary_and_composed_certificates() {
        for boundary in [
            ".root",
            ":scope",
            ":host",
            ":host-context(.outside)",
            "::part(control)",
            "::slotted(.light)",
        ] {
            let source = compiled(&format!("@scope({boundary}){{b{{color:red}}}}"));
            let metadata: Value = serde_json::from_str(&source.metadata().unwrap()).unwrap();
            let item = &metadata["groups"][0]["scope"]["startItems"][0];
            assert!(item.as_object().unwrap().contains_key("policy"));
            if boundary == ".root" || boundary == ":scope" {
                assert!(item["policy"].is_null());
            } else {
                assert!(item["policy"].is_object());
            }
        }
        let source = compiled("@scope(.root) to(:host-context(.stop)){b{color:red}}");
        let metadata: Value = serde_json::from_str(&source.metadata().unwrap()).unwrap();
        assert_eq!(
            metadata["groups"][0]["scope"]["endItems"][0]["policy"]["target"]["kind"],
            "unsupported_composed"
        );
    }
    fn prepared(compiled: &CompiledHandle) -> PreparedHandle {
        let request = json!({"abiVersion":1,"uses":[{"sourceId":7,"origin":"author","treeScope":0,"scopeDepth":0,"disabled":false,"media":null}],"environment":{
            "mediaType":"screen","features":{},"unitBases":[["em",16.0],["rem",16.0]],"selectorSupport":{},"stylePhase":"steady"
        },"limits":{"uses":4,"rules":128,"layers":128,"computation":computation()}});
        compiled.prepare(&request.to_string()).unwrap()
    }
    fn limits() -> Value {
        json!({"targets":8,"answerRows":128,"answerIds":1024,"pathNodes":128,"inputUnits":8192,"retainedResultUnits":16384,"matchingDecisions":1024,"inlineDeclarations":128,"inlineProperties":1024,"customRollbacks":128,"valueRetainedUnits":8192,"computation":computation()})
    }
    fn target(id: u32) -> Value {
        json!({"id":id,"parentId":null,"path":[id,1],"implicitScopeRoot":1,"treeScope":0,"scopeDepth":0,"pseudo":null,"inline":null,"context":{"initialColor":"rgb(0, 0, 0)","initialFontFamily":"serif","initialFontSize":"16px","unitBases":[["rem",16.0]],"fontSizeUnitBases":[],"isRoot":false}})
    }
    fn request(id: u32, color: bool) -> Value {
        json!({"abiVersion":1,"targets":[target(id)],"domains":[{"id":0,"nodeIds":[id]}],"selectorAnswers":[{"selectorId":0,"itemId":0,"rootId":null,"domainId":0,"matches":if color {vec![id]} else {vec![]}}],"scopeAnswers":[],"limits":limits()})
    }
    #[test]
    fn renderer_preferences_do_not_materialize_public_cssom_keys() {
        let source = compiled(".x { color: red }");
        let prepared = prepared(&source);
        let mut input = request(2, false);
        input["targets"][0]["context"]["initialFontFamily"] = json!("monospace");
        let output: Value =
            serde_json::from_str(&prepared.resolve_batch(&input.to_string()).unwrap()).unwrap();
        let row = &output["targets"][0];
        assert_eq!(
            row["rendererValues"]["font-family"],
            json!("monospace".encode_utf16().collect::<Vec<_>>())
        );
        assert_eq!(
            row["rendererValues"]["color"],
            json!("rgb(0, 0, 0)".encode_utf16().collect::<Vec<_>>())
        );
        assert!(row["values"].get("font-family").is_none());
        assert!(row["values"].get("color").is_none());
        assert_eq!(row["publicKeys"], json!([]));
    }
    #[test]
    fn renderer_inherited_family_wins_over_child_initial_context() {
        let source = compiled(".x { color: red }");
        let prepared = prepared(&source);
        let mut input = request(2, false);
        input["targets"][0]["inline"] =
            json!("font-family: monospace".encode_utf16().collect::<Vec<_>>());
        let mut child = target(3);
        child["parentId"] = json!(2);
        child["context"]["initialFontFamily"] = json!("Times");
        input["targets"].as_array_mut().unwrap().push(child);
        input["domains"][0]["nodeIds"] = json!([2, 3]);
        let output: Value =
            serde_json::from_str(&prepared.resolve_batch(&input.to_string()).unwrap()).unwrap();
        assert_eq!(
            output["targets"][1]["rendererValues"]["font-family"],
            json!("monospace".encode_utf16().collect::<Vec<_>>())
        );
        input["targets"][1]["inline"] =
            json!("font-family: initial".encode_utf16().collect::<Vec<_>>());
        let output: Value =
            serde_json::from_str(&prepared.resolve_batch(&input.to_string()).unwrap()).unwrap();
        assert_eq!(
            output["targets"][1]["rendererValues"]["font-family"],
            json!("Times".encode_utf16().collect::<Vec<_>>())
        );
        input["targets"][1]["inline"] =
            json!("font-family: unset".encode_utf16().collect::<Vec<_>>());
        let output: Value =
            serde_json::from_str(&prepared.resolve_batch(&input.to_string()).unwrap()).unwrap();
        assert_eq!(
            output["targets"][1]["rendererValues"]["font-family"],
            json!("monospace".encode_utf16().collect::<Vec<_>>())
        );
    }
    #[test]
    fn normalized_initial_context_is_inherited_before_relative_lengths() {
        let source = compiled(".x { color: red }");
        let prepared = prepared(&source);
        let mut input = request(2, false);
        input["targets"][0]["context"]["initialFontSize"] = json!("10000px");
        input["targets"][0]["context"]["unitBases"] = json!([["rem", 10000.0]]);
        let mut child = target(3);
        child["parentId"] = json!(2);
        child["inline"] = json!("font-size: inherit; width: 2em"
            .encode_utf16()
            .collect::<Vec<_>>());
        input["targets"].as_array_mut().unwrap().push(child);
        input["domains"][0]["nodeIds"] = json!([2, 3]);
        let output: Value =
            serde_json::from_str(&prepared.resolve_batch(&input.to_string()).unwrap()).unwrap();
        assert_eq!(output["targets"][0]["fontSizePx"], 10000.0);
        assert_eq!(output["targets"][1]["fontSizePx"], 10000.0);
        assert_eq!(
            output["targets"][1]["values"]["width"],
            json!("20000px".encode_utf16().collect::<Vec<_>>())
        );
    }
    fn profile_font_roundtrip(size: f64) -> (Value, Value) {
        let profile: Value = serde_json::from_str(&crate::css_preferences::preferences(&json!({
            "abiVersion":1,"hostPlatform":"macos","fontOverride":{"family":[65],"sizePx":size},
            "limits":{"inputUnits":1024,"retainedUnits":100_000,"computation":computation()}
        }).to_string()).unwrap()).unwrap();
        let units: Vec<u16> =
            serde_json::from_value(profile["initial"]["fontSizeInput"].clone()).unwrap();
        let initial = String::from_utf16(&units).unwrap();
        let source = compiled(".x { color: red }");
        let prepared = prepared(&source);
        let mut input = request(2, false);
        input["targets"][0]["context"]["initialFontSize"] = json!(initial);
        let mut child = target(3);
        child["parentId"] = json!(2);
        child["inline"] = json!("font-size: inherit; width: 2em"
            .encode_utf16()
            .collect::<Vec<_>>());
        input["targets"].as_array_mut().unwrap().push(child);
        input["domains"][0]["nodeIds"] = json!([2, 3]);
        let output: Value =
            serde_json::from_str(&prepared.resolve_batch(&input.to_string()).unwrap()).unwrap();
        eprintln!(
            "PROFILE_FONT_ROUNDTRIP {}",
            json!({"raw":size,"profile":profile["initial"],"parent":output["targets"][0]["fontSizePx"],"child":output["targets"][1]["fontSizePx"],"width":output["targets"][1]["values"]["width"]})
        );
        (profile, output)
    }
    #[test]
    fn fractional_profile_basis_survives_root_initial_context_and_inherited_em() {
        let (profile, output) = profile_font_roundtrip(16.123_456_789);
        assert_eq!(
            output["targets"][0]["fontSizePx"],
            profile["initial"]["computedFontSizePx"]
        );
        assert_eq!(
            output["targets"][1]["fontSizePx"],
            profile["initial"]["computedFontSizePx"]
        );
        assert_eq!(
            output["targets"][1]["values"]["width"],
            json!("32.2469px".encode_utf16().collect::<Vec<_>>())
        );
    }
    #[test]
    fn tiny_finite_profile_basis_survives_root_initial_context_and_inherited_em() {
        let (profile, output) = profile_font_roundtrip(1e-300);
        assert_eq!(
            output["targets"][0]["fontSizePx"],
            profile["initial"]["computedFontSizePx"]
        );
        assert_eq!(
            output["targets"][1]["fontSizePx"],
            profile["initial"]["computedFontSizePx"]
        );
        assert_eq!(
            output["targets"][1]["values"]["width"],
            json!("0px".encode_utf16().collect::<Vec<_>>())
        );
    }
    #[test]
    fn ua_source_is_the_canonical_layout_source_without_a_second_table() {
        let data: Value = serde_json::from_str(&user_agent_source_v1().unwrap()).unwrap();
        let units = |name: &str| {
            data[name]
                .as_array()
                .unwrap()
                .iter()
                .map(|value| u16::try_from(value.as_u64().unwrap()).unwrap())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            String::from_utf16(&units("text")).unwrap(),
            turbo_dom_layout::style::user_agent_css()
        );
        assert_eq!(
            String::from_utf16(&units("documentText")).unwrap(),
            turbo_dom_layout::style::DOCUMENT_USER_AGENT_CSS
        );
        assert_eq!(
            String::from_utf16(&units("fullscreenText")).unwrap(),
            turbo_dom_layout::style::fullscreen_user_agent_css()
        );
    }
    #[test]
    fn container_geometry_projection_and_partial_native_facts_remain_separate_from_final_styles() {
        let compiled = compiled(".x{color:red}@container box (min-width:300px){.x{color:blue}}");
        let prepared = prepared(&compiled);
        let mut input = request(2, true);
        input["targets"][0]["path"] = json!([2, 3, 1]);
        input["selectorAnswers"]
            .as_array_mut()
            .unwrap()
            .push(json!({"selectorId":1,"itemId":0,"rootId":null,"domainId":0,"matches":[2]}));
        assert!(prepared
            .resolve_batch(&input.to_string())
            .unwrap_err()
            .contains("RequiresContainer"));
        let geometry: Value = serde_json::from_str(
            &prepared
                .resolve_container_geometry(&input.to_string())
                .unwrap(),
        )
        .unwrap();
        assert_eq!(geometry["geometryOnly"], true);
        assert_eq!(
            geometry["targets"][0]["values"]["color"],
            json!("rgb(255, 0, 0)".encode_utf16().collect::<Vec<_>>())
        );
        input["targets"][0]["containerAncestors"] = json!([{"id":3,"containerName":"box".encode_utf16().collect::<Vec<_>>(),"containerType":"inline-size".encode_utf16().collect::<Vec<_>>(),"writingMode":null,"unitBases":[["em",16.0]],"widthPx":400.0,"heightPx":null}]);
        let full: Value =
            serde_json::from_str(&prepared.resolve_batch(&input.to_string()).unwrap()).unwrap();
        assert_eq!(full["geometryOnly"], false);
        assert_eq!(
            full["targets"][0]["values"]["color"],
            json!("rgb(0, 0, 255)".encode_utf16().collect::<Vec<_>>())
        );
        input["targets"][0]["containerAncestors"][0]["widthPx"] = json!(0);
        let zero: Value =
            serde_json::from_str(&prepared.resolve_batch(&input.to_string()).unwrap()).unwrap();
        assert_eq!(
            zero["targets"][0]["values"]["color"],
            geometry["targets"][0]["values"]["color"]
        );
        input["targets"][0]["containerAncestors"][0]["widthPx"] = Value::Null;
        assert!(prepared
            .resolve_batch(&input.to_string())
            .unwrap_err()
            .contains("RequiresMeasurement(3)"));
        let duplicate = input["targets"][0]["containerAncestors"][0].clone();
        input["targets"][0]["containerAncestors"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        assert!(prepared
            .resolve_batch(&input.to_string())
            .unwrap_err()
            .contains("DuplicateContainerAncestor"));
    }
    #[test]
    fn pseudo_rows_keep_original_dom_subject_identity_and_separate_style_parents() {
        let compiled = compiled(".x{color:red}.x::before{color:blue}");
        let prepared = prepared(&compiled);
        let mut input = request(2, true);
        let mut pseudo = target(3);
        pseudo["subjectId"] = json!(2);
        pseudo["parentId"] = json!(2);
        pseudo["path"] = json!([2, 1]);
        pseudo["pseudo"] = json!("before");
        input["targets"].as_array_mut().unwrap().push(pseudo);
        input["contextRoots"] =
            json!([{"treeScope":0,"root":1,"host":null,"parentTreeScope":null}]);
        input["limits"]["routingContexts"] = json!(2);
        input["selectorAnswers"]
            .as_array_mut()
            .unwrap()
            .push(json!({"selectorId":1,"itemId":0,"rootId":null,"domainId":0,"matches":[2]}));
        let result: Value =
            serde_json::from_str(&prepared.resolve_batch(&input.to_string()).unwrap()).unwrap();
        assert_eq!(
            result["targets"][0]["values"]["color"],
            json!("rgb(255, 0, 0)".encode_utf16().collect::<Vec<_>>())
        );
        assert_eq!(
            result["targets"][1]["values"]["color"],
            json!("rgb(0, 0, 255)".encode_utf16().collect::<Vec<_>>())
        );
        input["targets"][1]
            .as_object_mut()
            .unwrap()
            .remove("subjectId");
        assert!(prepared
            .resolve_batch(&input.to_string())
            .unwrap_err()
            .contains("InvalidPath"));
    }
    #[test]
    fn bulk_original_context_facts_are_validated_and_budgeted() {
        let compiled = compiled(".x{color:red}");
        let prepared = prepared(&compiled);
        let mut input = request(2, true);
        input["contextRoots"] =
            json!([{"treeScope":0,"root":1,"host":null,"parentTreeScope":null}]);
        assert!(prepared
            .resolve_batch(&input.to_string())
            .unwrap_err()
            .contains("ResourceLimit"));
        input["limits"]["routingContexts"] = json!(1);
        let result: Value =
            serde_json::from_str(&prepared.resolve_batch(&input.to_string()).unwrap()).unwrap();
        assert_eq!(
            result["targets"][0]["values"]["color"],
            json!("rgb(255, 0, 0)".encode_utf16().collect::<Vec<_>>())
        );
        input["contextRoots"][0]["root"] = json!(99);
        assert!(prepared
            .resolve_batch(&input.to_string())
            .unwrap_err()
            .contains("InvalidPath"));
    }
    #[test]
    fn identical_retained_requests_need_one_native_cascade() {
        let compiled = compiled(".x{color:red}");
        let mut owner = prepared(&compiled);
        let request = request(2, true);
        let facts = json!({"domains":request["domains"],"selectorAnswers":request["selectorAnswers"],"scopeAnswers":[]});
        let input =
            json!({"abiVersion":1,"targets":request["targets"],"limits":limits()}).to_string();
        let token = owner
            .store_matching(&facts.to_string(), &limits().to_string())
            .unwrap();
        let first = owner.resolve_matching(&input, &token).unwrap();
        assert_eq!(owner.resolve_matching(&input, &token).unwrap(), first);
        assert_eq!(owner.matching.as_ref().unwrap().cascade_calls(), 1);
    }
    fn retained_fixture() -> (PreparedHandle, String, Value) {
        let compiled = compiled(".x{color:red}");
        let mut owner = prepared(&compiled);
        let mut input = request(2, true);
        input["contextRoots"] =
            json!([{"treeScope":0,"root":1,"host":null,"parentTreeScope":null}]);
        input["limits"]["routingContexts"] = json!(1);
        let facts = json!({"domains":input["domains"],"selectorAnswers":input["selectorAnswers"],"scopeAnswers":[],"contextRoots":input["contextRoots"]});
        let token = owner
            .store_matching(&facts.to_string(), &input["limits"].to_string())
            .unwrap();
        (owner, token, input)
    }
    fn retained_input(input: &Value) -> String {
        json!({"abiVersion":input["abiVersion"],"targets":input["targets"],"limits":input["limits"]}).to_string()
    }
    #[test]
    fn retained_result_budget_errors_are_fresh_and_not_memoized() {
        let (owner, token, input) = retained_fixture();
        let wire = retained_input(&input);
        owner.resolve_matching(&wire, &token).unwrap();
        for name in [
            "targets",
            "answerRows",
            "answerIds",
            "pathNodes",
            "routingContexts",
            "inputUnits",
            "retainedResultUnits",
            "matchingDecisions",
            "valueRetainedUnits",
        ] {
            let mut bad = input.clone();
            bad["limits"][name] = json!(0);
            let expected = owner.resolve_batch(&bad.to_string()).unwrap_err();
            assert_eq!(
                owner
                    .resolve_matching(&retained_input(&bad), &token)
                    .unwrap_err(),
                expected,
                "{name}"
            );
        }
        for name in ["maxOutputUnits", "maxTotalUnits"] {
            let mut bad = input.clone();
            bad["limits"]["computation"][name] = json!(0);
            let expected = owner.resolve_batch(&bad.to_string()).unwrap_err();
            assert_eq!(
                owner
                    .resolve_matching(&retained_input(&bad), &token)
                    .unwrap_err(),
                expected,
                "{name}"
            );
        }
        let mut bad = input.clone();
        bad["limits"]["answerRows"] = json!(0);
        let before = owner.matching.as_ref().unwrap().cascade_calls();
        for _ in 0..2 {
            assert!(owner
                .resolve_matching(&retained_input(&bad), &token)
                .is_err());
        }
        assert_eq!(owner.matching.as_ref().unwrap().cascade_calls(), before + 2);
        let before = owner.matching.as_ref().unwrap().cascade_calls();
        assert!(owner.resolve_matching("invalid-json", &token).is_err());
        bad["abiVersion"] = json!(2);
        assert!(owner
            .resolve_matching(&retained_input(&bad), &token)
            .unwrap_err()
            .contains("AbiVersion"));
        assert_eq!(owner.matching.as_ref().unwrap().cascade_calls(), before);
        assert_eq!(
            owner.resolve_matching(&wire, &token).unwrap(),
            owner.resolve_batch(&input.to_string()).unwrap()
        );
    }
    #[test]
    fn retained_result_exact_inputs_and_owner_lifetime_are_preserved() {
        let (mut owner, token, mut input) = retained_fixture();
        let base_bytes = owner.retained_bytes();
        let first = owner
            .resolve_matching(&retained_input(&input), &token)
            .unwrap();
        assert_eq!(
            owner.retained_bytes() as usize,
            base_bytes as usize + retained_input(&input).len() + first.len()
        );
        input["targets"][0]["inline"] = json!("color:blue".encode_utf16().collect::<Vec<_>>());
        let second = owner
            .resolve_matching(&retained_input(&input), &token)
            .unwrap();
        assert_ne!(first, second);
        assert_eq!(second, owner.resolve_batch(&input.to_string()).unwrap());
        assert_eq!(
            owner
                .resolve_matching(&retained_input(&input), &token)
                .unwrap(),
            second
        );
        assert_eq!(owner.matching.as_ref().unwrap().cascade_calls(), 2);
        let (other, other_token, _) = retained_fixture();
        assert!(other
            .resolve_matching(&retained_input(&input), &token)
            .unwrap_err()
            .contains("StaleMatching"));
        assert!(owner
            .resolve_matching(&retained_input(&input), &other_token)
            .unwrap_err()
            .contains("StaleMatching"));
        let empty = json!({"domains":[],"selectorAnswers":[],"scopeAnswers":[]});
        owner
            .store_matching(&empty.to_string(), &limits().to_string())
            .unwrap();
        assert!(owner.retained_bytes() < base_bytes + first.len() as f64);
        assert!(owner
            .resolve_matching(&retained_input(&input), &token)
            .unwrap_err()
            .contains("StaleMatching"));
        owner.dispose();
        assert_eq!(owner.retained_bytes(), 0.0);
        assert!(owner
            .resolve_matching(&retained_input(&input), &token)
            .unwrap_err()
            .contains("Disposed"));
    }
    #[test]
    fn oversized_result_request_is_never_retained() {
        let (owner, token, input) = retained_fixture();
        let before = owner.retained_bytes();
        let wire = " ".repeat(64 * 1024) + &retained_input(&input);
        let first = owner.resolve_matching(&wire, &token).unwrap();
        assert_eq!(owner.resolve_matching(&wire, &token).unwrap(), first);
        assert_eq!(owner.matching.as_ref().unwrap().cascade_calls(), 2);
        assert_eq!(owner.retained_bytes(), before);
    }
    #[test]
    fn oversized_result_response_is_never_retained() {
        let (owner, token, mut input) = retained_fixture();
        let before = owner.retained_bytes();
        input["targets"][0]["inline"] = json!(("--blob:".to_owned() + &"x".repeat(20000))
            .encode_utf16()
            .collect::<Vec<_>>());
        for name in ["inputUnits", "retainedResultUnits", "valueRetainedUnits"] {
            input["limits"][name] = json!(60000);
        }
        input["limits"]["computation"]["maxOutputUnits"] = json!(60000);
        input["limits"]["computation"]["maxTotalUnits"] = json!(120_000);
        let wire = retained_input(&input);
        let first = owner.resolve_matching(&wire, &token).unwrap();
        assert!(first.len() > 64 * 1024);
        assert_eq!(owner.resolve_matching(&wire, &token).unwrap(), first);
        assert_eq!(owner.matching.as_ref().unwrap().cascade_calls(), 2);
        assert_eq!(owner.retained_bytes(), before);
    }
    #[test]
    fn retained_result_can_be_read_on_serial_scoped_threads() {
        let (owner, token, input) = retained_fixture();
        let wire = retained_input(&input);
        std::thread::scope(|scope| {
            let first = scope
                .spawn(|| owner.resolve_matching(&wire, &token).unwrap())
                .join()
                .unwrap();
            let second = scope
                .spawn(|| owner.resolve_matching(&wire, &token).unwrap())
                .join()
                .unwrap();
            assert_eq!(first, second);
        });
        assert_eq!(owner.matching.as_ref().unwrap().cascade_calls(), 1);
    }
    #[test]
    fn retained_matching_requires_exact_owner_and_checks_each_read_budget() {
        let compiled = compiled(".x{color:red}");
        let mut owner = prepared(&compiled);
        let other = prepared(&compiled);
        let request = request(2, true);
        let facts = json!({"domains":request["domains"],"selectorAnswers":request["selectorAnswers"],"scopeAnswers":[]});
        let mut input = json!({"abiVersion":1,"targets":request["targets"],"limits":limits()});
        let baseline = owner.retained_bytes();
        let token = owner
            .store_matching(&facts.to_string(), &limits().to_string())
            .unwrap();
        assert!(owner.retained_bytes() > baseline);
        assert_eq!(
            owner.resolve_matching(&input.to_string(), &token).unwrap(),
            owner.resolve_batch(&request.to_string()).unwrap()
        );
        assert!(other.resolve_matching(&input.to_string(), &token).is_err());
        let mut bad = facts.clone();
        bad["selectorAnswers"][0]["matches"] = json!([999]);
        assert!(owner
            .store_matching(&bad.to_string(), &limits().to_string())
            .unwrap_err()
            .contains("OutsideDomain"));
        let large = " ".repeat(256 * 1024 + 1);
        assert!(owner
            .store_matching(&large, &limits().to_string())
            .unwrap_err()
            .contains("ResourceLimit"));
        for name in ["answerRows", "answerIds", "targets", "pathNodes"] {
            input["limits"] = limits();
            input["limits"][name] = json!(0);
            assert!(owner
                .resolve_matching(&input.to_string(), &token)
                .unwrap_err()
                .contains("ResourceLimit"));
        }
        input["limits"] = limits();
        let empty = json!({"domains":[],"selectorAnswers":[],"scopeAnswers":[]});
        let retained = owner.retained_bytes();
        let next = owner
            .store_matching(&empty.to_string(), &limits().to_string())
            .unwrap();
        assert!(owner.retained_bytes() < retained);
        assert!(owner
            .resolve_matching(&input.to_string(), &token)
            .unwrap_err()
            .contains("StaleMatching"));
        owner.dispose();
        assert_eq!(owner.retained_bytes(), 0.0);
        assert!(owner
            .resolve_matching(&input.to_string(), &next)
            .unwrap_err()
            .contains("Disposed"));
    }
    #[test]
    fn immutable_models_resolve_original_units_and_survive_other_owner_disposal() {
        let mut compiled = compiled(".x{--color:red;color:var(--color);width:2em}");
        let metadata: Value = serde_json::from_str(&compiled.metadata().unwrap()).unwrap();
        assert_eq!(
            metadata["retainedBytesKind"],
            "shared-css-payload-lower-bound"
        );
        assert!(compiled.retained_bytes() > 0.0);
        let mut prepared = prepared(&compiled);
        compiled.dispose();
        assert_eq!(compiled.retained_bytes(), 0.0);
        assert!(compiled
            .metadata()
            .unwrap_err()
            .starts_with("CSSBackendDisposed:"));
        let request = request(2, true);
        let result: Value =
            serde_json::from_str(&prepared.resolve_batch(&request.to_string()).unwrap()).unwrap();
        let values = &result["targets"][0]["values"];
        assert_eq!(
            values["color"],
            json!("rgb(255, 0, 0)".encode_utf16().collect::<Vec<_>>())
        );
        assert_eq!(
            values["width"],
            json!("32px".encode_utf16().collect::<Vec<_>>())
        );
        assert!(prepared.retained_bytes() > 0.0);
        assert_eq!(
            serde_json::from_str::<Value>(&prepared.metadata().unwrap()).unwrap()["abiVersion"],
            1
        );
        prepared.dispose();
        assert_eq!(prepared.retained_bytes(), 0.0);
        assert!(prepared
            .resolve_batch(&request.to_string())
            .unwrap_err()
            .starts_with("CSSBackendDisposed:"));
    }
    #[test]
    fn transport_errors_and_real_limits_are_not_converted_to_empty_styles() {
        assert!(CompiledHandle::new("not json")
            .err()
            .unwrap()
            .starts_with("CSSBackendTransport:"));
        assert!(abi(2).is_err());
        let compiled = compiled(".x{color:red}");
        let prepared = prepared(&compiled);
        let mut request = request(2, false);
        request["limits"]["targets"] = json!(0);
        assert_eq!(
            prepared.resolve_batch(&request.to_string()).unwrap_err(),
            "CSSBackendResolve:ResourceLimit"
        );
        request["limits"] = limits();
        request["targets"][0]["parentId"] = json!(99);
        assert_eq!(
            prepared.resolve_batch(&request.to_string()).unwrap_err(),
            "CSSBackendResolve:MissingParent"
        );
    }
    #[test]
    fn declared_marker_and_public_keys_survive_disposal_with_exact_transport_quota() {
        let compiled = compiled(".x{text-decoration:underline dotted red}");
        let mut prepared = prepared(&compiled);
        let mut input = request(2, true);
        let output: Value =
            serde_json::from_str(&prepared.resolve_batch(&input.to_string()).unwrap()).unwrap();
        let row = &output["targets"][0];
        assert!(row["publicKeys"]
            .as_array()
            .unwrap()
            .contains(&json!("text-decoration-line")));
        let marker = &row["declaredTextDecoration"];
        assert_eq!(marker["origin"], "author");
        assert_eq!(marker["important"], false);
        assert_eq!(marker["inline"], false);
        assert_eq!(
            marker["value"],
            json!("underline dotted red".encode_utf16().collect::<Vec<_>>())
        );
        let mut units = 0;
        for field in ["values", "rendererValues", "custom"] {
            for (name, value) in row[field].as_object().unwrap() {
                units += name.encode_utf16().count() + value.as_array().unwrap().len();
            }
        }
        units += row["publicKeys"]
            .as_array()
            .unwrap()
            .iter()
            .map(|name| name.as_str().unwrap().encode_utf16().count())
            .sum::<usize>();
        units += marker["value"].as_array().unwrap().len() + "author".len();
        input["limits"]["retainedResultUnits"] = json!(units);
        assert_eq!(
            serde_json::from_str::<Value>(&prepared.resolve_batch(&input.to_string()).unwrap())
                .unwrap(),
            output
        );
        input["limits"]["retainedResultUnits"] = json!(units - 1);
        assert_eq!(
            prepared.resolve_batch(&input.to_string()).unwrap_err(),
            "CSSBackendResolve:ResourceLimit"
        );
        prepared.dispose();
        assert_eq!(
            marker["value"],
            json!("underline dotted red".encode_utf16().collect::<Vec<_>>())
        );
    }
    #[test]
    fn empty_scene_request_retains_world_units_without_touching_a_font_provider() {
        let source = json!({"abiVersion":1,"scene":{"world":[55296,0,57344],"viewport":{"width":800,"height":600},"fonts":"bundled_html","limits":{"maxNodes":0,"maxDepth":0,"maxPathUnits":0,"maxTextUnits":0,"maxAttributes":0,"maxAttributeUnits":0,"maxProperties":0,"maxStyleUnits":0,"maxWorldUnits":3,"maxRequests":0},"nodes":[],"requested":[]}});
        let result: Value =
            serde_json::from_str(&measure_scene_v1(&source.to_string()).unwrap()).unwrap();
        assert_eq!(result["world"], json!([55296, 0, 57344]));
    }
}
