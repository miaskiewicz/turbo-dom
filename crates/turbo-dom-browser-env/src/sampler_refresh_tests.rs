//! Public sampler work counters; elapsed time and CPU samples are not call counts.
use super::*;
use crate::host_internals as host;

const REASONS: [&str; 19] = [
    "forced_root_fresh",
    "local_hit",
    "shared_absent",
    "checking",
    "volatile",
    "box_failed",
    "owner",
    "world",
    "probe",
    "media",
    "deps_absent",
    "deps_replaced",
    "post_change",
    "contextless_fresh",
    "shared_hit",
    "mode",
    "store_failed",
    "volatile_entries",
    "lazy_validation",
];

fn with_scene(html: &str, test: impl FnOnce(&mut v8::PinScope)) {
    tests::init_v8();
    let isolate = &mut tests::plain_isolate();
    isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_independent_html(scope, html);
    run_engine_script(scope, "globalThis.__samplerClockCalls=0;__tsNs.installAnimationHost({now:()=>{__samplerClockCalls++;return 1000},frame(){},task(){}});globalThis.__samplerProviderCalls=0;__tsNs.layoutUsedSize=()=>{__samplerProviderCalls++;throw Error('sampler requested used geometry')};").unwrap();
    let owner = host::enter_context_document(context).unwrap();
    test(scope);
    drop(owner);
    reset();
}

fn fixture(depth: usize, count: usize, extra: &str) -> String {
    let mut html = format!("<!doctype html><style>.ancestor{{display:block;font:16px/20px serif;color:rgb(1,2,3)}}.target{{display:block;width:100px;height:20px;{extra}}}</style>");
    for _ in 0..depth {
        html.push_str("<div class=ancestor>");
    }
    for index in 0..count {
        html.push_str(&format!("<div class=target id=target{index}></div>"));
    }
    for _ in 0..depth {
        html.push_str("</div>");
    }
    html
}

fn world() -> (u64, u64, u64, u64) {
    let (full, selector) = host::with_tree(|tree| (tree.version, tree.selector_version)).unwrap();
    let (cssom, live) = host_style_revisions();
    (full, selector, cssom, live)
}

fn public_targets<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    count: usize,
) -> (v8::Local<'s, v8::Object>, Vec<host::Handle>) {
    let (root, targets) = host::with_tree(|tree| {
        (
            tree.root(),
            (0..count)
                .map(|index| tree.query_selector(&format!("#target{index}")).unwrap())
                .collect(),
        )
    })
    .unwrap();
    (host::wrap(scope, root), targets)
}

fn evaluate(scope: &mut v8::PinScope, source: &str) -> String {
    let source = v8::String::new(scope, source).unwrap();
    v8::Script::compile(scope, source, None)
        .unwrap()
        .run(scope)
        .unwrap()
        .to_rust_string_lossy(scope)
}

fn measured_sample(
    scope: &mut v8::PinScope,
    document: v8::Local<v8::Object>,
    targets: &[host::Handle],
) -> (host::PresentedEffectsSnapshot, serde_json::Value) {
    let before = effect_refresh_counts(scope);
    let cascades = cascade_runs();
    let validations = effect_validations();
    let clock_before = evaluate(scope, "__samplerClockCalls")
        .parse::<u64>()
        .unwrap();
    let sample = host::sample_presented_effects(scope, document, targets).unwrap();
    let clock_after = evaluate(scope, "__samplerClockCalls")
        .parse::<u64>()
        .unwrap();
    assert_eq!(
        clock_after - clock_before,
        1,
        "one canonical clock read per HOST sample"
    );
    let after = effect_refresh_counts(scope);
    let mut counts = serde_json::Map::new();
    for (index, name) in REASONS.iter().enumerate() {
        counts.insert((*name).into(), (after[index] - before[index]).into());
    }
    counts.insert("clock_reads".into(), (clock_after - clock_before).into());
    counts.insert("cascades".into(), (cascade_runs() - cascades).into());
    counts.insert(
        "validations".into(),
        (effect_validations() - validations).into(),
    );
    (sample, counts.into())
}

