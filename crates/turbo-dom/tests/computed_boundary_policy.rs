//! Boundary capability certificates reuse the ordinary immutable selector inspector.
#![cfg(feature = "computed-style-grammar")]
use turbo_dom::rtdom::computed_css::{
    model::{CompileError, CompileLimits, CompiledStyles, SheetSource},
    selector_policy::{SelectorPolicyInspection, SelectorTarget},
    CssText,
};
fn limits(source_units: usize) -> CompileLimits {
    CompileLimits {
        sources: 1,
        source_units,
        rules: 16,
        source_rule_preludes: 16,
        selectors: 16,
        declarations: 16,
        properties: 64,
        group_depth: 8,
        token_nesting: 32,
    }
}
fn compile(source: &str) -> CompiledStyles {
    CompiledStyles::compile(
        &[SheetSource {
            id: 1,
            text: CssText::from_units(source.encode_utf16().collect()),
        }],
        limits(source.encode_utf16().count()),
    )
    .unwrap()
}
#[test]
fn ordinary_scope_host_and_slotted_boundaries_have_canonical_certificates() {
    for source in [
        "@scope(.root){b{color:red}}",
        "@scope(:scope){b{color:red}}",
    ] {
        let model = compile(source);
        assert!(matches!(
            model.metadata().groups[0].scope_start_policy,
            Some(SelectorPolicyInspection::Unchanged)
        ));
    }
    let model = compile("@scope(:host) to(::slotted(.stop)){b{color:red}}");
    let group = &model.metadata().groups[0];
    let Some(SelectorPolicyInspection::Items(start)) = &group.scope_start_policy else {
        panic!("host must have a canonical policy")
    };
    assert!(start[0].host_reference);
    assert!(matches!(start[0].target, SelectorTarget::Element));
    let Some(SelectorPolicyInspection::Items(end)) = &group.scope_end_policy else {
        panic!("slotted must have a canonical policy")
    };
    assert!(matches!(
        end[0].target,
        SelectorTarget::Slotted { pseudo: None }
    ));
}
#[test]
fn unsupported_composed_boundaries_are_recorded_not_rewritten_or_dropped() {
    for boundary in [":host-context(.outside)", "::part(control)"] {
        let source = format!("@scope({boundary}){{b{{color:red}}}}");
        let model = compile(&source);
        let group = &model.metadata().groups[0];
        let Some(SelectorPolicyInspection::Items(items)) = &group.scope_start_policy else {
            panic!("unsupported composed source must retain its policy")
        };
        assert!(matches!(
            items[0].target,
            SelectorTarget::UnsupportedComposed
        ));
        assert_eq!(model.metadata().rules.len(), 1);
    }
}
#[test]
fn boundary_policy_preserves_exact_source_quota_and_payload_owner() {
    let source = "@scope(:host){b{color:red}}";
    let model = compile(source);
    let payload = model.retained_payload_bytes();
    let retained = model.clone();
    drop(model);
    assert_eq!(retained.retained_payload_bytes(), payload);
    let result = CompiledStyles::compile(
        &[SheetSource {
            id: 1,
            text: CssText::from_units(source.encode_utf16().collect()),
        }],
        limits(source.encode_utf16().count() - 1),
    );
    assert!(matches!(result, Err(CompileError::SourceUnits)));
}
