use super::tests::{retained_fixture, retained_input};
use super::*;
use std::sync::Barrier;

fn variant(input: &Value, color: &str) -> Value {
    let mut next = input.clone();
    next["targets"][0]["inline"] =
        json!(format!("color:{color}").encode_utf16().collect::<Vec<_>>());
    next
}
fn calls(owner: &PreparedHandle) -> (usize, usize) {
    let slot = owner.matching.as_ref().unwrap();
    (slot.parse_calls(), slot.cascade_calls())
}
#[test]
fn retained_multi_entry_aba_reuses_both_exact_successes() {
    let (owner, token, input) = retained_fixture();
    let base_bytes = owner.retained_bytes() as usize;
    let a = retained_input(&input);
    let b = retained_input(&variant(&input, "blue"));
    let first = owner.resolve_matching(&a, &token).unwrap();
    let second = owner.resolve_matching(&b, &token).unwrap();
    assert_ne!(first, second);
    assert_eq!(owner.resolve_matching(&a, &token).unwrap(), first);
    assert_eq!(calls(&owner), (2, 2));
    assert_eq!(owner.resolve_matching(&b, &token).unwrap(), second);
    assert_eq!(calls(&owner), (2, 2));
    assert_eq!(
        owner.retained_bytes() as usize,
        base_bytes + a.len() + b.len() + first.len() + second.len()
    );
}
#[test]
fn retained_multi_entry_count_bound_and_hit_promotion_are_lru() {
    let (owner, token, input) = retained_fixture();
    let base = retained_input(&input);
    let wires: Vec<_> = (0..9).map(|n| format!("{base}{}", " ".repeat(n))).collect();
    for wire in &wires[..8] {
        owner.resolve_matching(wire, &token).unwrap();
    }
    assert_eq!(calls(&owner), (8, 8));
    owner.resolve_matching(&wires[0], &token).unwrap();
    owner.resolve_matching(&wires[8], &token).unwrap();
    owner.resolve_matching(&wires[0], &token).unwrap();
    assert_eq!(calls(&owner), (9, 9));
    owner.resolve_matching(&wires[1], &token).unwrap();
    assert_eq!(calls(&owner), (10, 10));
}
#[test]
fn retained_multi_entry_byte_pressure_evicts_multiple_old_successes() {
    let (owner, token, input) = retained_fixture();
    let baseline = owner.retained_bytes() as usize;
    let base = retained_input(&input);
    for n in 0..3 {
        owner
            .resolve_matching(&format!("{base}{}", " ".repeat(10000 + n)), &token)
            .unwrap();
    }
    let large = format!("{base}{}", " ".repeat(58000));
    let response = owner.resolve_matching(&large, &token).unwrap();
    assert_eq!(
        owner.retained_bytes() as usize,
        baseline + large.len() + response.len()
    );
    assert!(owner.retained_bytes() as usize <= baseline + 64 * 1024);
    assert_eq!(owner.resolve_matching(&large, &token).unwrap(), response);
    assert_eq!(calls(&owner), (4, 4));
    owner
        .resolve_matching(&format!("{base}{}", " ".repeat(10000)), &token)
        .unwrap();
    assert_eq!(calls(&owner), (5, 5));
}
#[test]
fn retained_multi_entry_errors_do_not_publish_or_evict_successes() {
    let (owner, token, input) = retained_fixture();
    let a = retained_input(&input);
    let b = retained_input(&variant(&input, "blue"));
    let first = owner.resolve_matching(&a, &token).unwrap();
    let second = owner.resolve_matching(&b, &token).unwrap();
    let bytes = owner.retained_bytes();
    for _ in 0..2 {
        assert!(owner.resolve_matching("invalid-json", &token).is_err());
    }
    let mut bad = input.clone();
    bad["limits"]["answerRows"] = json!(0);
    assert_eq!(
        owner
            .resolve_matching(&retained_input(&bad), &token)
            .unwrap_err(),
        owner.resolve_batch(&bad.to_string()).unwrap_err()
    );
    let before = calls(&owner);
    assert_eq!(owner.resolve_matching(&a, &token).unwrap(), first);
    assert_eq!(owner.resolve_matching(&b, &token).unwrap(), second);
    assert_eq!(calls(&owner), before);
    assert_eq!(owner.retained_bytes(), bytes);
}
#[test]
fn retained_multi_entry_oversized_success_clears_all_previous_results() {
    let (owner, token, input) = retained_fixture();
    let before = owner.retained_bytes();
    let a = retained_input(&input);
    let b = retained_input(&variant(&input, "blue"));
    owner.resolve_matching(&a, &token).unwrap();
    owner.resolve_matching(&b, &token).unwrap();
    let large = format!("{}{a}", " ".repeat(64 * 1024));
    owner.resolve_matching(&large, &token).unwrap();
    assert_eq!(owner.retained_bytes(), before);
    owner.resolve_matching(&a, &token).unwrap();
    owner.resolve_matching(&b, &token).unwrap();
    assert_eq!(calls(&owner), (5, 5));
}
#[test]
fn retained_multi_entry_owners_replacement_and_disposal_are_checked_first() {
    let (mut owner, token, input) = retained_fixture();
    let (other, other_token, _) = retained_fixture();
    let wires = [
        retained_input(&input),
        retained_input(&variant(&input, "blue")),
    ];
    for wire in &wires {
        owner.resolve_matching(wire, &token).unwrap();
    }
    for wire in &wires {
        assert!(owner
            .resolve_matching(wire, &other_token)
            .unwrap_err()
            .contains("StaleMatching"));
        assert!(other
            .resolve_matching(wire, &token)
            .unwrap_err()
            .contains("StaleMatching"));
    }
    assert_eq!(calls(&owner), (2, 2));
    assert_eq!(calls(&other), (0, 0));
    let empty = json!({"domains":[],"selectorAnswers":[],"scopeAnswers":[]});
    owner
        .store_matching(&empty.to_string(), &input["limits"].to_string())
        .unwrap();
    for wire in &wires {
        assert!(owner
            .resolve_matching(wire, &token)
            .unwrap_err()
            .contains("StaleMatching"));
    }
    assert_eq!(calls(&owner), (0, 0));
    owner.dispose();
    assert_eq!(owner.retained_bytes(), 0.0);
    assert!(owner
        .resolve_matching("invalid-json", &token)
        .unwrap_err()
        .contains("Disposed"));
}
#[test]
fn retained_multi_entry_concurrent_duplicate_misses_keep_unique_accounting() {
    let (owner, token, input) = retained_fixture();
    let baseline = owner.retained_bytes() as usize;
    let requests: Vec<_> = ["red", "blue", "green", "black"]
        .into_iter()
        .map(|color| variant(&input, color))
        .collect();
    let wires: Vec<_> = requests.iter().map(retained_input).collect();
    let expected: Vec<_> = requests
        .iter()
        .map(|v| owner.resolve_batch(&v.to_string()).unwrap())
        .collect();
    let barrier = Barrier::new(8);
    std::thread::scope(|scope| {
        let mut workers = Vec::new();
        for n in 0..8 {
            let wire = &wires[n % 4];
            let barrier = &barrier;
            let owner = &owner;
            let token = &token;
            workers.push(scope.spawn(move || {
                barrier.wait();
                owner.resolve_matching(wire, token).unwrap()
            }));
        }
        for (n, worker) in workers.into_iter().enumerate() {
            assert_eq!(worker.join().unwrap(), expected[n % 4]);
        }
    });
    let before = calls(&owner);
    assert!((4..=8).contains(&before.0));
    for (wire, response) in wires.iter().zip(&expected) {
        assert_eq!(owner.resolve_matching(wire, &token).unwrap(), *response);
    }
    assert_eq!(calls(&owner), before);
    let payload: usize = wires
        .iter()
        .zip(&expected)
        .map(|(wire, response)| wire.len() + response.len())
        .sum();
    assert_eq!(owner.retained_bytes() as usize, baseline + payload);
}