fn assert_fresh_reference(
    scope: &mut v8::PinScope,
    document: v8::Local<v8::Object>,
    targets: &[host::Handle],
    expected: &host::PresentedEffectsSnapshot,
) {
    // Existing no-token inspection is the original fresh-map fallback, not a
    // manufactured cache flag or permission to ignore an uncertain entry.
    run_engine_script(scope, "__tsNs.__freshReference=__tsNs.refreshCssEffectInputs;__tsNs.refreshCssEffectInputs=function(el){return __tsNs.__freshReference(el);};").unwrap();
    let reference = host::sample_presented_effects(scope, document, targets);
    run_engine_script(
        scope,
        "__tsNs.refreshCssEffectInputs=__tsNs.__freshReference;delete __tsNs.__freshReference;",
    )
    .unwrap();
    assert_eq!(format!("{:?}", reference.unwrap()), format!("{expected:?}"));
}

fn assert_static_sample(sample: &host::PresentedEffectsSnapshot, count: usize) {
    assert_eq!(sample.sample_time, Some(1000.0));
    assert_eq!(sample.targets.len(), count);
    assert!(sample.targets.iter().all(|target| target.effects.is_empty() && target.css_animation_names.is_empty() && matches!(&target.presentation, host::PresentedTargetState::Certified { animation, transition } if animation.is_empty() && transition.is_empty())));
}

fn assert_static_work_bound(rows: &[serde_json::Value]) {
    for row in rows {
        let targets = row["targets"].as_u64().unwrap();
        let bound = targets + row["depth"].as_u64().unwrap() + 2;
        for phase in ["first", "second"] {
            let counts = &row[phase];
            assert_eq!(counts["forced_root_fresh"], targets, "{row}");
            assert!(counts["cascades"].as_u64().unwrap() <= bound, "{row}");
        }
    }
}

#[test]
fn lazy_inherited_defaults_preserve_the_public_sampler_batch_context() {
    let mut rows = Vec::new();
    for (case, extra) in [
        ("explicit", ""),
        ("inherited-default", "letter-spacing:inherit"),
    ] {
        for (depth, count) in [(4, 16), (4, 64), (16, 16), (16, 64)] {
            with_scene(&fixture(depth, count, extra), |scope| {
                let (document, targets) = public_targets(scope, count);
                // Materialize ordinary CSSOM/native bookkeeping before the measured
                // pair. Each public sampler still creates its own separate batch.
                host::sample_presented_effects(scope, document, &targets).unwrap();
                let before = world();
                let (first, first_counts) = measured_sample(scope, document, &targets);
                let middle = world();
                let (second, second_counts) = measured_sample(scope, document, &targets);
                let after = world();
                assert_eq!(
                    before, middle,
                    "{case}/{depth}/{count}: first world changed"
                );
                assert_eq!(
                    middle, after,
                    "{case}/{depth}/{count}: second world changed"
                );
                assert_static_sample(&first, count);
                assert_static_sample(&second, count);
                // Extra public metadata reads happen only after both counter windows.
                assert_eq!(evaluate(scope, "__samplerProviderCalls"), "0");
                assert_eq!(
                    evaluate(
                        scope,
                        "getComputedStyle(document.getElementById('target0')).letterSpacing"
                    ),
                    "normal"
                );
                rows.push(serde_json::json!({"case":case,"depth":depth,"targets":count,"world":before,"first":first_counts,"second":second_counts}));
            });
        }
    }
    eprintln!(
        "SAMPLER_REFRESH_ROWS={}",
        serde_json::to_string(&rows).unwrap()
    );
    assert_static_work_bound(&rows);
    // Inherited defaults must stay on the same proven, local/batch snapshot
    // path rather than silently starting uncoupled ancestor cascades.
    assert!(
        rows.iter()
            .all(|row| row["first"]["contextless_fresh"] == 0
                && row["second"]["contextless_fresh"] == 0),
        "{}",
        serde_json::to_string(&rows).unwrap()
    );
}

fn prepared_effect_case(scope: &mut v8::PinScope, case: &str) {
    let script = match case {
        "paused-ancestor" => "globalThis.__probe=document.querySelector('.ancestor').animate([{opacity:.2},{opacity:.8}],{duration:1000,fill:'both'});__probe.pause();__probe.currentTime=500;",
        "custom-default" => "document.querySelector('.ancestor').style.setProperty('--dx','10px');",
        "reference-box" => "for(const el of document.querySelectorAll('.target')){const a=el.animate([{transform:'translateX(0%)'},{transform:'translateX(50%)'}],{duration:1000,fill:'both'});a.pause();a.currentTime=500;}",
        _ => unreachable!(),
    };
    run_engine_script(scope, script).unwrap();
}

