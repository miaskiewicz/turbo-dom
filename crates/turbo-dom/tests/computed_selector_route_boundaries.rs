#![cfg(feature = "computed-style-grammar")]
use turbo_dom::rtdom::computed_css::{
    scope_plan::ScopeFactError,
    selector_policy::{inspect_selector_policy, SelectorPolicyInspection, SelectorTarget},
    shadow_routes::{ContextRoot, RouteBudget, RouteError, RouteFacts, ValidatedRouteFacts},
    target_scope::ScopeError,
    ComputationLimits, CssText,
};
fn limits() -> ComputationLimits {
    ComputationLimits {
        max_nesting: 32,
        max_output_units: 4096,
        max_total_units: 4096,
    }
}
fn inspect(value: &str) -> SelectorPolicyInspection {
    inspect_selector_policy(
        &CssText::from_units(value.encode_utf16().collect()),
        limits(),
    )
    .unwrap()
}
#[test]
fn nested_host_references_keep_canonical_policy_through_logical_and_structural_functions() {
    for selector in [
        ":where(:host)",
        ":not(:host)",
        ":has(:host)",
        ":nth-child(1 of :host)",
    ] {
        let SelectorPolicyInspection::Items(items) = inspect(selector) else {
            panic!("{selector}")
        };
        assert!(items[0].host_reference, "{selector}");
        assert_eq!(items[0].target, SelectorTarget::Element, "{selector}");
    }
}
#[test]
fn nested_unimplemented_composed_capabilities_remain_explicit() {
    for selector in [
        ":host-context(.outside)",
        ":is(:host-context(.outside))",
        ":where(:host-context(.outside))",
        ":not(:host-context(.outside))",
        ":has(:host-context(.outside))",
        ":nth-child(1 of :host-context(.outside))",
    ] {
        let SelectorPolicyInspection::Items(items) = inspect(selector) else {
            panic!("{selector}")
        };
        assert_eq!(
            items[0].target,
            SelectorTarget::UnsupportedComposed,
            "{selector}"
        );
    }
}
fn invalid_context(roots: &[ContextRoot]) -> RouteError {
    let facts = RouteFacts {
        target: 11,
        own_tree_scope: 1,
        own_path: &[11, 10],
        host_of: None,
        assigned_slot: None,
        context_roots: roots,
        requested_pseudo: None,
    };
    match ValidatedRouteFacts::new(
        &facts,
        &mut RouteBudget {
            contexts: roots.len(),
            path_units: 2,
        },
    ) {
        Err(error) => error,
        Ok(_) => panic!("invalid foreign context graph admitted"),
    }
}
#[test]
fn mismatched_context_shapes_duplicate_hosts_and_cycles_cannot_be_reused() {
    let document = ContextRoot {
        tree_scope: 1,
        root: 10,
        host: None,
        parent_tree_scope: None,
    };
    for root in [
        ContextRoot {
            tree_scope: 2,
            root: 20,
            host: Some(11),
            parent_tree_scope: None,
        },
        ContextRoot {
            tree_scope: 2,
            root: 20,
            host: None,
            parent_tree_scope: Some(1),
        },
        ContextRoot {
            tree_scope: 2,
            root: 20,
            host: Some(20),
            parent_tree_scope: Some(1),
        },
    ] {
        assert_eq!(
            invalid_context(&[document, root]),
            RouteError::InvalidContext
        );
    }
    let shadow = ContextRoot {
        tree_scope: 2,
        root: 20,
        host: Some(11),
        parent_tree_scope: Some(1),
    };
    let second = ContextRoot {
        tree_scope: 3,
        root: 30,
        host: Some(11),
        parent_tree_scope: Some(1),
    };
    assert_eq!(
        invalid_context(&[document, shadow, second]),
        RouteError::InvalidContext
    );
    let first = ContextRoot {
        tree_scope: 1,
        root: 10,
        host: Some(21),
        parent_tree_scope: Some(2),
    };
    let second = ContextRoot {
        tree_scope: 2,
        root: 20,
        host: Some(11),
        parent_tree_scope: Some(1),
    };
    assert_eq!(
        invalid_context(&[first, second]),
        RouteError::InvalidContext
    );
}
#[test]
fn scope_fact_errors_preserve_invalid_path_and_resource_exhaustion_distinction() {
    assert_eq!(
        ScopeError::from(ScopeFactError::InvalidPath),
        ScopeError::InvalidPath
    );
    assert_eq!(
        ScopeError::from(ScopeFactError::ResourceLimit),
        ScopeError::ResourceLimit
    );
}

#[test]
fn backend_opaque_depth_is_refused_before_recursive_selector_grammar() {
    use turbo_dom::rtdom::computed_css::{grammar::BACKEND_MAX_COMPONENT_DEPTH, ComputedCssError};
    // This MUST fail the shared resource-only preflight. The recursive backend
    // must never be invoked at/above its unsafe fixed depth boundary.
    let count = BACKEND_MAX_COMPONENT_DEPTH + 1;
    let source = format!(
        ":host-context({}.x{})",
        "f(".repeat(count),
        ")".repeat(count)
    );
    let raw = CssText::from_units(source.encode_utf16().collect());
    assert_eq!(
        inspect_selector_policy(
            &raw,
            ComputationLimits {
                max_nesting: count + 2,
                max_output_units: 4096,
                max_total_units: 4096
            }
        ),
        Err(ComputedCssError::NestingLimit)
    );
}
