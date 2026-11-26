use super::*;
use turbo_dom_parser::rtdom::tree::{CollectionBudget, CollectionError};

fn budget() -> CollectionBudget {
    CollectionBudget {
        max_nodes: 100_000,
        max_edges: 1_000_000,
        max_payload_bytes: 16 * 1024 * 1024,
    }
}

#[test]
fn sparse_storage_reports_each_owner_and_preserves_only_proven_checkpoints() {
    init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    browser_env::install_independent_html(scope, "<p>active</p>");
    let main = object(scope, "document");
    let main_id = document_id_of(scope, main);
    let first = collect_native_storage_if_needed(scope, budget(), 1).unwrap();
    let [(owner, StorageCollectionOutcome::Compacted(first))] = first.documents.as_slice() else {
        panic!("first sweep should materialize the parsed document")
    };
    assert_eq!(main_id, *owner);
    assert_eq!(
        first.reclaimed_nodes,
        first.before.live_nodes - first.after.live_nodes
    );
    let stable = document_checkpoint(scope, main).unwrap();

    let extra = browser_env::parse_html_document(scope, "<p>inert</p>", false);
    let extra_id = document_id_of(scope, extra);
    assert_ne!(extra_id, main_id);
    assert_eq!(
        collect_native_storage_if_needed(scope, budget(), 1).unwrap_err(),
        CollectionError::BudgetExceeded
    );
    assert_eq!(document_checkpoint(scope, main), Some(stable));
    let report = collect_native_storage_if_needed(scope, budget(), 2).unwrap();
    assert_eq!(report.documents.len(), 2);
    for (owner, outcome) in report.documents {
        if owner == main_id {
            assert!(matches!(outcome, StorageCollectionOutcome::Unchanged(_)));
            assert_eq!(outcome.report().before, outcome.report().after);
        } else {
            assert_eq!(owner, extra_id);
            assert!(matches!(outcome, StorageCollectionOutcome::Compacted(_)));
        }
    }
    assert_eq!(document_checkpoint(scope, main), Some(stable));
    let ordinary = browser_env::collect_native_storage(scope, budget(), 2).unwrap();
    assert_eq!(ordinary.documents.len(), 2);
    assert!(ordinary
        .documents
        .iter()
        .all(|(_, report)| report.reclaimed_nodes == 0));
    assert_ne!(document_checkpoint(scope, main).unwrap(), stable);
    browser_env::reset();
}