fn mutate_effect_case(scope: &mut v8::PinScope, case: &str) {
    let script = match case {
        "paused-ancestor" => "__probe.currentTime=750;",
        "custom-default" => "document.querySelector('.ancestor').style.setProperty('--dx','20px');",
        "reference-box" => "document.querySelector('.ancestor').style.width='600px';",
        _ => unreachable!(),
    };
    run_engine_script(scope, script).unwrap();
}

fn sample_property_values(sample: &host::PresentedEffectsSnapshot, property: &str) -> Vec<String> {
    let mut values = Vec::new();
    for target in &sample.targets {
        let host::PresentedTargetState::Certified {
            animation,
            transition,
        } = &target.presentation
        else {
            panic!("{sample:?}");
        };
        values.extend(
            animation
                .iter()
                .chain(transition)
                .filter(|pair| pair.name == property)
                .map(|pair| pair.value.clone()),
        );
    }
    values.sort();
    values.dedup();
    values
}

fn assert_effect_case(
    sample: &host::PresentedEffectsSnapshot,
    case: &str,
    count: usize,
    changed: bool,
) {
    assert_eq!(sample.sample_time, Some(1000.0));
    let expected_count = count + usize::from(case == "paused-ancestor");
    assert_eq!(sample.targets.len(), expected_count);
    let (property, expected) = match case {
        "paused-ancestor" => ("opacity", if changed { "0.65" } else { "0.5" }),
        "reference-box" => ("transform", "translate(25%, 0px)"),
        "custom-default" => {
            assert_static_sample(sample, count);
            return;
        }
        _ => unreachable!(),
    };
    assert_eq!(sample_property_values(sample, property), [expected]);
}

#[test]
fn repeated_sampler_refresh_counters_separate_active_custom_and_reference_inputs() {
    let mut rows = Vec::new();
    for case in ["paused-ancestor", "custom-default", "reference-box"] {
        let extra = match case {
            "custom-default" => "letter-spacing:inherit;transform:translateX(var(--dx))",
            "reference-box" => "width:50%",
            _ => "",
        };
        for (depth, count) in [(4, 16), (4, 64), (16, 16), (16, 64)] {
            with_scene(&fixture(depth, count, extra), |scope| {
                prepared_effect_case(scope, case);
                let (document, targets) = public_targets(scope, count);
                host::sample_presented_effects(scope, document, &targets).unwrap();
                let before = world();
                let (first, first_counts) = measured_sample(scope, document, &targets);
                let middle = world();
                let (second, second_counts) = measured_sample(scope, document, &targets);
                let after = world();
                assert_eq!(before, middle, "{case}/{depth}/{count}");
                assert_eq!(middle, after, "{case}/{depth}/{count}");
                assert_effect_case(&first, case, count, false);
                assert_effect_case(&second, case, count, false);
                assert_eq!(evaluate(scope, "__samplerProviderCalls"), "0");
                if case == "custom-default" {
                    assert_eq!(
                        evaluate(
                            scope,
                            "getComputedStyle(document.getElementById('target0')).transform"
                        ),
                        "matrix(1, 0, 0, 1, 10, 0)"
                    );
                }
                let mutation_before = world();
                mutate_effect_case(scope, case);
                let mutation_after = world();
                assert_ne!(
                    mutation_before, mutation_after,
                    "{case}/{depth}/{count}: input mutation was not tracked"
                );
                let (changed, changed_counts) = measured_sample(scope, document, &targets);
                assert_effect_case(&changed, case, count, true);
                assert_eq!(evaluate(scope, "__samplerProviderCalls"), "0");
                if case == "custom-default" {
                    assert_eq!(
                        evaluate(
                            scope,
                            "getComputedStyle(document.getElementById('target0')).transform"
                        ),
                        "matrix(1, 0, 0, 1, 20, 0)"
                    );
                }
                assert_fresh_reference(scope, document, &targets, &changed);
                let row = serde_json::json!({"case":case,"depth":depth,"targets":count,"world":before,"first":first_counts,"second":second_counts,"mutated_world":mutation_after,"changed":changed_counts});
                eprintln!("SAMPLER_EFFECT_ROW={row}");
                rows.push(row);
            });
        }
    }
    eprintln!(
        "SAMPLER_EFFECT_ROWS={}",
        serde_json::to_string(&rows).unwrap()
    );
}

fn mutate_lazy_world(scope: &mut v8::PinScope) {
    run_engine_script(
        scope,
        "document.querySelector('.ancestor').style.letterSpacing='7px';",
    )
    .unwrap();
}

fn mutate_lazy_probe(scope: &mut v8::PinScope) {
    assert!(set_computed_font_defaults(
        scope.get_current_context(),
        "serif",
        32.0
    ));
}

