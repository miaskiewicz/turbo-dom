//! Runtime resource policy, not CSS validity or Chrome compatibility thresholds.
//! The default JS assembler consumes these values without inventing quotas.
use serde_json::{json, Value};
use turbo_dom_layout::live::{
    MAX_ATTRIBUTE_BYTES, MAX_DECLARATIONS, MAX_DEPTH, MAX_NODES, MAX_TEXT_BYTES,
};

// Mirrors the existing native-source envelope in layout_snapshot/text.rs.
// Here it is an explicit UTF-16 backing-store allowance: two bytes per unit.
const SOURCE_ENVELOPE_BYTES: usize = 16 * 1024 * 1024;
const UTF16_BYTES: usize = 2;
// Existing browser-env animation_tokens.rs policy; no V8 dependency is needed.
const TOKEN_UNITS: usize = 1_048_576;
const TOKEN_NESTING: usize = 16;

fn arithmetic() -> String {
    "CSSBackendRuntimePolicy:ResourceLimit".into()
}
fn product(left: usize, right: usize) -> Result<usize, String> {
    left.checked_mul(right).ok_or_else(arithmetic)
}
fn computation() -> Value {
    json!({"maxNesting":TOKEN_NESTING,"maxOutputUnits":TOKEN_UNITS,"maxTotalUnits":TOKEN_UNITS})
}
fn primitive_policy(units: usize) -> Value {
    json!({"inputUnits":units,"retainedUnits":units,"computation":computation()})
}
struct Bounds {
    units: usize,
    paths: usize,
    edges: usize,
}
impl Bounds {
    fn new() -> Result<Self, String> {
        let envelope = product(SOURCE_ENVELOPE_BYTES / UTF16_BYTES, UTF16_BYTES)?;
        // An aggregate serialized-u32 allowance, NOT a per-node path bound.
        // Physical, context-root and assigned-slot path charges share it.
        let ids = envelope / std::mem::size_of::<u32>();
        Ok(Self {
            units: envelope / UTF16_BYTES,
            paths: ids,
            edges: ids,
        })
    }
    fn snapshot(&self) -> Value {
        json!({"maxNodes":MAX_NODES,"maxDepth":MAX_DEPTH,"maxPathUnits":self.paths,
            "maxTextUnits":MAX_TEXT_BYTES,"maxAttributes":MAX_ATTRIBUTE_BYTES,
            "maxAttributeUnits":MAX_ATTRIBUTE_BYTES})
    }
    fn scene(&self) -> Value {
        json!({"maxNodes":MAX_NODES,"maxDepth":MAX_DEPTH,"maxPathUnits":self.paths,
            "maxTextUnits":MAX_TEXT_BYTES,"maxAttributes":MAX_ATTRIBUTE_BYTES,
            "maxAttributeUnits":MAX_ATTRIBUTE_BYTES,"maxProperties":MAX_DECLARATIONS,
            "maxStyleUnits":self.units,"maxWorldUnits":self.units,"maxRequests":MAX_NODES})
    }
    fn compile(&self) -> Value {
        json!({"sources":MAX_NODES,"sourceUnits":self.units,"rules":self.units,
            "sourceRulePreludes":self.units,"selectors":self.units,
            "declarations":MAX_DECLARATIONS,"properties":MAX_DECLARATIONS,
            "groupDepth":TOKEN_NESTING,"tokenNesting":TOKEN_NESTING})
    }
    fn resolve(&self) -> Value {
        json!({"targets":MAX_NODES,"answerRows":self.units,"answerIds":self.units,
            "pathNodes":self.paths,"inputUnits":self.units,"retainedResultUnits":self.units,
            "matchingDecisions":self.units,"routingContexts":MAX_NODES,
            "inlineDeclarations":MAX_DECLARATIONS,"inlineProperties":MAX_DECLARATIONS,
            "customRollbacks":MAX_DECLARATIONS,"valueRetainedUnits":self.units,
            "computation":computation()})
    }
    fn adapter(&self) -> Value {
        let mut serialize = primitive_policy(self.units);
        serialize["targets"] = json!(MAX_NODES);
        json!({"compile":self.compile(),
            "prepare":{"uses":MAX_NODES,"rules":self.units,"layers":self.units,"computation":computation()},
            "matching":{"maxNodes":MAX_NODES,"maxAnswers":self.units,"maxAncestorDepth":MAX_DEPTH},
            "contextualMatching":{"maxContexts":MAX_NODES,"maxNodes":MAX_NODES,
                "maxAncestorDepth":MAX_DEPTH,"maxAnswers":self.units,"maxDomainUnits":self.units,
                "routing":{"maxRoots":MAX_NODES,"maxNodes":MAX_NODES,"maxPathUnits":self.paths,"maxScopeDepth":MAX_DEPTH}},
            "resolve":self.resolve(),"serialize":serialize,"property":primitive_policy(self.units),
            "providerSnapshot":{"maxDepth":MAX_DEPTH,"maxEntries":self.units}})
    }
}
fn policy(fonts: &str) -> Result<Value, String> {
    let bounds = Bounds::new()?;
    Ok(
        json!({"abiVersion":1,"preferences":primitive_policy(bounds.units),
        "environment":primitive_policy(bounds.units),"snapshot":bounds.snapshot(),
        "selectorFacts":{"maxNodes":MAX_NODES,"maxEdges":bounds.edges,
            "maxTextUnits":bounds.units,"maxAttributeUnits":MAX_ATTRIBUTE_BYTES},
        "adapter":bounds.adapter(),"scene":bounds.scene(),
        "composition":{"maxRoots":MAX_NODES,"maxNodes":MAX_NODES,"maxDepth":MAX_DEPTH,
            "maxEdges":bounds.edges,"maxPathUnits":bounds.paths},"fonts":fonts}),
    )
}
pub(super) fn runtime_policy() -> Result<String, String> {
    let fonts = if cfg!(target_arch = "wasm32") {
        "bundled_html"
    } else {
        "native_html_system"
    };
    serde_json::to_string(&policy(fonts)?).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_and_wasm_have_identical_units_and_distinct_actual_font_capability() {
        let mut native = policy("native_html_system").unwrap();
        let mut wasm = policy("bundled_html").unwrap();
        assert_eq!(native["fonts"], "native_html_system");
        assert_eq!(wasm["fonts"], "bundled_html");
        native.as_object_mut().unwrap().remove("fonts");
        wasm.as_object_mut().unwrap().remove("fonts");
        assert_eq!(native, wasm);
    }
    #[test]
    fn existing_caps_and_utf16_backing_envelope_are_explicit() {
        let policy: Value = serde_json::from_str(&runtime_policy().unwrap()).unwrap();
        println!("RUNTIME_POLICY_JSON {policy}");
        assert_eq!(policy["abiVersion"], 1);
        assert_eq!(policy["snapshot"]["maxNodes"], MAX_NODES);
        assert_eq!(policy["snapshot"]["maxTextUnits"], MAX_TEXT_BYTES);
        assert_eq!(
            policy["adapter"]["compile"]["sourceUnits"],
            SOURCE_ENVELOPE_BYTES / 2
        );
        assert_eq!(policy["adapter"]["resolve"]["computation"], computation());
        assert_eq!(policy["adapter"]["serialize"]["targets"], MAX_NODES);
        assert_eq!(
            policy["scene"]["maxPathUnits"],
            SOURCE_ENVELOPE_BYTES / std::mem::size_of::<u32>()
        );
    }
    #[test]
    fn checked_derived_counts_reject_overflow_and_preserve_exact_boundaries() {
        assert_eq!(product(usize::MAX, 1).unwrap(), usize::MAX);
        assert!(product(usize::MAX, 2)
            .unwrap_err()
            .contains("ResourceLimit"));
        assert_eq!(product(0, usize::MAX).unwrap(), 0);
    }
}