fn mutate_lazy_media(scope: &mut v8::PinScope) {
    run_engine_script(scope, "innerWidth=500;").unwrap();
}

fn throw_lazy_media(scope: &mut v8::PinScope) {
    run_engine_script(scope, "__tsNs.__originalLazyMedia=__tsNs.mediaTextMatches;__tsNs.mediaTextMatches=()=>{throw Error('host media unavailable')};").unwrap();
}

fn arm_lazy_action(scope: &mut v8::PinScope, action: fn(&mut v8::PinScope)) {
    scope
        .get_slot::<EffectRefreshCounts>()
        .unwrap()
        .action
        .set(Some(action));
}

fn assert_lazy_action_reached(scope: &v8::Isolate) {
    assert!(
        scope
            .get_slot::<EffectRefreshCounts>()
            .unwrap()
            .action
            .get()
            .is_none(),
        "native mutation callback was not reached at the actual lazy-validation boundary"
    );
}

fn capture_refreshed_values(scope: &mut v8::PinScope) {
    run_engine_script(scope, "globalThis.__sampledValues=[];const ns=__tsNs,original=ns.refreshCssEffectInputs;ns.refreshCssEffectInputs=function(el,token){const value=Reflect.apply(original,this,[el,token]);if(value)__sampledValues.push([ns.computedStyleValue(value,'letter-spacing'),ns.computedStyleValue(value,'font-size')]);return value;};globalThis.__samplerMicrotasks=0;Promise.resolve().then(()=>{__samplerMicrotasks++});").unwrap();
}

fn lazy_media_fixture() -> String {
    fixture(4, 3, "letter-spacing:inherit").replace(
        "</style>",
        "@media(max-width:600px){.ancestor{letter-spacing:7px}}</style>",
    )
}

#[test]
fn current_map_lazy_reads_revalidate_mutation_inside_the_same_root() {
    for (case, action, reason, spacing) in [
        (
            "world",
            mutate_lazy_world as fn(&mut v8::PinScope),
            "world",
            "7px",
        ),
        ("probe", mutate_lazy_probe, "probe", "normal"),
        ("media", mutate_lazy_media, "media", "7px"),
    ] {
        with_scene(&lazy_media_fixture(), |scope| {
            run_engine_script(scope, "innerWidth=1000;").unwrap();
            let (document, targets) = public_targets(scope, 3);
            host::sample_presented_effects(scope, document, &targets).unwrap();
            capture_refreshed_values(scope);
            arm_lazy_action(scope, action);
            let before = world();
            let (sample, counts) = measured_sample(scope, document, &targets);
            let after = world();
            assert_lazy_action_reached(scope);
            assert_static_sample(&sample, 3);
            assert!(counts[reason].as_u64().unwrap() > 0, "{case}: {counts}");
            assert!(
                counts["contextless_fresh"].as_u64().unwrap() > 0,
                "uncertain input must retain fresh-map fallback: {case}: {counts}"
            );
            if case == "world" {
                assert_ne!(before, after);
            } else {
                assert_eq!(before, after);
            }
            let values: serde_json::Value =
                serde_json::from_str(&evaluate(scope, "JSON.stringify(__sampledValues)")).unwrap();
            assert!(
                values
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|row| row[0] == spacing),
                "{case}: {values}"
            );
            assert_eq!(evaluate(scope, "__samplerProviderCalls"), "0");
            assert_eq!(evaluate(scope, "__samplerMicrotasks"), "0");
            eprintln!(
                "SAMPLER_NATIVE_MUTATION={}",
                serde_json::json!({"case":case,"before":before,"after":after,"counts":counts,"underlying":values})
            );
            assert_fresh_reference(scope, document, &targets, &sample);
            assert_eq!(evaluate(scope, "__samplerMicrotasks"), "0");
            scope.perform_microtask_checkpoint();
            assert_eq!(evaluate(scope, "__samplerMicrotasks"), "1");
        });
    }
}

#[test]
fn current_map_lazy_reads_keep_between_root_mutations_fresh() {
    with_scene(&fixture(4, 2, "letter-spacing:inherit"), |scope| {
        let (document, targets) = public_targets(scope, 2);
        host::sample_presented_effects(scope, document, &targets).unwrap();
        run_engine_script(scope, "const ns=__tsNs,original=ns.refreshCssEffectInputs;globalThis.__betweenValues=[];ns.refreshCssEffectInputs=function(el,token){if(__betweenValues.length===1)document.querySelector('.ancestor').style.letterSpacing='7px';const value=Reflect.apply(original,this,[el,token]);if(value)__betweenValues.push(ns.computedStyleValue(value,'letter-spacing'));return value;};").unwrap();
        let (sample, counts) = measured_sample(scope, document, &targets);
        assert_static_sample(&sample, 2);
        assert_eq!(
            evaluate(scope, "JSON.stringify(__betweenValues)"),
            "[\"normal\",\"7px\"]"
        );
        assert!(counts["world"].as_u64().unwrap() > 0, "{counts}");
        assert_eq!(evaluate(scope, "__samplerProviderCalls"), "0");
    });
}

#[test]
fn current_map_lazy_reads_preserve_volatile_and_failed_box_refusal() {
    with_scene(&fixture(4, 3, "letter-spacing:inherit"), |scope| {
        prepared_effect_case(scope, "paused-ancestor");
        let (document, targets) = public_targets(scope, 3);
        host::sample_presented_effects(scope, document, &targets).unwrap();
        let (sample, counts) = measured_sample(scope, document, &targets);
        assert_effect_case(&sample, "paused-ancestor", 3, false);
        assert!(counts["volatile"].as_u64().unwrap() > 0, "{counts}");
        assert!(
            counts["contextless_fresh"].as_u64().unwrap() > 0,
            "{counts}"
        );
        assert_eq!(evaluate(scope, "__samplerProviderCalls"), "0");
        assert_fresh_reference(scope, document, &targets, &sample);
    });
    with_scene(&lazy_media_fixture(), |scope| {
        run_engine_script(scope, "innerWidth=1000;").unwrap();
        let (document, targets) = public_targets(scope, 3);
        host::sample_presented_effects(scope, document, &targets).unwrap();
        arm_lazy_action(scope, throw_lazy_media);
        let (sample, counts) = measured_sample(scope, document, &targets);
        assert_lazy_action_reached(scope);
        assert_static_sample(&sample, 3);
        assert!(
            counts["box_failed"].as_u64().unwrap() > 0,
            "failed cascade metadata must not become reusable: {counts}"
        );
        assert!(
            counts["contextless_fresh"].as_u64().unwrap() > 0,
            "{counts}"
        );
        assert_eq!(evaluate(scope, "__samplerProviderCalls"), "0");
        assert_fresh_reference(scope, document, &targets, &sample);
        run_engine_script(
            scope,
            "__tsNs.mediaTextMatches=__tsNs.__originalLazyMedia;delete __tsNs.__originalLazyMedia;",
        )
        .unwrap();
        assert_static_sample(
            &host::sample_presented_effects(scope, document, &targets).unwrap(),
            3,
        );
    });
}

#[test]
fn sampler_refresh_counter_registry_is_private_and_isolate_owned() {
    with_scene(&fixture(1, 2, ""), |scope| {
        let (document, targets) = public_targets(scope, 2);
        host::sample_presented_effects(scope, document, &targets).unwrap();
        let before = effect_refresh_counts(scope);
        assert!(before[0] > 0);
        assert_eq!(evaluate(scope, "typeof __noteEffectRefresh"), "undefined");
        with_scene(&fixture(1, 3, "letter-spacing:inherit"), |inner| {
            assert_eq!(effect_refresh_counts(inner), [0; 19]);
            let (document, targets) = public_targets(inner, 3);
            host::sample_presented_effects(inner, document, &targets).unwrap();
            assert_eq!(effect_refresh_counts(inner)[0], 3);
        });
        assert_eq!(effect_refresh_counts(scope), before);
    });
}

#[test]
fn sampler_refresh_counters_work_after_native_snapshot_rehydration() {
    tests::init_v8();
    let blob = snapshot_blob().unwrap();
    let isolate = &mut v8::Isolate::new(blob.create_params());
    isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    assert!(install_snapshot_context(
        scope,
        Tree::parse_with_processing_instructions(&fixture(1, 1, ""))
    )
    .is_ok());
    assert!(scope.get_slot::<EffectRefreshCounts>().is_none());
    run_engine_script(
        scope,
        "__tsNs.installAnimationHost({now:()=>1000,frame(){},task(){}});",
    )
    .unwrap();
    let (document, targets) = public_targets(scope, 1);
    let owner = host::enter_document_of(scope, document);
    let sample = host::sample_presented_effects(scope, document, &targets).unwrap();
    assert_static_sample(&sample, 1);
    assert_eq!(effect_refresh_counts(scope)[0], 1);
    drop(owner);
    reset();
}
