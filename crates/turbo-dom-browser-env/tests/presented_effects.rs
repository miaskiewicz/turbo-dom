#![cfg(feature = "host-internals")]
use sha2::{Digest, Sha256};
use std::sync::Once;
use turbo_dom_browser_env::{browser_env, host_internals::*};

fn eval<'s>(scope: &mut v8::PinScope<'s, '_>, source: &str) -> v8::Local<'s, v8::Value> {
    let source = v8::String::new(scope, source).unwrap();
    v8::Script::compile(scope, source, None)
        .unwrap()
        .run(scope)
        .unwrap()
}

fn object<'s>(scope: &mut v8::PinScope<'s, '_>, source: &str) -> v8::Local<'s, v8::Object> {
    v8::Local::try_from(eval(scope, source)).unwrap()
}

fn init_v8() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let platform = v8::new_default_platform(0, false).make_shared();
        v8::V8::initialize_platform(platform);
        v8::V8::initialize();
    });
}

fn with_dom_using(independent: bool, test: impl FnOnce(&mut v8::PinScope)) {
    with_dom_lifecycle(independent, true, test);
}

fn with_dom_lifecycle(independent: bool, reset: bool, test: impl FnOnce(&mut v8::PinScope)) {
    with_html(
        independent,
        reset,
        "<!doctype html><div id=target style='width:100px;height:20px'></div>",
        test,
    );
}

fn with_html(independent: bool, reset: bool, html: &str, test: impl FnOnce(&mut v8::PinScope)) {
    init_v8();
    let isolate = &mut v8::Isolate::new(snapshot_params());
    isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    if independent {
        browser_env::install_independent_html(scope, html);
    } else {
        browser_env::install_with_html(scope, html);
    }
    browser_env::run_engine_script(
        scope,
        r#"
        let now=1000;
        globalThis.__clockCalls=0;globalThis.__steppingClock=false;
        __tsNs.installAnimationHost({now:()=>{__clockCalls++;return now+(__steppingClock?__clockCalls:0)},frame(){},task(){}});
        globalThis.__layoutCalls=0;
        __tsNs.layoutUsedSize=()=>{__layoutCalls++;throw Error('layout provider called')};
        __tsNs.resolveTransform=()=>{__layoutCalls++;throw Error('transform provider called')};
    "#,
    )
    .unwrap();
    test(scope);
    if reset {
        browser_env::reset();
    }
}

fn with_dom(test: impl FnOnce(&mut v8::PinScope)) {
    with_dom_using(true, test);
}

#[test]
fn badge_transition_resolves_custom_tokens_and_keeps_symbolic_reference_lengths() {
    with_html(
        true,
        true,
        include_str!("fixtures/mui_badge_transition.html"),
        |scope| {
            eval(scope,"globalThis.badge=document.getElementById('badge');getComputedStyle(badge).transform;badge.className='visible';__layoutCalls=0;");
            let document = object(scope, "document");
            let badge = object(scope, "badge");
            let handle = handle_of(scope, badge).unwrap();
            let sample = sample_presented_effects(scope, document, &[handle]).unwrap();
            let PresentedTargetState::Certified {
                animation,
                transition,
            } = &sample.targets[0].presentation
            else {
                panic!("{sample:?}")
            };
            assert!(animation.is_empty());
            assert!(transition.iter().any(|pair| pair.name == "transform"));
            assert!(eval(scope, "__layoutCalls===0").is_true());
        },
    );
}

#[test]
fn badge_transition_freezes_variable_endpoints_before_restart() {
    let oracle: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/mui_badge_model_chrome159.json")).unwrap();
    let html = include_str!("fixtures/mui_badge_transition.html");
    assert_eq!(
        format!("{:x}", Sha256::digest(html)),
        oracle["sha256"].as_str().unwrap()
    );
    with_html(true, true, html, |scope| {
        let value=eval(scope,"globalThis.badge=document.getElementById('badge');getComputedStyle(badge).transform;badge.className='visible';globalThis.oldBadge=badge.getAnimations().find(a=>a instanceof CSSTransition);oldBadge.pause();oldBadge.currentTime=0;globalThis.beforeBadge=oldBadge.effect.getKeyframes();badge.style.setProperty('--Badge-translate','25%,-25%');getComputedStyle(badge).transform;JSON.stringify({before:beforeBadge,after:oldBadge.effect.getKeyframes(),next:badge.getAnimations().find(a=>a instanceof CSSTransition&&a!==oldBadge).effect.getKeyframes()})");
        let actual: serde_json::Value =
            serde_json::from_str(&value.to_rust_string_lossy(scope)).unwrap();
        assert_eq!(actual["before"], actual["after"]);
        for field in ["before", "after", "next"] {
            for index in 0..2 {
                let actual = actual[field][index]["transform"]
                    .as_str()
                    .unwrap()
                    .replace(' ', "");
                let expected = oracle["result"][field][index]["transform"]
                    .as_str()
                    .unwrap()
                    .replace(' ', "");
                assert_eq!(actual, expected, "{field}/{index}");
            }
        }
    });
}

#[test]
fn badge_variables_use_parent_computed_tokens_and_real_fallbacks() {
    for (parent, own, reference) in [
        (
            "--pair:50%,-50%;--alias:var(--pair)",
            "--pair:25%,-25%",
            "var(--alias)",
        ),
        ("", "", "var(--missing,50%,-50%)"),
        ("--pair:50%,-50%", "", "var(--pair)"),
    ] {
        let html=format!("<!doctype html><div style='{parent}'><span id=target style='{own};transform:scale(0) translate({reference});transition:transform 225ms linear'></span></div>");
        with_html(true, true, &html, |scope| {
            eval(scope,&format!("globalThis.target=document.getElementById('target');getComputedStyle(target).transform;target.style.transform='scale(1) translate({reference})';__layoutCalls=0;"));
            let document = object(scope, "document");
            let target = object(scope, "target");
            let handle = handle_of(scope, target).unwrap();
            let sample = sample_presented_effects(scope, document, &[handle]).unwrap();
            let PresentedTargetState::Certified { transition, .. } =
                &sample.targets[0].presentation
            else {
                panic!("{sample:?}")
            };
            let transform = &transition
                .iter()
                .find(|pair| pair.name == "transform")
                .unwrap()
                .value;
            assert!(
                transform.replace(' ', "").contains("translate(50%,-50%)"),
                "{transform}"
            );
            assert!(eval(scope, "__layoutCalls===0").is_true());
        });
    }
}

#[test]
fn badge_invalid_custom_cycles_use_the_canonical_initial_transform() {
    with_dom(|scope| {
        let value=eval(scope,"globalThis.target=document.getElementById('target');target.style.cssText='--a:var(--b);--b:var(--a);transform:translate(var(--a));transition:transform 225ms linear';getComputedStyle(target).transform");
        assert_eq!(value.to_rust_string_lossy(scope), "none");
        let document = object(scope, "document");
        let target = object(scope, "target");
        let handle = handle_of(scope, target).unwrap();
        let sample = sample_presented_effects(scope, document, &[handle]).unwrap();
        let PresentedTargetState::Certified {
            animation,
            transition,
        } = &sample.targets[0].presentation
        else {
            panic!("{sample:?}")
        };
        assert!(animation.is_empty() && transition.is_empty());
    });
}

#[test]
fn repeated_badge_tokens_below_the_operation_ceiling_are_valid() {
    with_dom(|scope| {
        eval(scope,"globalThis.target=document.getElementById('target');target.style.setProperty('--piece','translate(50%,-50%)');target.style.setProperty('--many','var(--piece) '.repeat(32));target.style.cssText+=';transform:scale(0) var(--many);transition:transform 225ms linear';getComputedStyle(target).transform;target.style.transform='scale(1) var(--many)';__layoutCalls=0;");
        let document = object(scope, "document");
        let target = object(scope, "target");
        let handle = handle_of(scope, target).unwrap();
        let sample = sample_presented_effects(scope, document, &[handle]).unwrap();
        let PresentedTargetState::Certified { transition, .. } = &sample.targets[0].presentation
        else {
            panic!("{sample:?}")
        };
        assert_eq!(
            transition
                .iter()
                .find(|pair| pair.name == "transform")
                .unwrap()
                .value
                .matches("translate(")
                .count(),
            32
        );
        assert!(eval(scope, "__layoutCalls===0").is_true());
    });
}

#[test]
fn token_bridge_preserves_utf16_and_scopes_its_new_operation_ceiling() {
    with_dom(|scope| {
        browser_env::run_engine_script(scope,r#"
          const ns=__tsNs,cap=1048576;
          const tokens=ns.computeEffectCustomTokens([],['--raw','\ud800','--pair','50%,-50%']);
          globalThis.tokenUnits=tokens[Array.prototype.indexOf.call(tokens,'--raw')+1].charCodeAt(0);
          const literal='translate(0px)'+ ' '.repeat(cap);
          globalThis.largeLiteral=ns.substituteEffectTokens(literal,null)===literal;
          let typed=false;try{ns.substituteEffectTokens('var(--pair)'+ ' '.repeat(cap),[]);}catch(error){typed=error instanceof RangeError;}
          globalThis.largeVariable=typed;
          const exact=ns.computeEffectCustomTokens([],['--x','x'.repeat(cap-3)]);
          globalThis.exactTokenCap=exact[1].length===cap-3;
          typed=false;try{ns.computeEffectCustomTokens([],['--x','x'.repeat(cap-2)]);}catch(error){typed=error instanceof RangeError;}
          globalThis.overTokenCap=typed;
        "#).unwrap();
        assert!(eval(
            scope,
            "tokenUnits===0xd800&&largeLiteral&&largeVariable&&exactTokenCap&&overTokenCap"
        )
        .is_true());
    });
}

#[test]
fn numeric_retargets_start_from_the_same_clock_presented_value() {
    for (property, from, to, next, expected) in [
        ("width", "100px", "200px", "300px", "150px"),
        ("opacity", "0", "1", "0.25", "0.5"),
    ] {
        with_dom(|scope| {
            eval(scope,&format!("globalThis.target=document.getElementById('target');target.style.setProperty('{property}','{from}');target.style.transition='{property} 1000ms linear';getComputedStyle(target).getPropertyValue('{property}');target.style.setProperty('{property}','{to}');globalThis.old=target.getAnimations().find(a=>a instanceof CSSTransition);old.pause();old.currentTime=500;target.style.setProperty('{property}','{next}');getComputedStyle(target).getPropertyValue('{property}');globalThis.newMotion=target.getAnimations().find(a=>a instanceof CSSTransition&&a!==old);"));
            let text = eval(
                scope,
                &format!("newMotion.effect.getKeyframes()[0]['{property}']"),
            )
            .to_rust_string_lossy(scope);
            assert_eq!(text, expected);
        });
    }
}

#[test]
fn symbolic_transform_decomposition_with_different_lists_stays_unknown() {
    with_dom(|scope| {
        eval(scope,"globalThis.target=document.getElementById('target');globalThis.motion=target.animate([{transform:'rotate(0deg)'},{transform:'translateX(50%)'}],{duration:1000,fill:'both'});motion.pause();motion.currentTime=500;__layoutCalls=0;");
        let document = object(scope, "document");
        let target = object(scope, "target");
        let handle = handle_of(scope, target).unwrap();
        let sample = sample_presented_effects(scope, document, &[handle]).unwrap();
        assert!(
            matches!(
                sample.targets[0].presentation,
                PresentedTargetState::Unknown { .. }
            ),
            "{sample:?}"
        );
        assert!(eval(scope, "__layoutCalls===0").is_true());
    });
}

#[test]
fn percentage_zero_is_not_an_absolute_z_coordinate() {
    with_dom(|scope| {
        assert!(eval(scope,"const target=document.getElementById('target');['translateZ(0%)','translate3d(0,0,0%)'].every(transform=>{const a=target.animate([{transform},{transform}],{duration:1000,fill:'both'});const dropped=a.effect.getKeyframes().every(frame=>frame.transform===undefined);a.cancel();return dropped;})").is_true());
        assert!(eval(scope,"['translate(0%,0%)','translateZ(0px)','translate3d(0%,0%,0px)'].every(transform=>{const a=target.animate([{transform},{transform}],{duration:1000,fill:'both'});const kept=a.effect.getKeyframes().every(frame=>frame.transform!==undefined);a.cancel();return kept;})").is_true());
    });
}

#[test]
fn animated_custom_properties_on_open_and_closed_hosts_remain_unproved() {
    for mode in ["open", "closed"] {
        with_dom(|scope| {
            eval(scope,&format!("globalThis.host=document.createElement('div');document.body.append(host);host.style.setProperty('--pair','50%,-50%');globalThis.root=host.attachShadow({{mode:'{mode}'}});root.innerHTML='<span id=badge style=\"transform:scale(0) translate(var(--pair));transition:transform 225ms linear\"></span>';globalThis.badge=root.getElementById('badge');globalThis.custom=host.animate([{{'--pair':'50%,-50%'}},{{'--pair':'25%,-25%'}}],{{duration:1000,fill:'both'}});custom.pause();custom.currentTime=500;getComputedStyle(badge).transform;badge.style.transform='scale(1) translate(var(--pair))';__layoutCalls=0;"));
            let document = object(scope, "document");
            let badge = object(scope, "badge");
            let handle = handle_of(scope, badge).unwrap();
            let sample = sample_presented_effects(scope, document, &[handle]).unwrap();
            let target = sample
                .targets
                .iter()
                .find(|target| target.element == handle)
                .unwrap();
            assert!(
                matches!(target.presentation, PresentedTargetState::Unknown { .. }),
                "{mode}: {sample:?}"
            );
            assert!(eval(scope, "__layoutCalls===0").is_true());
        });
    }
}

#[test]
fn captured_auth_label_length_percentage_transitions_are_certified() {
    with_html(
        true,
        true,
        include_str!("fixtures/auth_label_transitions.html"),
        |scope| {
            eval(scope,"globalThis.label=document.getElementById('label');globalThis.legend=document.getElementById('legend');getComputedStyle(label).maxWidth;getComputedStyle(legend).maxWidth;label.className='focused';legend.className='focused';__layoutCalls=0");
            let document = object(scope, "document");
            let label = object(scope, "label");
            let legend = object(scope, "legend");
            let handles = [
                handle_of(scope, label).unwrap(),
                handle_of(scope, legend).unwrap(),
            ];
            let initial = sample_presented_effects(scope, document, &handles).unwrap();
            assert!(
                initial.targets.iter().all(|target| matches!(
                    target.presentation,
                    PresentedTargetState::Certified { .. }
                )),
                "{initial:?}"
            );
            assert!(eval(scope, "__layoutCalls===0").is_true());
            eval(scope,"for(const element of [label,legend])for(const effect of element.getAnimations()){effect.pause();effect.currentTime=100;}__layoutCalls=0;");
            let sampled = sample_presented_effects(scope, document, &handles).unwrap();
            assert!(
                sampled.targets.iter().all(|target| matches!(
                    target.presentation,
                    PresentedTargetState::Certified { .. }
                )),
                "{sampled:?}"
            );
            assert!(eval(scope, "__layoutCalls===0").is_true());
        },
    );
}

fn length_percentage_coefficients(text: &str) -> (f64, f64) {
    if let Some(body) = text
        .strip_prefix("calc(")
        .and_then(|text| text.strip_suffix(')'))
    {
        let parts: Vec<_> = body.split_whitespace().collect();
        let percent = parts[0].trim_end_matches('%').parse::<f64>().unwrap();
        let px = parts[2].trim_end_matches("px").parse::<f64>().unwrap();
        return (percent, if parts[1] == "-" { -px } else { px });
    }
    if let Some(percent) = text.strip_suffix('%') {
        return (percent.parse().unwrap(), 0.0);
    }
    (0.0, text.strip_suffix("px").unwrap().parse().unwrap())
}

#[test]
fn captured_auth_transitions_match_chrome_symbolic_coefficients() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/auth_label_transitions_chrome159.json"
    ))
    .unwrap();
    let html = include_str!("fixtures/auth_label_transitions.html");
    assert_eq!(
        format!("{:x}", Sha256::digest(html)),
        oracle["sha256"].as_str().unwrap()
    );
    with_html(true, true, html, |scope| {
        eval(scope,"globalThis.label=document.getElementById('label');globalThis.legend=document.getElementById('legend');getComputedStyle(label).maxWidth;getComputedStyle(legend).maxWidth;label.className='focused';legend.className='focused';globalThis.motions=[...label.getAnimations(),...legend.getAnimations()];for(const effect of motions)effect.pause();");
        let document = object(scope, "document");
        let label = object(scope, "label");
        let legend = object(scope, "legend");
        let handles = [
            handle_of(scope, label).unwrap(),
            handle_of(scope, legend).unwrap(),
        ];
        for row in oracle["result"].as_array().unwrap() {
            let time = row["time"].as_u64().unwrap();
            eval(
                scope,
                &format!("for(const effect of motions)effect.currentTime={time};__layoutCalls=0;"),
            );
            let sample = sample_presented_effects(scope, document, &handles).unwrap();
            for (index, target) in sample.targets.iter().enumerate() {
                let PresentedTargetState::Certified { transition, .. } = &target.presentation
                else {
                    panic!("{sample:?}")
                };
                let actual = transition
                    .iter()
                    .find(|pair| pair.name == "max-width")
                    .map(|pair| pair.value.as_str());
                // At the active end the transition no longer contributes.
                if let Some(actual) = actual {
                    let expected = row["targets"][index]["maxWidth"].as_str().unwrap();
                    let (ap, ax) = length_percentage_coefficients(actual);
                    let (ep, ex) = length_percentage_coefficients(expected);
                    assert!(
                        (ap - ep).abs() < 0.001 && (ax - ex).abs() < 0.001,
                        "{time}: {actual} != {expected}"
                    );
                }
            }
            assert!(eval(scope, "__layoutCalls===0").is_true());
        }
    });
}

#[test]
fn symbolic_lengths_compose_without_resolving_a_percentage_basis() {
    with_dom(|scope| {
        eval(scope,"globalThis.target=document.getElementById('target');globalThis.motion=target.animate([{width:'calc(10% - 2px)'},{width:'calc(30% + 2px)'}],{duration:1000,fill:'both',composite:'add'});motion.pause();motion.currentTime=500;__layoutCalls=0;");
        let document = object(scope, "document");
        let target = object(scope, "target");
        let handle = handle_of(scope, target).unwrap();
        let sample = sample_presented_effects(scope, document, &[handle]).unwrap();
        let PresentedTargetState::Certified { animation, .. } = &sample.targets[0].presentation
        else {
            panic!("{sample:?}")
        };
        assert_eq!(
            animation
                .iter()
                .find(|pair| pair.name == "width")
                .unwrap()
                .value,
            "calc(20% + 100px)"
        );
        assert!(eval(scope, "__layoutCalls===0").is_true());
        assert_eq!(
            eval(scope, "getComputedStyle(target).width").to_rust_string_lossy(scope),
            "calc(20% + 100px)"
        );
    });
}

#[test]
fn unproved_length_and_transform_bases_remain_unknown() {
    for (property, value) in [
        ("width", "auto"),
        ("width", "min(10%, 20px)"),
        ("width", "max(10%, 20px)"),
        ("width", "clamp(10px, 20%, 30px)"),
        ("width", "calc(1em + 10%)"),
        ("transform", "translateX(calc(50% + 1em))"),
    ] {
        with_dom(|scope| {
            eval(scope,&format!("globalThis.target=document.getElementById('target');globalThis.motion=target.animate([{{'{property}':'{value}'}},{{'{property}':'{value}'}}],{{duration:1000,fill:'both'}});motion.pause();motion.currentTime=500;__layoutCalls=0;"));
            let document = object(scope, "document");
            let target = object(scope, "target");
            let handle = handle_of(scope, target).unwrap();
            let sample = sample_presented_effects(scope, document, &[handle]).unwrap();
            assert!(
                matches!(
                    sample.targets[0].presentation,
                    PresentedTargetState::Unknown { .. }
                ),
                "{property}:{value}: {sample:?}"
            );
            assert!(eval(scope, "__layoutCalls===0").is_true());
        });
    }
}

#[test]
fn symbolic_math_bridge_refuses_nonfinite_or_nonlinear_coefficients() {
    with_dom(|scope| {
        browser_env::run_engine_script(scope,"globalThis.bridgeRejects=['calc(infinity * 1px)','calc(NaN * 1px)','calc(1px / 0)','calc(1deg + 2deg)','min(1px,2%)','calc(1px + max(2px,3%))'].every(value=>__tsNs.animationLengthPercentage(value)===null);globalThis.bridgePrivate=Object.getPrototypeOf(__tsNs.animationLengthPercentage('calc(100% - 24px)'))===null;").unwrap();
        assert!(eval(scope, "bridgeRejects&&bridgePrivate").is_true());
    });
}

#[test]
fn discrete_none_to_length_transition_is_not_certified_as_a_symbolic_length() {
    with_dom(|scope| {
        eval(scope,"globalThis.target=document.getElementById('target');target.style.maxWidth='none';target.style.transition='max-width 1000ms linear allow-discrete';getComputedStyle(target).maxWidth;target.style.maxWidth='100px';__layoutCalls=0;");
        let document = object(scope, "document");
        let target = object(scope, "target");
        let handle = handle_of(scope, target).unwrap();
        let sample = sample_presented_effects(scope, document, &[handle]).unwrap();
        assert!(
            matches!(
                sample.targets[0].presentation,
                PresentedTargetState::Unknown { .. }
            ),
            "{sample:?}"
        );
        assert!(eval(scope, "__layoutCalls===0").is_true());
    });
}

#[test]
fn discovers_paused_waapi_without_css_candidates_and_never_calls_layout() {
    with_dom(|scope| {
        eval(scope,"globalThis.target=document.getElementById('target');globalThis.effect=target.animate([{width:'100px'},{width:'200px'}],{duration:1000,fill:'both'});effect.pause();effect.currentTime=500");
        let document = object(scope, "document");
        let target = object(scope, "target");
        let handle = handle_of(scope, target).unwrap();
        let sample = sample_presented_effects(scope, document, &[])
            .expect("canonical WAAPI target discovered independently of CSS candidates");
        assert_eq!(sample.targets.len(), 1);
        assert_eq!(sample.targets[0].element, handle);
        let PresentedTargetState::Certified {
            animation,
            transition,
        } = &sample.targets[0].presentation
        else {
            panic!("absolute paused width must be certified: {sample:?}");
        };
        assert_eq!(
            animation,
            &[PresentedProperty {
                name: "width".into(),
                value: "150px".into()
            }]
        );
        assert!(transition.is_empty());
        assert!(eval(scope, "__layoutCalls===0").is_true());
    });
}

#[test]
fn canonical_context_evidence_includes_delay_and_distinguishes_css_ownership() {
    let probe = include_str!("fixtures/presented_context_phases_probe.js");
    let control: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/presented_context_phases_chrome159.json"
    ))
    .unwrap();
    assert!(control["browser"].as_str().unwrap().starts_with("159."));
    assert_eq!(
        control["probeSha256"],
        format!("{:x}", Sha256::digest(probe.as_bytes()))
    );
    let cases = control["result"].as_array().unwrap();
    assert_eq!(cases.len(), 14);
    for case in cases {
        let css = case["kind"] == "css";
        let action = phase_action(&case["action"]);
        with_dom(|scope| {
            let setup = if css {
                "const style=document.createElement('style');style.textContent='@keyframes context{from{opacity:.5}to{opacity:1}}';document.head.append(style);target.style.animation='context 1000ms 1000ms linear';globalThis.motion=target.getAnimations()[0]"
            } else {
                "globalThis.motion=target.animate([{opacity:.5},{opacity:1}],{duration:1000,delay:1000,fill:'none'})"
            };
            eval(scope, &format!("globalThis.target=document.getElementById('target');{setup};{action};__layoutCalls=0"));
            let document = object(scope, "document");
            let target = object(scope, "target");
            let handle = handle_of(scope, target).unwrap();
            let sample = sample_presented_effects(scope, document, &[handle]).unwrap();
            assert_eq!(
                sample.targets[0].establishes_stacking_context,
                case["hit"] == "cover",
                "{case:?}"
            );
            assert!(eval(scope, "__layoutCalls===0").is_true());
        });
    }
}

fn phase_action(action: &serde_json::Value) -> String {
    if let Some(time) = action.as_i64() {
        format!("motion.pause();motion.currentTime={time}")
    } else if action == "finish" {
        "motion.finish()".into()
    } else {
        "motion.cancel()".into()
    }
}

#[test]
fn context_metadata_obeys_presentation_policy_and_live_css_rule_ownership() {
    with_dom(|scope| {
        eval(scope, "globalThis.target=document.getElementById('target');const style=document.createElement('style');style.textContent='@keyframes context{from{opacity:1}to{opacity:1}}';document.head.append(style);target.style.animation='context 1000ms linear both';target.style.setProperty('opacity','1','important');globalThis.motion=target.getAnimations()[0];motion.pause();motion.currentTime=500");
        let document = object(scope, "document");
        let target = object(scope, "target");
        let handle = handle_of(scope, target).unwrap();
        assert!(
            sample_presented_effects(scope, document, &[handle])
                .unwrap()
                .targets[0]
                .establishes_stacking_context
        );
        set_animation_presentation(scope, AnimationPresentation::Disabled);
        assert!(
            !sample_presented_effects(scope, document, &[handle])
                .unwrap()
                .targets[0]
                .establishes_stacking_context
        );
        set_animation_presentation(scope, AnimationPresentation::Enabled);
        eval(scope, "target.style.animation='none'");
        assert!(
            !sample_presented_effects(scope, document, &[handle])
                .unwrap()
                .targets[0]
                .establishes_stacking_context
        );
    });
}

#[test]
fn context_metadata_preserves_existing_effects_when_css_creation_is_disabled() {
    with_dom(|scope| {
        eval(scope, "globalThis.target=document.getElementById('target');const style=document.createElement('style');style.textContent='@keyframes context{from{opacity:.5}to{opacity:1}}';document.head.append(style);target.style.animation='context 1000ms 1000ms linear';globalThis.motion=target.getAnimations()[0];motion.pause();motion.currentTime=500");
        browser_env::set_css_animations(scope, false);
        assert!(eval(scope, "document.getAnimations().includes(motion)").is_true());
        let document = object(scope, "document");
        let target = object(scope, "target");
        let handle = handle_of(scope, target).unwrap();
        let sample = sample_presented_effects(scope, document, &[handle]).unwrap();
        assert!(
            sample.targets[0].establishes_stacking_context,
            "canonical retained before-phase effect: {sample:?}"
        );
    });
    with_dom(|scope| {
        browser_env::set_css_animations(scope, false);
        eval(scope, "globalThis.target=document.getElementById('target');const style=document.createElement('style');style.textContent='@keyframes context{from{opacity:.5}to{opacity:1}}';document.head.append(style);target.style.animation='context 1000ms 1000ms linear'");
        assert!(eval(scope, "document.getAnimations().length===0").is_true());
        let document = object(scope, "document");
        let target = object(scope, "target");
        let handle = handle_of(scope, target).unwrap();
        let sample = sample_presented_effects(scope, document, &[handle]).unwrap();
        assert!(!sample.targets[0].establishes_stacking_context);
    });
}

fn snapshot_params() -> v8::CreateParams {
    match std::env::var_os("TURBO_DOM_SNAPSHOT_PATH").and(browser_env::snapshot_blob()) {
        Some(blob) => blob.create_params(),
        None => Default::default(),
    }
}

#[test]
fn default_document_zero_has_a_native_empty_certificate_without_guest_getters() {
    with_dom_using(false, |scope| {
        let document = object(scope, "document");
        let snapshot =
            sample_presented_effects(scope, document, &[]).expect("native default document zero");
        assert_eq!(snapshot.document_id, 0);
        assert!(snapshot.targets.is_empty());
        let fake = object(scope, "({})");
        assert!(sample_presented_effects(scope, fake, &[]).is_none());
        let proxy = object(scope, "globalThis.__traps=0;new Proxy(document,{get(){__traps++;throw Error('guest getter')}})");
        assert!(sample_presented_effects(scope, proxy, &[]).is_none());
        assert!(eval(scope, "__traps===0 && __layoutCalls===0").is_true());
    });
}

#[test]
fn css_animation_and_important_transition_keep_separate_cascade_origins() {
    with_dom(|scope| {
        eval(scope,"globalThis.target=document.getElementById('target');const style=document.createElement('style');style.textContent='@keyframes grow{from{width:100px}to{width:200px}}';document.head.append(style);target.style.animation='grow 1000ms linear both';globalThis.motion=target.getAnimations()[0];motion.pause();motion.currentTime=500;__layoutCalls=0");
        let document = object(scope, "document");
        let target = object(scope, "target");
        let handle = handle_of(scope, target).unwrap();
        let sample = sample_presented_effects(scope, document, &[handle]).unwrap();
        let PresentedTargetState::Certified {
            animation,
            transition,
        } = &sample.targets[0].presentation
        else {
            panic!("literal CSS width must be certified: {sample:?}");
        };
        assert_eq!(
            animation,
            &[PresentedProperty {
                name: "width".into(),
                value: "150px".into()
            }]
        );
        assert!(transition.is_empty());
        assert!(eval(scope, "__layoutCalls===0").is_true());
        eval(scope,"target.style.animation='none';target.style.transition='width 1000ms linear';target.style.setProperty('width','100px','important');getComputedStyle(target).width;target.style.setProperty('width','200px','important');globalThis.transition=target.getAnimations().find(a=>a instanceof CSSTransition);transition.pause();transition.currentTime=500;__layoutCalls=0");
        let sample = sample_presented_effects(scope, document, &[handle]).unwrap();
        let PresentedTargetState::Certified {
            animation,
            transition,
        } = &sample.targets[0].presentation
        else {
            panic!("literal transition must be certified: {sample:?}");
        };
        assert!(animation.is_empty());
        assert_eq!(
            transition,
            &[PresentedProperty {
                name: "width".into(),
                value: "150px".into()
            }]
        );
        assert!(eval(scope, "__layoutCalls===0").is_true());
    });
}

#[test]
fn entire_target_batch_uses_one_canonical_clock_sample() {
    with_dom(|scope| {
        eval(scope,"for(let i=0;i<2;i++){const el=document.createElement('div');el.style.width='100px';document.body.append(el);const a=el.animate([{width:'100px'},{width:'200px'}],{duration:1000,fill:'both'});a.pause();a.currentTime=500}");
        eval(scope, "__clockCalls=0;__steppingClock=true");
        let document = object(scope, "document");
        let sample = sample_presented_effects(scope, document, &[]).unwrap();
        assert_eq!(sample.targets.len(), 2);
        assert_eq!(sample.sample_time, Some(1001.0));
        assert!(eval(scope, "__clockCalls===1 && __layoutCalls===0").is_true());
    });
}

#[test]
fn reference_box_translation_stays_symbolic_without_layout() {
    with_dom(|scope| {
        eval(scope,"const target=document.getElementById('target');const a=target.animate([{transform:'translateX(0%)'},{transform:'translateX(50%)'}],{duration:1000,fill:'both'});a.pause();a.currentTime=500");
        let document = object(scope, "document");
        let sample = sample_presented_effects(scope, document, &[]).unwrap();
        let PresentedTargetState::Certified {
            animation,
            transition,
        } = &sample.targets[0].presentation
        else {
            panic!("matching symbolic reference-box transform must be certified: {sample:?}");
        };
        assert!(transition.is_empty());
        assert_eq!(
            animation,
            &[PresentedProperty {
                name: "transform".into(),
                value: "translate(25%, 0px)".into()
            }]
        );
        assert!(eval(scope, "__layoutCalls===0").is_true());
    });
}

fn expose(scope: &mut v8::PinScope, name: &str, value: v8::Local<v8::Object>) {
    let key = v8::String::new(scope, name).unwrap();
    let global = scope.get_current_context().global(scope);
    assert_eq!(
        global.define_own_property(scope, key.into(), value.into(), v8::PropertyAttribute::NONE),
        Some(true)
    );
}

#[test]
fn borrowed_foreign_animation_cannot_become_an_empty_owner_certificate() {
    with_dom(|scope| {
        let document = object(scope, "document");
        let target = object(scope, "document.getElementById('target')");
        let foreign = v8::Context::new(scope, Default::default());
        {
            let inner = &mut v8::ContextScope::new(scope, foreign);
            browser_env::install_independent_html(inner, "<!doctype html><main>other</main>");
            expose(inner, "foreignTarget", target);
            eval(inner, "globalThis.motion=Element.prototype.animate.call(foreignTarget,[{width:'100px'},{width:'200px'}],{duration:1000,fill:'both'});motion.pause();motion.currentTime=500");
        }
        assert!(sample_presented_effects(scope, document, &[]).is_none());
        assert!(eval(scope, "__layoutCalls===0").is_true());
    });
}

#[test]
fn adopting_an_animated_target_refuses_the_new_owner_without_rooting_a_tree_copy() {
    with_dom(|scope| {
        eval(scope, "globalThis.target=document.getElementById('target');globalThis.motion=target.animate([{width:'100px'},{width:'200px'}],{duration:1000,fill:'both'});motion.pause();motion.currentTime=500");
        let document = object(scope, "document");
        let target = object(scope, "target");
        assert_eq!(
            sample_presented_effects(scope, document, &[])
                .unwrap()
                .targets
                .len(),
            1
        );
        let foreign = v8::Context::new(scope, Default::default());
        {
            let inner = &mut v8::ContextScope::new(scope, foreign);
            browser_env::install_independent_html(inner, "<!doctype html><main>other</main>");
            expose(inner, "foreignTarget", target);
            eval(
                inner,
                "document.body.appendChild(document.adoptNode(foreignTarget))",
            );
            let new_document = object(inner, "document");
            assert!(sample_presented_effects(inner, new_document, &[]).is_none());
        }
        assert!(sample_presented_effects(scope, document, &[])
            .unwrap()
            .targets
            .is_empty());
    });
}

fn animated_value<'a>(target: &'a PresentedTarget, property: &str) -> &'a str {
    let PresentedTargetState::Certified { animation, .. } = &target.presentation else {
        panic!("expected certified presentation: {target:?}");
    };
    &animation
        .iter()
        .find(|pair| pair.name == property)
        .unwrap_or_else(|| panic!("missing {property}: {target:?}"))
        .value
}

#[test]
fn closed_shadow_keyframes_use_their_actual_scope_and_native_owner() {
    with_dom(|scope| {
        eval(scope,"const host=document.getElementById('target');globalThis.shadow=host.attachShadow({mode:'closed'});shadow.innerHTML='<style>@keyframes grow{from{width:100px}to{width:200px}}#inside{width:100px;animation:grow 1000ms linear both}</style><div id=inside></div>';globalThis.inside=shadow.querySelector('#inside');const a=inside.getAnimations()[0];a.pause();a.currentTime=500;__layoutCalls=0");
        let document = object(scope, "document");
        let inside = object(scope, "inside");
        let handle = handle_of(scope, inside).unwrap();
        let sample = sample_presented_effects(scope, document, &[handle]).unwrap();
        assert_eq!(sample.targets.len(), 1);
        assert_eq!(sample.targets[0].element, handle);
        assert_eq!(animated_value(&sample.targets[0], "width"), "150px");
        assert!(eval(scope, "__layoutCalls===0").is_true());
    });
}

#[test]
fn absolute_inherited_font_and_unregistered_custom_tokens_are_origin_inputs() {
    with_dom(|scope| {
        eval(scope,"globalThis.target=document.getElementById('target');const s=document.createElement('style');s.textContent='@keyframes text{from{font-size:10px;--measure:100px}to{font-size:30px;--measure:200px}}#target{--measure:100px;width:var(--measure);animation:text 1000ms linear both}';document.head.append(s);target.innerHTML='<span>inherits presented font</span>';const a=target.getAnimations()[0];a.pause();a.currentTime=500;__layoutCalls=0");
        let document = object(scope, "document");
        let target = object(scope, "target");
        let handle = handle_of(scope, target).unwrap();
        let sample = sample_presented_effects(scope, document, &[handle]).unwrap();
        assert!(eval(scope, "__layoutCalls===0").is_true());
        assert!(eval(
            scope,
            "target.getAnimations()[0].effect.getKeyframes()[0]['--measure']===undefined && getComputedStyle(target).getPropertyValue('--measure')==='200px'"
        )
        .is_true());
        assert_eq!(animated_value(&sample.targets[0], "font-size"), "20px");
        assert_eq!(animated_value(&sample.targets[0], "--measure"), "200px");
    });
}

#[test]
fn independent_compatibility_switches_preserve_script_and_presentation_policy() {
    with_dom(|scope| {
        eval(scope,"globalThis.target=document.getElementById('target');globalThis.motion=target.animate([{width:'100px'},{width:'200px'}],{duration:1000,fill:'both'});motion.pause();motion.currentTime=500");
        browser_env::set_css_animations(scope, false);
        browser_env::set_css_transitions(scope, false);
        browser_env::set_media_mode(scope, browser_env::MediaMode::Jsdom);
        browser_env::set_touch_event_mode(scope, browser_env::TouchEventMode::Jsdom);
        browser_env::set_computed_style_mode(scope, browser_env::ComputedStyleMode::Jsdom);
        let document = object(scope, "document");
        let sample = sample_presented_effects(scope, document, &[]).unwrap();
        assert!(!sample.css_animations_enabled);
        assert!(!sample.css_transitions_enabled);
        assert_eq!(animated_value(&sample.targets[0], "width"), "150px");
        assert!(eval(scope, "typeof Element.prototype.animate==='undefined'").is_true());
        set_animation_presentation(scope, AnimationPresentation::Disabled);
        let suppressed = sample_presented_effects(scope, document, &[]).unwrap();
        assert!(suppressed.presentation_suppressed);
        let PresentedTargetState::Certified {
            animation,
            transition,
        } = &suppressed.targets[0].presentation
        else {
            panic!("trusted presentation suppression");
        };
        assert!(animation.is_empty() && transition.is_empty());
        set_animation_presentation(scope, AnimationPresentation::Enabled);
        let restored = sample_presented_effects(scope, document, &[]).unwrap();
        assert_eq!(animated_value(&restored.targets[0], "width"), "150px");
        assert!(eval(scope, "motion.currentTime===500 && __layoutCalls===0").is_true());
    });
}

#[test]
fn important_keyframe_declarations_do_not_become_presented_width() {
    with_dom(|scope| {
        eval(scope,"globalThis.target=document.getElementById('target');const s=document.createElement('style');s.textContent='@keyframes ignored{from{width:100px!important}to{width:200px!important}}#target{width:300px;animation:ignored 1000ms linear both}';document.head.append(s);const a=target.getAnimations()[0];a.pause();a.currentTime=500;__layoutCalls=0");
        let document = object(scope, "document");
        let target = object(scope, "target");
        let handle = handle_of(scope, target).unwrap();
        let sample = sample_presented_effects(scope, document, &[handle]).unwrap();
        assert!(eval(scope, "__layoutCalls===0").is_true());
        let PresentedTargetState::Certified {
            animation,
            transition,
        } = &sample.targets[0].presentation
        else {
            panic!("ignored important keyframes must have no presentation");
        };
        assert!(animation.is_empty() && transition.is_empty(), "{sample:?}");
        assert!(eval(scope,"target.getAnimations()[0].effect.getKeyframes().every(frame=>!Object.hasOwn(frame,'width'))").is_true());
    });
}

#[test]
fn unmodeled_css_declarations_cannot_be_certified_as_an_empty_effect() {
    with_dom(|scope| {
        eval(scope,"globalThis.target=document.getElementById('target');const s=document.createElement('style');s.textContent='@keyframes floatIt{from{float:left}to{float:right}}#target{animation:floatIt 1000ms linear both}';document.head.append(s);const a=target.getAnimations()[0];a.pause();a.currentTime=500;__layoutCalls=0");
        let document = object(scope, "document");
        let target = object(scope, "target");
        let handle = handle_of(scope, target).unwrap();
        let sample = sample_presented_effects(scope, document, &[handle]).unwrap();
        assert!(
            matches!(
                sample.targets[0].presentation,
                PresentedTargetState::Unknown { .. }
            ),
            "unmodeled float must not become an empty certificate: {sample:?}"
        );
        assert!(eval(scope, "__layoutCalls===0").is_true());
    });
}

#[test]
fn registered_custom_properties_are_unknown_instead_of_unregistered_discrete_values() {
    with_dom(|scope| {
        eval(
            scope,
            r#"globalThis.target=document.getElementById('target');const s=document.createElement('style');s.textContent='@property --measure{syntax: "<length>";inherits:false;initial-value:0px}@keyframes measure{from{--measure:100px}to{--measure:200px}}#target{animation:measure 1000ms linear both}';document.head.append(s);const a=target.getAnimations()[0];a.pause();a.currentTime=500;__layoutCalls=0"#,
        );
        let document = object(scope, "document");
        let target = object(scope, "target");
        let handle = handle_of(scope, target).unwrap();
        let sample = sample_presented_effects(scope, document, &[handle]).unwrap();
        assert!(
            matches!(
                sample.targets[0].presentation,
                PresentedTargetState::Unknown { .. }
            ),
            "registered interpolation must not be certified discrete: {sample:?}"
        );
        assert!(eval(scope, "__layoutCalls===0").is_true());
    });
}

#[test]
fn repaired_non_scalar_custom_payload_is_unknown_instead_of_a_lossy_certificate() {
    with_dom(|scope| {
        eval(
            scope,
            r#"const target=document.getElementById('target'),raw='"'+String.fromCharCode(0xd800)+'"';const a=target.animate([{'--unit':raw},{'--unit':raw}],{duration:1000,fill:'both'});a.pause();a.currentTime=500;__layoutCalls=0"#,
        );
        let document = object(scope, "document");
        let sample = sample_presented_effects(scope, document, &[]).unwrap();
        assert!(
            matches!(
                sample.targets[0].presentation,
                PresentedTargetState::Unknown { .. }
            ),
            "lossy native value transport must not be certified: {sample:?}"
        );
        assert!(eval(scope, "__layoutCalls===0").is_true());
    });
}

#[test]
fn presented_sampling_survives_startup_snapshot_rehydration() {
    init_v8();
    let mut window = browser_env::install_from_snapshot(
        "<!doctype html><div id=target style='width:100px'></div>",
    )
    .unwrap();
    let (isolate, context) = window.parts();
    isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
    {
        v8::scope!(let scope, isolate);
        let context = v8::Local::new(scope, context);
        let scope = &mut v8::ContextScope::new(scope, context);
        browser_env::run_engine_script(scope,"globalThis.__layoutCalls=0;__tsNs.layoutUsedSize=()=>{__layoutCalls++;throw Error('provider called')};__tsNs.resolveTransform=()=>{__layoutCalls++;throw Error('transform called')}").unwrap();
        eval(scope,"const target=document.getElementById('target');const a=target.animate([{width:'100px'},{width:'200px'}],{duration:1000,fill:'both'});a.pause();a.currentTime=500");
        let document = object(scope, "document");
        recover_after_termination(scope).expect("rehydrated private recovery callback");
        let sample =
            sample_presented_effects(scope, document, &[]).expect("rehydrated canonical sampler");
        assert_eq!(sample.targets.len(), 1);
        assert_eq!(animated_value(&sample.targets[0], "width"), "150px");
        assert!(eval(scope, "__layoutCalls===0").is_true());
    }
    browser_env::reset();
}

#[test]
fn svg_unitless_dash_offsets_use_canonical_length_interpolation() {
    with_dom(|scope| {
        eval(scope,"const s=document.createElement('style');s.textContent='@keyframes dash{from{stroke-dashoffset:0}to{stroke-dashoffset:100}}#shape{animation:dash 1000ms linear infinite}';document.head.append(s);document.body.innerHTML='<svg width=100 height=100><circle id=shape cx=50 cy=50 r=30 /></svg>';globalThis.shape=document.getElementById('shape');const a=shape.getAnimations()[0];a.pause();a.currentTime=500;__layoutCalls=0");
        let document = object(scope, "document");
        let shape = object(scope, "shape");
        let handle = handle_of(scope, shape).unwrap();
        let sample = sample_presented_effects(scope, document, &[handle]).unwrap();
        assert_eq!(
            animated_value(&sample.targets[0], "stroke-dashoffset"),
            "50px"
        );
        assert!(eval(scope, "__layoutCalls===0").is_true());
    });
}

#[test]
fn unregistered_custom_names_do_not_inherit_standard_property_interpolation_types() {
    with_dom(|scope| {
        eval(scope,"const target=document.getElementById('target');const a=target.animate([{'--width':'100px'},{'--width':'200px'}],{duration:1000,fill:'both'});a.pause();a.currentTime=500;__layoutCalls=0");
        let document = object(scope, "document");
        let sample = sample_presented_effects(scope, document, &[]).unwrap();
        assert_eq!(animated_value(&sample.targets[0], "--width"), "200px");
        assert!(eval(scope, "__layoutCalls===0").is_true());
    });
}

#[test]
fn simultaneous_isolates_keep_animation_ownership_separate() {
    with_dom(|outer| {
        eval(outer, "globalThis.effect=document.getElementById('target').animate([{width:'100px'},{width:'200px'}],{duration:1000,fill:'both'});effect.pause();effect.currentTime=500");
        // Creating a nested isolate enters it; dropping it restores the outer isolate.
        with_dom_lifecycle(true, false, |inner| {
            eval(inner, "globalThis.effect=document.getElementById('target').animate([{width:'100px'},{width:'300px'}],{duration:1000,fill:'both'});effect.pause();effect.currentTime=500");
            let document = object(inner, "document");
            let snapshot = sample_presented_effects(inner, document, &[]).unwrap();
            assert_eq!(snapshot.targets.len(), 1);
            assert!(matches!(&snapshot.targets[0].presentation,
                PresentedTargetState::Certified { animation, .. }
                if animation.iter().any(|pair| pair.name == "width" && pair.value == "200px")));
        });
        assert!(eval(outer, "effect.cancel();effect.playState==='idle'").is_true());
    });
    // A replacement isolate must not inherit weak handles from either destroyed isolate.
    discovers_paused_waapi_without_css_candidates_and_never_calls_layout();
}

#[test]
fn inherited_animated_context_cannot_certify_a_suppressed_additive_base() {
    with_dom(|scope| {
        eval(scope, "document.body.style.fontSize='10px';target=document.getElementById('target');target.style.width='calc(10em * sibling-index())';let parentEffect=document.body.animate([{fontSize:'10px'},{fontSize:'30px'}],{duration:1000,fill:'both'});parentEffect.pause();parentEffect.currentTime=500;let childEffect=target.animate([{width:'10px'},{width:'20px'}],{duration:1000,fill:'both',composite:'add'});childEffect.pause();childEffect.currentTime=500");
        let document = object(scope, "document");
        let target = object(scope, "target");
        let handle = handle_of(scope, target).unwrap();
        let sample = sample_presented_effects(scope, document, &[]).unwrap();
        let target = sample
            .targets
            .iter()
            .find(|target| target.element == handle)
            .unwrap();
        assert!(
            matches!(target.presentation, PresentedTargetState::Unknown { .. }),
            "suppressed 10em base cannot certify presented inherited font: {target:?}"
        );
    });
}

#[test]
fn imported_registration_cannot_certify_unregistered_custom_interpolation() {
    with_dom(|scope| {
        eval(
            scope,
            r#"let sheet=document.createElement('style');sheet.textContent='@import url("data:text/css,@property%20--size%7Bsyntax:%22%3Clength%3E%22;inherits:false;initial-value:0px%7D");';document.head.appendChild(sheet);target=document.getElementById('target');let effect=target.animate([{'--size':'100px'},{'--size':'200px'}],{duration:1000,fill:'both'});effect.pause();effect.currentTime=500"#,
        );
        let document = object(scope, "document");
        let sample = sample_presented_effects(scope, document, &[]).unwrap();
        assert!(
            matches!(
                sample.targets[0].presentation,
                PresentedTargetState::Unknown { .. }
            ),
            "imported registration completeness is unknown: {sample:?}"
        );
    });
}

#[test]
fn canonical_sampling_preserves_internal_exceptions() {
    with_dom(|scope| {
        browser_env::run_engine_script(
            scope,
            "__tsNs.refreshCssEffectInputs=()=>{throw Error('canonical sampling sentinel')}",
        )
        .unwrap();
        let document = object(scope, "document");
        let target = object(scope, "document.getElementById('target')");
        let handle = handle_of(scope, target).unwrap();
        v8::tc_scope!(let scope, scope);
        assert!(sample_presented_effects(scope, document, &[handle]).is_none());
        assert!(
            scope.has_caught(),
            "canonical failures must not become an unavailable certificate"
        );
        let exception = scope.exception().unwrap();
        assert!(exception
            .to_rust_string_lossy(scope)
            .contains("canonical sampling sentinel"));
    });
}

fn presented_animation_pairs(scope: &mut v8::PinScope) -> Vec<PresentedProperty> {
    let document = object(scope, "document");
    let sample = sample_presented_effects(scope, document, &[]).expect("owned canonical sample");
    assert_eq!(sample.targets.len(), 1);
    let PresentedTargetState::Certified { animation, .. } = &sample.targets[0].presentation else {
        panic!("modeled absolute effect must be certified: {sample:?}");
    };
    animation.clone()
}

#[test]
fn actual_mui_svg_dasharray_uses_canonical_piecewise_list_interpolation() {
    with_dom(|scope| {
        eval(scope, "document.body.innerHTML='<svg width=100 height=100><circle id=subject cx=50 cy=50 r=30 fill=none stroke=red /></svg>';const s=document.createElement('style');s.textContent='@keyframes motion{0%{stroke-dasharray:1px,200px;stroke-dashoffset:0px}50%{stroke-dasharray:100px,200px;stroke-dashoffset:-15px}100%{stroke-dasharray:1px,200px;stroke-dashoffset:-126px}}#subject{animation:motion 1000ms linear both}';document.head.append(s);globalThis.subject=document.getElementById('subject');globalThis.effect=subject.getAnimations()[0];effect.pause();__layoutCalls=0");
        for (time, array, offset) in [
            (0, "1px, 200px", "0px"),
            (250, "50.5px, 200px", "-7.5px"),
            (500, "100px, 200px", "-15px"),
            (750, "50.5px, 200px", "-70.5px"),
            (1000, "1px, 200px", "-126px"),
        ] {
            eval(scope, &format!("effect.currentTime={time};__layoutCalls=0"));
            let pairs = presented_animation_pairs(scope);
            for (name, value) in [("stroke-dasharray", array), ("stroke-dashoffset", offset)] {
                assert!(
                    pairs.contains(&PresentedProperty {
                        name: name.into(),
                        value: value.into()
                    }),
                    "{time} {pairs:?}"
                );
            }
            assert!(eval(scope, "__layoutCalls===0").is_true());
            assert!(eval(scope, &format!("getComputedStyle(subject).strokeDasharray==='{array}' && getComputedStyle(subject).strokeDashoffset==='{offset}'")).is_true());
        }
    });
}

#[test]
fn repeatable_svg_lists_normalize_units_and_repeat_to_least_common_multiple() {
    with_dom(|scope| {
        eval(scope, "document.body.innerHTML='<svg><circle id=subject /></svg>';globalThis.subject=document.getElementById('subject')");
        for (from, to, expected) in [
            ("1 200", "100 200", "50.5px, 200px"),
            ("2 4", "10 20 30", "6px, 12px, 16px, 7px, 11px, 17px"),
            ("2", "10 20 30", "6px, 11px, 16px"),
            ("1in 2px", "192px 4px", "144px, 3px"),
            ("10% 20%", "30% 40%", "20%, 30%"),
        ] {
            eval(scope, &format!("if(globalThis.effect)effect.cancel();globalThis.effect=subject.animate([{{strokeDasharray:'{from}'}},{{strokeDasharray:'{to}'}}],{{duration:1000,fill:'both'}});effect.pause();effect.currentTime=500;__layoutCalls=0"));
            let pairs = presented_animation_pairs(scope);
            assert!(
                pairs.contains(&PresentedProperty {
                    name: "stroke-dasharray".into(),
                    value: expected.into()
                }),
                "{from} -> {to}: {pairs:?}"
            );
            assert!(eval(scope, "__layoutCalls===0").is_true());
            assert!(eval(
                scope,
                &format!("getComputedStyle(subject).strokeDasharray==='{expected}'")
            )
            .is_true());
        }
    });
}

#[test]
fn svg_list_uncertainties_remain_unknown_and_nonadditive_values_do_not_sum() {
    with_dom(|scope| {
        eval(scope, "document.body.innerHTML='<svg><circle id=subject style=\"stroke-dasharray:1,2\" /></svg>';globalThis.subject=document.getElementById('subject')");
        for composite in ["add", "accumulate"] {
            eval(scope, &format!("if(globalThis.effect)effect.cancel();globalThis.effect=subject.animate([{{strokeDasharray:'10 20'}},{{strokeDasharray:'30 40'}}],{{duration:1000,fill:'both',composite:'{composite}'}});effect.pause();effect.currentTime=500;__layoutCalls=0"));
            let document = object(scope, "document");
            let sample = sample_presented_effects(scope, document, &[]).unwrap();
            assert!(matches!(
                sample.targets[0].presentation,
                PresentedTargetState::Unknown { .. }
            ));
            assert!(eval(scope, "__layoutCalls===0").is_true());
            assert!(eval(
                scope,
                "getComputedStyle(subject).strokeDasharray==='20px, 30px'"
            )
            .is_true());
        }
        eval(scope,"effect.cancel();effect=subject.animate([{strokeDasharray:'10% 20px'},{strokeDasharray:'30px 40%'}],{duration:1000,fill:'both'});effect.pause();effect.currentTime=500");
        let document = object(scope, "document");
        let sample = sample_presented_effects(scope, document, &[]).unwrap();
        assert!(matches!(
            sample.targets[0].presentation,
            PresentedTargetState::Unknown { .. }
        ));
    });
}

#[test]
fn chrome_ignored_ms_transform_does_not_hide_supported_css_effects() {
    with_dom(|scope| {
        for (from, to, expected) in [
            (
                "-ms-transform:rotate(0deg);transform:rotate(0deg)",
                "-ms-transform:rotate(360deg);transform:rotate(360deg)",
                "matrix(-1, 0, 0, -1, 0, 0)",
            ),
            (
                "-ms-transform:scale(0);transform:scale(0);opacity:0.1",
                "-ms-transform:scale(1);transform:scale(1);opacity:0.3",
                "matrix(0.5, 0, 0, 0.5, 0, 0)",
            ),
        ] {
            eval(scope, &format!("{{document.head.innerHTML='';const s=document.createElement('style');s.textContent='@keyframes motion{{from{{{from}}}to{{{to}}}}}#target{{animation:motion 1000ms linear both}}';document.head.append(s);globalThis.effect=document.getElementById('target').getAnimations()[0];effect.pause();effect.currentTime=500;__layoutCalls=0}}"));
            assert!(eval(scope, "document.styleSheets[0].cssRules[0].cssRules[0].style.getPropertyValue('-ms-transform')==='' && !('-ms-transform' in effect.effect.getKeyframes()[0])").is_true());
            let pairs = presented_animation_pairs(scope);
            let transform = pairs.iter().find(|pair| pair.name == "transform").unwrap();
            assert_matrix_close(&transform.value, expected);
            assert!(eval(scope, "__layoutCalls===0").is_true());
        }
    });
}

#[test]
fn repeatable_svg_list_expansion_is_bounded_before_composition() {
    with_dom(|scope| {
        eval(scope,"document.body.innerHTML='<svg><circle id=subject /></svg>';globalThis.subject=document.getElementById('subject');globalThis.effect=subject.animate([{strokeDasharray:Array(17).fill('1').join(' ')},{strokeDasharray:Array(19).fill('2').join(' ')}],{duration:1000,fill:'both'});effect.pause();effect.currentTime=500;__layoutCalls=0");
        let document = object(scope, "document");
        let sample = sample_presented_effects(scope, document, &[]).unwrap();
        assert!(matches!(
            sample.targets[0].presentation,
            PresentedTargetState::Unknown { .. }
        ));
        assert!(eval(scope, "__layoutCalls===0").is_true());
        assert!(eval(scope, "(()=>{try{getComputedStyle(subject).strokeDasharray;return false}catch(error){return error instanceof RangeError && /source-derived work budget/.test(error.message)}})()").is_true());
    });
}

#[test]
fn captured_mui_effect_sources_are_certified_at_all_fifteen_oracle_samples() {
    use sha2::{Digest, Sha256};
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/captured_mui_animation_chrome159.json"
    ))
    .unwrap();
    assert_eq!(fixture["oracleVersion"], "159.0.8154.6");
    for case in fixture["cases"].as_array().unwrap() {
        let html = case["html"].as_str().unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(html)),
            case["htmlSHA256"].as_str().unwrap()
        );
        with_html(true, true, html, |scope| {
            eval(scope,"globalThis.subject=document.getElementById('subject');globalThis.fixtureEffects=document.getAnimations();for(const effect of fixtureEffects)effect.pause()");
            for sample in case["samples"].as_array().unwrap() {
                let time = sample["time"].as_u64().unwrap();
                eval(scope, &format!("for(const effect of fixtureEffects)effect.currentTime={time};__layoutCalls=0"));
                let pairs = presented_animation_pairs(scope);
                assert!(eval(scope, "__layoutCalls===0").is_true());
                if case["kind"] == "svg" {
                    let array = pairs
                        .iter()
                        .find(|pair| pair.name == "stroke-dasharray")
                        .unwrap();
                    let values: Vec<f64> = array
                        .value
                        .split(',')
                        .map(|value| value.trim().trim_end_matches("px").parse().unwrap())
                        .collect();
                    let expected: Vec<f64> = sample["observation"]["dasharray"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|value| value.as_f64().unwrap())
                        .collect();
                    assert_eq!(values, expected, "{} time{time}", case["id"]);
                    let offset = pairs
                        .iter()
                        .find(|pair| pair.name == "stroke-dashoffset")
                        .unwrap()
                        .value
                        .trim_end_matches("px")
                        .parse::<f64>()
                        .unwrap();
                    assert_eq!(
                        offset,
                        sample["observation"]["dashoffset"].as_f64().unwrap()
                    );
                } else {
                    assert!(
                        pairs.iter().any(|pair| pair.name == "transform"),
                        "{} time{time} {pairs:?}",
                        case["id"]
                    );
                    assert!(eval(scope,"Array.from(document.styleSheets[0].cssRules).find(rule=>rule.type===7).cssRules[0].style.getPropertyValue('-ms-transform')===''").is_true());
                    assert!(browser_env::run_engine_script(
                        scope,
                        "return __tsNs.cssSupportsProperty('-ms-transform','rotate(1deg)')===false"
                    )
                    .unwrap()
                    .is_true());
                }
            }
        });
    }
}

fn assert_matrix_close(actual: &str, expected: &str) {
    let values = |text: &str| -> Vec<f64> {
        text.trim_start_matches("matrix(")
            .trim_end_matches(')')
            .split(',')
            .map(|part| part.trim().parse().unwrap())
            .collect()
    };
    let actual = values(actual);
    let expected = values(expected);
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.iter().zip(expected) {
        assert!(
            (actual - expected).abs() < 0.00001,
            "matrix component {actual} != {expected}"
        );
    }
}

#[test]
fn captured_mui_effects_are_certified_without_pausing_or_seeking() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/captured_mui_animation_chrome159.json"
    ))
    .unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        with_html(true, true, case["html"].as_str().unwrap(), |scope| {
            let document = object(scope, "document");
            let subject = object(scope, "document.getElementById('subject')");
            let handle = handle_of(scope, subject).unwrap();
            let sample = sample_presented_effects(scope, document, &[handle]).unwrap();
            assert_eq!(sample.targets.len(), 1);
            assert!(
                matches!(&sample.targets[0].presentation, PresentedTargetState::Certified { animation,.. } if !animation.is_empty()),
                "{} {sample:?}",
                case["id"]
            );
            assert!(sample.targets[0]
                .effects
                .iter()
                .all(|effect| !effect.script_modified));
            assert!(eval(scope, "__layoutCalls===0").is_true());
        });
    }
}

#[test]
fn legacy_vendor_admission_preserves_webkit_and_custom_controls() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/vendor_animation_properties_chrome159.json"
    ))
    .unwrap();
    with_dom(|scope| {
        for row in fixture["output"].as_array().unwrap() {
            let name = serde_json::to_string(row["property"].as_str().unwrap()).unwrap();
            let supported = browser_env::run_engine_script(
                scope,
                &format!("return __tsNs.cssSupportsProperty({name},'rotate(1deg)')"),
            )
            .unwrap();
            assert_eq!(
                supported.is_true(),
                row["supports"].as_bool().unwrap(),
                "{}",
                row["property"]
            );
            let expected = serde_json::to_string(row["value"].as_str().unwrap()).unwrap();
            assert!(eval(scope,&format!("(()=>{{const el=document.createElement('div');el.style.setProperty({name},'rotate(1deg)');return el.style.getPropertyValue({name})==={expected}}})()")).is_true(),"{}",row["property"]);
        }
    });
}

fn auth_menu_candidates(scope: &mut v8::PinScope) -> Vec<Handle> {
    [
        ".dot:nth-child(1)",
        ".dot:nth-child(2)",
        ".dot:nth-child(3)",
        "#message",
        ".user-menu",
    ]
    .iter()
    .map(|selector| {
        let element = object(scope, &format!("document.querySelector('{selector}')"));
        handle_of(scope, element).unwrap()
    })
    .collect()
}

#[test]
fn exact_auth_fixture_certifies_display_suppression_across_phase_changes() {
    use sha2::{Digest, Sha256};
    const HTML: &str = include_str!("fixtures/auth_menu_animations.html");
    assert_eq!(
        format!("{:x}", Sha256::digest(HTML)),
        "e2134c7ad68f7eb63ca92e173985715c029c45be96e87a2440431fe6ae00d0d3"
    );
    with_html(true, true, HTML, |scope| {
        let document = object(scope, "document");
        let candidates = auth_menu_candidates(scope);
        for phase in ["none", "typing", "message", "none", "typing"] {
            eval(
                scope,
                &format!(
                    "document.documentElement.setAttribute('data-phase','{phase}');__layoutCalls=0"
                ),
            );
            let sample = sample_presented_effects(scope, document, &candidates).unwrap();
            assert!(
                sample.targets.iter().all(|target| matches!(
                    target.presentation,
                    PresentedTargetState::Certified { .. }
                )),
                "{phase}: {sample:?}"
            );
            if phase != "typing" {
                for handle in &candidates[..3] {
                    let target = sample
                        .targets
                        .iter()
                        .find(|target| target.element == *handle)
                        .unwrap();
                    assert!(
                        matches!(&target.presentation,PresentedTargetState::Certified{animation,transition} if animation.is_empty()&&transition.is_empty()),
                        "{phase}: {target:?}"
                    );
                    assert!(
                        target
                            .effects
                            .iter()
                            .all(|effect| !effect.contributes && !effect.script_modified),
                        "canonical hiding is not guest script modification: {target:?}"
                    );
                }
            }
            assert!(eval(scope, "__layoutCalls===0").is_true());
        }
    });
}

#[test]
fn canonical_css_cancellation_preserves_genuine_script_modification_evidence() {
    with_html(true,true,"<style>@keyframes fade{from{opacity:.3}to{opacity:1}}#target{animation:fade 1s infinite}</style><div id=parent><div id=target></div></div>",|scope|{
        let document = object(scope,"document");
        let target = object(scope,"document.getElementById('target')");
        let handle = handle_of(scope,target).unwrap();
        let first = sample_presented_effects(scope,document,&[handle]).unwrap();
        let identity = first.targets[0].effects[0].identity;
        assert!(!first.targets[0].effects[0].script_modified);
        eval(scope,"globalThis.effect=document.getElementById('target').getAnimations()[0];effect.pause();document.getElementById('parent').style.display='none';__layoutCalls=0");
        let hidden = sample_presented_effects(scope,document,&[handle]).unwrap();
        assert!(matches!(&hidden.targets[0].presentation,PresentedTargetState::Certified{animation,transition} if animation.is_empty()&&transition.is_empty()));
        let effect = hidden.targets[0].effects.iter().find(|effect|effect.identity==identity).unwrap();
        assert!(effect.script_modified && !effect.contributes,"genuine pause must remain recorded: {effect:?}");
        assert!(eval(scope,"__layoutCalls===0").is_true());
    });
}

#[test]
fn suppressed_css_animations_do_not_mask_unsupported_presented_rendering_context() {
    for frames in [
        "[{display:'block'},{display:'block'}]",
        "[{'--box':'block'},{'--box':'block'}]",
    ] {
        with_html(true,true,"<style>@keyframes fade{from{opacity:.3}to{opacity:1}}#parent{--box:none;display:var(--box)}#target{animation:fade 1s infinite}</style><div id=parent><div id=target></div></div>",|scope|{
            eval(scope,&format!("globalThis.effect=document.getElementById('parent').animate({frames},{{duration:1000,fill:'both'}});effect.pause();effect.currentTime=500;__layoutCalls=0"));
            let document = object(scope,"document");
            let target = object(scope,"document.getElementById('target')");
            let handle = handle_of(scope,target).unwrap();
            let sample = sample_presented_effects(scope,document,&[handle]).unwrap();
            let target = sample.targets.iter().find(|target|target.element==handle).unwrap();
            assert!(matches!(&target.presentation,PresentedTargetState::Unknown{..}) || matches!(&target.presentation,PresentedTargetState::Certified{animation,..}if animation.iter().any(|property|property.name=="opacity"&&property.value=="0.3")),"presented visibility must not become an empty hidden certificate: {sample:?}");
            assert!(eval(scope,"__layoutCalls===0").is_true());
        });
    }
}

#[test]
fn hidden_unmodeled_css_stays_inactive_but_visible_unsupported_sources_stay_unknown() {
    with_html(true,true,"<style>@keyframes move{from{float:left}to{float:right}}#parent{display:none}#target{animation:move 1s infinite}</style><div id=parent><div id=target></div></div>",|scope|{
        let document = object(scope,"document");
        let target = object(scope,"document.getElementById('target')");
        let handle = handle_of(scope,target).unwrap();
        let hidden = sample_presented_effects(scope,document,&[handle]).unwrap();
        assert!(matches!(&hidden.targets[0].presentation,PresentedTargetState::Certified{animation,transition} if animation.is_empty()&&transition.is_empty()),"{hidden:?}");
        eval(scope,"document.getElementById('parent').style.display='block'");
        let visible = sample_presented_effects(scope,document,&[handle]).unwrap();
        assert!(matches!(visible.targets[0].presentation,PresentedTargetState::Unknown{..}),"{visible:?}");
        eval(scope,"document.getElementById('target').style.animationName='missing-definition'");
        let missing = sample_presented_effects(scope,document,&[handle]).unwrap();
        assert!(matches!(&missing.targets[0].presentation,PresentedTargetState::Certified{animation,transition}if animation.is_empty()&&transition.is_empty()),"complete missing-definition proof: {missing:?}");
        assert_eq!(missing.targets[0].css_animation_names,["missing-definition"]);
    });
}

const POISON_SCROLL_INTRINSICS: &str = r#"
    globalThis.__poisonCalls=0;
    Array.prototype.includes=Array.prototype.push=Array.prototype.splice=Array.prototype.indexOf=()=>{__poisonCalls++;throw Error('public array')};
    Array.prototype[Symbol.iterator]=()=>{__poisonCalls++;throw Error('public iterator')};
    globalThis.Event=function(){__poisonCalls++;throw Error('public Event')};
    globalThis.setTimeout=()=>{__poisonCalls++;throw Error('public timer')};
    Promise.prototype.then=Promise.prototype.then;
"#;

#[test]
fn empty_presentation_uses_private_records_after_scroll_intrinsic_poisoning() {
    with_html(true,true,"<!doctype html><body style='margin:0'><div id=box style='width:50px;height:50px;overflow:auto'><div style='height:500px'></div></div><div style='height:3000px'></div></body>",|scope|{
        let document=object(scope,"document");
        eval(scope,POISON_SCROLL_INTRINSICS);
        let sample=sample_presented_effects(scope,document,&[]).expect("a proven empty canonical target set needs no guest prototype inspection");
        assert!(sample.targets.is_empty());
        assert!(eval(scope,"__poisonCalls===0 && __layoutCalls===0 && __clockCalls===1").is_true());
    });
}

#[test]
fn active_presentation_keeps_strict_prototype_guards_after_scroll_intrinsic_poisoning() {
    with_dom(|scope| {
        eval(scope,"globalThis.effect=document.getElementById('target').animate([{opacity:0},{opacity:1}],{duration:1000,fill:'both'});effect.pause();effect.currentTime=500;__clockCalls=0;__layoutCalls=0");
        let document = object(scope, "document");
        eval(scope, POISON_SCROLL_INTRINSICS);
        v8::tc_scope!(let scope,scope);
        assert!(sample_presented_effects(scope, document, &[]).is_none());
        assert!(
            !scope.has_caught(),
            "no public poisoned method should be invoked"
        );
        assert!(eval(scope, "__poisonCalls===0 && __layoutCalls===0").is_true());
    });
}

#[test]
fn exact_auth_menu_transitions_and_new_ripple_remain_canonically_sampleable() {
    with_html(
        true,
        true,
        include_str!("fixtures/auth_menu_animations.html"),
        |scope| {
            let document = object(scope, "document");
            let mut candidates = auth_menu_candidates(scope);
            eval(
                scope,
                "document.documentElement.setAttribute('data-phase','none')",
            );
            let baseline = sample_presented_effects(scope, document, &candidates).unwrap();
            assert!(
                baseline.targets.iter().all(|target| matches!(
                    target.presentation,
                    PresentedTargetState::Certified { .. }
                )),
                "{baseline:?}"
            );
            eval(scope,"const menu=document.querySelector('.user-menu');menu.style.opacity='1';menu.style.visibility='visible';const ripple=document.createElement('span');ripple.className='ripple';document.getElementById('ripple-root').append(ripple);globalThis.ripple=ripple;__layoutCalls=0");
            let ripple = object(scope, "ripple");
            let handle = handle_of(scope, ripple).unwrap();
            candidates.push(handle);
            let sample = sample_presented_effects(scope, document, &candidates).unwrap();
            assert!(
                sample.targets.iter().all(|target| matches!(
                    target.presentation,
                    PresentedTargetState::Certified { .. }
                )),
                "{sample:?}"
            );
            let target = sample
                .targets
                .iter()
                .find(|target| target.element == handle)
                .unwrap();
            assert!(
                matches!(&target.presentation,PresentedTargetState::Certified{animation,..}if animation.iter().any(|property|property.name=="transform")),
                "{target:?}"
            );
            assert!(eval(scope, "__layoutCalls===0").is_true());
        },
    );
}

#[test]
fn empty_target_discovery_never_reads_public_weak_maps_or_descriptor_getters() {
    with_dom(|scope| {
        eval(scope,"globalThis.effect=document.getElementById('target').animate([{opacity:0},{opacity:1}],{duration:1000});effect.effect.target=null;__clockCalls=0;__layoutCalls=0");
        let document = object(scope, "document");
        eval(scope, POISON_SCROLL_INTRINSICS);
        eval(scope,"WeakMap.prototype.get=()=>{__poisonCalls++;throw Error('public WeakMap')};Object.defineProperty(Object.prototype,'get',{configurable:true,get(){__poisonCalls++;throw Error('public descriptor')}})");
        v8::tc_scope!(let scope,scope);
        let sample = sample_presented_effects(scope, document, &[])
            .expect("untargeted private effects do not require guest prototype reads");
        assert!(sample.targets.is_empty());
        assert!(!scope.has_caught());
        assert!(eval(
            scope,
            "__poisonCalls===0 && __layoutCalls===0 && __clockCalls===1"
        )
        .is_true());
    });
}

#[test]
fn active_target_discovery_refuses_poison_without_invoking_public_descriptor_getters() {
    with_dom(|scope| {
        eval(scope,"globalThis.effect=document.getElementById('target').animate([{opacity:0},{opacity:1}],{duration:1000,fill:'both'});effect.pause();effect.currentTime=500;__clockCalls=0;__layoutCalls=0");
        let document = object(scope, "document");
        eval(scope, POISON_SCROLL_INTRINSICS);
        eval(scope,"WeakMap.prototype.get=()=>{__poisonCalls++;throw Error('public WeakMap')};Object.defineProperty(Object.prototype,'get',{configurable:true,get(){__poisonCalls++;throw Error('public descriptor')}})");
        v8::tc_scope!(let scope,scope);
        assert!(sample_presented_effects(scope, document, &[]).is_none());
        assert!(!scope.has_caught());
        assert!(eval(scope, "__poisonCalls===0 && __layoutCalls===0").is_true());
    });
}

#[test]
fn empty_presentation_bypasses_exact_runtime_descriptor_poisoning_without_conversion() {
    with_dom(|scope| {
        let document = object(scope, "document");
        let forged = object(scope, "Object.create(Document.prototype)");
        eval(scope,"globalThis.hooks=0;const define=Object.defineProperty,prototype=Object.prototype;define(prototype,'get',{get(){hooks++;throw Error('descriptor getter')},configurable:true});define(prototype,'set',{get(){hooks++;throw Error('descriptor setter')},configurable:true});__clockCalls=0;__layoutCalls=0");
        v8::tc_scope!(let scope,scope);
        assert!(sample_presented_effects(scope, forged, &[]).is_none());
        let sample = sample_presented_effects(scope, document, &[]).unwrap();
        assert!(sample.targets.is_empty() && !scope.has_caught());
        assert!(eval(scope, "hooks===0 && __clockCalls===1 && __layoutCalls===0").is_true());
    });
}

#[test]
fn suppressed_ordinary_animation_is_independent_of_an_unrelated_closed_shadow() {
    with_html(true,true,"<style>@keyframes fade{from{opacity:.3}to{opacity:1}}#parent{display:none}#target{animation:fade 1s infinite}</style><div id=parent><div id=target></div></div><div id=host></div>",|scope|{
        eval(scope,"document.getElementById('host').attachShadow({mode:'closed'}).innerHTML='<slot></slot>'");
        let document=object(scope,"document");
        let target=object(scope,"document.getElementById('target')");
        let handle=handle_of(scope,target).unwrap();
        let sample=sample_presented_effects(scope,document,&[handle]).unwrap();
        assert!(matches!(&sample.targets[0].presentation,PresentedTargetState::Certified{animation,transition}if animation.is_empty()&&transition.is_empty()),"{sample:?}");
    });
}

#[test]
fn suppressed_shadow_and_closed_slot_paths_remain_unproven() {
    for child in [false, true] {
        with_html(true,true,"<style>@keyframes fade{from{opacity:.3}to{opacity:1}}#hidden{display:none}.animated{animation:fade 1s infinite}</style><div id=host><div id=hidden><div id=target class=animated></div></div></div>",|scope|{
            if child {
                eval(scope,"globalThis.root=document.getElementById('host').attachShadow({mode:'closed'});root.innerHTML='<style>@keyframes fade{from{opacity:.3}to{opacity:1}}.animated{animation:fade 1s infinite}</style><div style=display:none><div id=target class=animated></div></div>';globalThis.target=root.getElementById('target')");
            } else {
                eval(scope,"globalThis.root=document.getElementById('host').attachShadow({mode:'closed'});root.innerHTML='<slot></slot>';globalThis.target=document.getElementById('target');if(target.parentElement.assignedSlot!==null)throw Error('closed slot must not be exposed')");
            }
            let document=object(scope,"document");
            let target=object(scope,"target");
            let handle=handle_of(scope,target).unwrap();
            let sample=sample_presented_effects(scope,document,&[handle]).unwrap();
            assert!(matches!(sample.targets[0].presentation,PresentedTargetState::Unknown{..}),"unproven composed path: {sample:?}");
        });
    }
}

#[test]
fn hidden_animation_does_not_certify_an_unsupported_container_cascade() {
    with_html(true,true,"<style>@keyframes fade{from{opacity:.3}to{opacity:1}}#container{container-type:inline-size;width:100px}#parent{display:none}@container(min-width:1px){#parent{display:block}}#target{animation:fade 1s infinite}</style><div id=container><div id=parent><div id=target></div></div></div>",|scope|{
        let document=object(scope,"document");
        let target=object(scope,"document.getElementById('target')");
        let handle=handle_of(scope,target).unwrap();
        let sample=sample_presented_effects(scope,document,&[handle]).unwrap();
        assert!(matches!(sample.targets[0].presentation,PresentedTargetState::Unknown{..}),"unsupported groups cannot supply rendering suppression: {sample:?}");
        assert!(eval(scope,"__layoutCalls===0").is_true());
    });
}

#[test]
fn presentation_policy_suppresses_rendering_context_effects_independently() {
    with_html(true,true,"<style>@keyframes fade{from{opacity:.3}to{opacity:1}}#parent{--box:none;display:var(--box)}#target{animation:fade 1s infinite}</style><div id=parent><div id=target></div></div>",|scope|{
        assert!(set_animation_presentation(scope,AnimationPresentation::Disabled));
        eval(scope,"globalThis.effect=document.getElementById('parent').animate([{'--box':'block'},{'--box':'block'}],{duration:1000,fill:'both'});effect.pause();effect.currentTime=500;__layoutCalls=0");
        let document=object(scope,"document");
        let target=object(scope,"document.getElementById('target')");
        let handle=handle_of(scope,target).unwrap();
        let sample=sample_presented_effects(scope,document,&[handle]).unwrap();
        assert!(sample.presentation_suppressed && sample.css_animations_enabled && sample.css_transitions_enabled);
        assert!(sample.targets.iter().all(|target|matches!(&target.presentation,PresentedTargetState::Certified{animation,transition}if animation.is_empty()&&transition.is_empty())),"{sample:?}");
        assert!(eval(scope,"__layoutCalls===0").is_true());
    });
}

#[test]
fn canonical_quoted_escaped_and_case_sensitive_names_match_chrome() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/keyframe_names_chrome159.json")).unwrap();
    for (case, oracle) in fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .zip(fixture["results"].as_array().unwrap())
    {
        let html=format!("<style>@keyframes {}{{to{{width:100px}}}}#target{{width:20px;height:10px;{}}}</style><div id=target></div>",case["rule"].as_str().unwrap(),case["declaration"].as_str().unwrap());
        with_html(true, true, &html, |scope| {
            let expected_names: Vec<&str> = oracle["effects"]
                .as_array()
                .unwrap()
                .iter()
                .map(|effect| effect["name"].as_str().unwrap())
                .collect();
            let names=eval(scope,"JSON.stringify(document.getElementById('target').getAnimations().map(effect=>effect.animationName))").to_rust_string_lossy(scope);
            assert_eq!(
                serde_json::from_str::<Vec<String>>(&names).unwrap(),
                expected_names,
                "{}",
                case["id"]
            );
            eval(scope,"globalThis.target=document.getElementById('target');globalThis.effects=target.getAnimations();for(const effect of effects){effect.pause();effect.currentTime=500};__layoutCalls=0");
            let document = object(scope, "document");
            let target = object(scope, "target");
            let handle = handle_of(scope, target).unwrap();
            let sample = sample_presented_effects(scope, document, &[handle]).unwrap();
            assert!(
                matches!(
                    sample.targets[0].presentation,
                    PresentedTargetState::Certified { .. }
                ),
                "{} {sample:?}",
                case["id"]
            );
            let actual_names: Vec<&str> = sample.targets[0]
                .effects
                .iter()
                .filter(|effect| effect.contributes)
                .map(|effect| effect.name.as_str())
                .collect();
            assert_eq!(actual_names, expected_names, "{} {sample:?}", case["id"]);
            assert!(eval(scope, "__layoutCalls===0").is_true());
        });
    }
}

#[test]
fn unchanged_unused_resource_inputs_never_create_a_canonical_effect() {
    use sha2::{Digest, Sha256};
    let source = include_str!("fixtures/resource_definitions_probe.js");
    assert_eq!(
        format!("{:x}", Sha256::digest(source)),
        "ef222738cd556f339e0321ba1e31828338ac6d883e9a6c2ef4bb0f6b5f1eafe0"
    );
    let mut cases = Vec::<String>::new();
    with_dom(|scope| {
        let prefix = source.split_once("  const result = cases.map").unwrap().0;
        let json = eval(
            scope,
            &format!("JSON.stringify({prefix}return cases;}})())"),
        )
        .to_rust_string_lossy(scope);
        cases = serde_json::from_str(&json).unwrap();
    });
    assert_eq!(cases.len(), 10);
    for css in cases {
        with_html(true,true,&format!("<style>#target{{width:43px;height:17px}}{css}</style><div id=target></div><div id=hidden><span>hidden</span></div>"),|scope|{
            let document=object(scope,"document");
            let target=object(scope,"document.getElementById('target')");
            let handle=handle_of(scope,target).unwrap();
            let sample=sample_presented_effects(scope,document,&[handle]).unwrap();
            assert!(matches!(&sample.targets[0].presentation,PresentedTargetState::Certified{animation,transition}if animation.is_empty()&&transition.is_empty()),"{css}: {sample:?}");
            assert!(sample.targets[0].effects.iter().all(|effect|!effect.contributes));
            assert!(eval(scope,"__layoutCalls===0").is_true());
        });
    }
}

#[test]
fn missing_names_do_not_certify_uncertain_import_or_container_definitions() {
    for group in [
        "@import url('/possible-keyframes.css');",
        "@container(min-width:1px){@keyframes missing{to{opacity:1}}}",
    ] {
        with_html(
            true,
            true,
            &format!("<style>{group}#target{{animation:missing 1s}}</style><div id=target></div>"),
            |scope| {
                let document = object(scope, "document");
                let target = object(scope, "document.getElementById('target')");
                let handle = handle_of(scope, target).unwrap();
                let sample = sample_presented_effects(scope, document, &[handle]).unwrap();
                assert!(
                    matches!(
                        sample.targets[0].presentation,
                        PresentedTargetState::Unknown { .. }
                    ),
                    "{sample:?}"
                );
            },
        );
    }
}

#[test]
fn unresolved_animation_name_functions_keep_their_cannot_prove_exception() {
    with_html(true,true,"<style>@keyframes Spin{to{width:100px}}#target{animation-name:env(unknown)}</style><div id=target></div>",|scope|{
        let document=object(scope,"document");
        let target=object(scope,"document.getElementById('target')");
        let handle=handle_of(scope,target).unwrap();
        {
            v8::tc_scope!(let scope,scope);
            assert!(sample_presented_effects(scope,document,&[handle]).is_none());
            assert!(scope.has_caught(),"unresolved syntax is not an inactive-effect certificate");
            let exception=scope.exception().unwrap().to_rust_string_lossy(scope);
            assert!(exception.contains("cannot prove"),"{exception}");
        }
        eval(scope,"document.getElementById('target').style.animationName='none'");
        let healthy=sample_presented_effects(scope,document,&[handle]).unwrap();
        assert!(matches!(&healthy.targets[0].presentation,PresentedTargetState::Certified{animation,transition}if animation.is_empty()&&transition.is_empty()));
    });
}

#[test]
fn current_css_names_do_not_duplicate_canceled_spinner_history() {
    with_html(
        true,
        true,
        include_str!("fixtures/clipped_spinner.html"),
        |scope| {
            let document = object(scope, "document");
            let spin = object(scope, "document.getElementById('spin')");
            let circle = object(scope, "document.querySelector('circle')");
            let handles = [
                handle_of(scope, spin).unwrap(),
                handle_of(scope, circle).unwrap(),
            ];
            let first = sample_presented_effects(scope, document, &handles).unwrap();
            assert_eq!(first.targets[0].css_animation_names, ["spin"]);
            assert_eq!(first.targets[1].css_animation_names, ["dash"]);
            eval(scope,"document.getElementById('spin').style.animation='none';document.querySelector('circle').style.animation='none'");
            let canceled = sample_presented_effects(scope, document, &handles).unwrap();
            assert!(canceled
                .targets
                .iter()
                .all(|target| target.css_animation_names.is_empty()));
            eval(scope,"document.getElementById('spin').style.removeProperty('animation');document.querySelector('circle').style.removeProperty('animation')");
            let restarted = sample_presented_effects(scope, document, &handles).unwrap();
            for (target, name) in restarted.targets.iter().zip(["spin", "dash"]) {
                assert_eq!(target.css_animation_names, [name]);
                assert!(
                    target
                        .effects
                        .iter()
                        .filter(|effect| effect.name == name)
                        .count()
                        >= 2,
                    "lifecycle provenance must remain intact: {target:?}"
                );
            }
            eval(scope,"globalThis.currentName=getComputedStyle(document.getElementById('spin')).animationName;__layoutCalls=0");
            let after_read = sample_presented_effects(scope, document, &handles).unwrap();
            assert_eq!(after_read.targets[0].css_animation_names, ["spin"]);
            assert_eq!(after_read.targets[1].css_animation_names, ["dash"]);
            assert!(
                after_read.targets.iter().all(|target| matches!(
                    target.presentation,
                    PresentedTargetState::Certified { .. }
                )),
                "{after_read:?}"
            );
            assert!(eval(scope, "currentName==='spin' && __layoutCalls===0").is_true());
        },
    );
}

#[test]
fn current_css_names_preserve_duplicates_and_ignore_only_the_none_keyword() {
    with_html(true,true,"<style>@keyframes repeated{from{opacity:.3}to{opacity:1}}@keyframes 'none'{from{width:20px}to{width:40px}}#target{width:20px;animation-name:none,repeated,repeated,'none';animation-duration:1s,2s,3s,4s}</style><div id=target></div>",|scope|{
        let document=object(scope,"document");
        let target=object(scope,"document.getElementById('target')");
        let handle=handle_of(scope,target).unwrap();
        let sample=sample_presented_effects(scope,document,&[handle]).unwrap();
        assert_eq!(sample.targets[0].css_animation_names,["repeated","repeated","none"]);
        assert_eq!(sample.targets[0].effects.len(),3);
    });
}

#[test]
fn current_css_name_wire_field_is_required_instead_of_defaulting_empty() {
    with_dom(|scope| {
        browser_env::run_engine_script(scope,"const original=__tsNs.samplePresentedEffects;__tsNs.samplePresentedEffects=function(document,candidates){const snapshot=original(document,candidates);for(let i=0;i<snapshot.targets.length;i++){const row=snapshot.targets[i],data=JSON.parse(row[1]);delete data.css_animation_names;row[1]=JSON.stringify(data)}return snapshot}").unwrap();
        let document = object(scope, "document");
        let target = object(scope, "document.getElementById('target')");
        let handle = handle_of(scope, target).unwrap();
        v8::tc_scope!(let scope,scope);
        assert!(sample_presented_effects(scope, document, &[handle]).is_none());
        assert!(!scope.has_caught());
    });
}

#[test]
fn a_missing_canonical_keyframe_resolver_never_proves_absence() {
    with_html(true,true,"<style>@keyframes present{to{opacity:1}}#target{animation:present 1s}</style><div id=target></div>",|scope|{
        browser_env::run_engine_script(scope,"__tsNs.cssKeyframeName=undefined").unwrap();
        let document=object(scope,"document");
        let target=object(scope,"document.getElementById('target')");
        let handle=handle_of(scope,target).unwrap();
        let sample=sample_presented_effects(scope,document,&[handle]).unwrap();
        assert_eq!(sample.targets[0].css_animation_names,["present"]);
        assert!(matches!(sample.targets[0].presentation,PresentedTargetState::Unknown{..}),"{sample:?}");
    });
}

#[test]
fn current_css_names_follow_all_seven_retained_restart_reorder_and_cancel_states() {
    with_html(true,true,"<!doctype html><style>@keyframes first{from{width:100px}to{width:200px}}@keyframes second{from{height:20px}to{height:40px}}#target{width:100px;height:20px;animation:first 1000ms linear}</style><div id=target></div>",|scope|{
        let document=object(scope,"document");
        let target=object(scope,"document.getElementById('target')");
        let handle=handle_of(scope,target).unwrap();
        eval(scope,"globalThis.target=document.getElementById('target');globalThis.retained=[]");
        let steps=[
            (None,vec!["first"],Some("150px"),None),
            (Some("none"),vec![],None,None),
            (Some("first 1000ms linear"),vec!["first"],Some("150px"),None),
            (Some("first 1000ms linear,second 1000ms linear"),vec!["first","second"],Some("150px"),Some("30px")),
            (Some("second 1000ms linear,first 1000ms linear"),vec!["second","first"],Some("150px"),Some("30px")),
            (Some("first 1000ms linear,first 2000ms linear"),vec!["first","first"],Some("125px"),None),
        ];
        for (style,names,width,height) in steps {
            if let Some(style)=style {eval(scope,&format!("target.style.animation='{style}'"));}
            if !names.is_empty() {eval(scope,"for(const animation of target.getAnimations()){retained.push(animation);animation.pause();animation.currentTime=500}");}
            eval(scope,"__layoutCalls=0");
            let sample=sample_presented_effects(scope,document,&[handle]).unwrap();
            let target=&sample.targets[0];
            assert_eq!(target.css_animation_names,names,"{sample:?}");
            let PresentedTargetState::Certified{animation,..}=&target.presentation else {panic!("{sample:?}")};
            for (property,value) in [("width",width),("height",height)] {
                assert_eq!(animation.iter().find(|pair|pair.name==property).map(|pair|pair.value.as_str()),value,"{sample:?}");
            }
            assert!(eval(scope,"__layoutCalls===0").is_true());
        }
        eval(scope,"for(const animation of target.getAnimations())animation.cancel();__layoutCalls=0");
        let canceled=sample_presented_effects(scope,document,&[handle]).unwrap();
        assert_eq!(canceled.targets[0].css_animation_names,["first","first"]);
        assert!(matches!(&canceled.targets[0].presentation,PresentedTargetState::Certified{animation,transition}if animation.is_empty()&&transition.is_empty()));
        assert!(canceled.targets[0].effects.iter().all(|effect|!effect.contributes));
        assert!(eval(scope,"__layoutCalls===0").is_true());
    });
}

#[test]
fn non_scalar_animation_name_payloads_never_alias_replacement_characters() {
    with_dom(|scope| {
        browser_env::run_engine_script(scope,"const original=__tsNs.computedStyleValue;__tsNs.computedStyleValue=(style,name)=>name==='animation-name'?String.fromCharCode(0xd800):original(style,name)").unwrap();
        let document = object(scope, "document");
        let target = object(scope, "document.getElementById('target')");
        let handle = handle_of(scope, target).unwrap();
        v8::tc_scope!(let scope,scope);
        assert!(sample_presented_effects(scope, document, &[handle]).is_none());
        assert!(scope.has_caught());
        assert!(scope
            .exception()
            .unwrap()
            .to_rust_string_lossy(scope)
            .contains("cannot prove"));
    });
}

thread_local! {
    static TERMINATE_INSPECTION_ONCE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn terminate_inspection_once(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    _result: v8::ReturnValue,
) {
    if TERMINATE_INSPECTION_ONCE.with(|armed| armed.replace(false)) {
        scope.terminate_execution();
    }
}

fn inspection_recovers_after_termination(hook: &str, animation: bool) {
    inspection_recovers_after_termination_mode(hook, animation, false);
}

fn inspection_recovers_after_termination_mode(hook: &str, animation: bool, public: bool) {
    with_html(
        true,
        true,
        "<!doctype html><style>@media (min-width:1px){#target{width:100px}}@keyframes pulse{from{opacity:0.2}to{opacity:0.8}}</style><div id=target style='height:20px'></div>",
        |scope| {
            browser_env::set_css_animations(scope, animation);
            if animation {
                eval(scope, "document.getElementById('target').style.animation='pulse 1s linear both'");
            }
            let document = object(scope, "document");
            let target = object(scope, "document.getElementById('target')");
            let handle = handle_of(scope, target).unwrap();
            let terminate = v8::Function::new(scope, terminate_inspection_once).unwrap();
            let key = v8::String::new(scope, "__terminateInspectionOnce").unwrap();
            scope.get_current_context().global(scope).set(scope, key.into(), terminate.into());
            browser_env::run_engine_script(scope, &format!(
                "__tsNs.__inspectionOriginal=__tsNs.{hook};__tsNs.{hook}=function(){{__terminateInspectionOnce();return __tsNs.__inspectionOriginal.apply(this,arguments)}}"
            )).unwrap();
            TERMINATE_INSPECTION_ONCE.with(|armed| armed.set(true));
            {
                v8::tc_scope!(let scope, scope);
                if public {
                    let source = v8::String::new(scope, "document.getAnimations()").unwrap();
                    assert!(v8::Script::compile(scope, source, None).unwrap().run(scope).is_none());
                } else {
                    assert!(sample_presented_effects(scope, document, &[handle]).is_none());
                }
                assert!(scope.has_terminated(), "inspection must reach native {hook} termination");
                assert!(!TERMINATE_INSPECTION_ONCE.with(std::cell::Cell::get));
                // Only the host which owns this terminated operation cancels it, after
                // the public sampler has returned. The document and realm stay alive.
                scope.cancel_terminate_execution();
                scope.reset();
            }
            recover_after_termination(scope).unwrap();
            browser_env::run_engine_script(scope, &format!(
                "__tsNs.{hook}=__tsNs.__inspectionOriginal;delete __tsNs.__inspectionOriginal"
            )).unwrap();
            let recovered = sample_presented_effects(scope, document, &[handle]);
            assert!(recovered.is_some(), "same-document inspection must recover after {hook} termination");
            eval(scope, "getComputedStyle(document.getElementById('target')).width;document.getAnimations()");
        },
    );
}

#[test]
fn presented_inspection_recovers_after_native_termination_in_refresh() {
    inspection_recovers_after_termination("refreshCssEffectInputs", false);
}

#[test]
fn computed_effect_inspection_recovers_after_native_termination_in_media() {
    inspection_recovers_after_termination("mediaTextMatches", false);
}

#[test]
fn css_update_busy_guard_recovers_after_native_termination_in_update() {
    inspection_recovers_after_termination("noteCssUpdatePass", true);
}

#[test]
fn presented_inspection_still_refuses_genuine_nested_reentry() {
    with_dom(|scope| {
        browser_env::run_engine_script(scope, r#"
            __tsNs.__inspectionOriginal=__tsNs.refreshCssEffectInputs;
            __tsNs.__nestedInspectionRefused=false;
            __tsNs.refreshCssEffectInputs=function(element){
                __tsNs.__nestedInspectionRefused=__tsNs.samplePresentedEffects(document,[element])===null;
                return __tsNs.__inspectionOriginal(element);
            };
        "#).unwrap();
        let document = object(scope, "document");
        let target = object(scope, "document.getElementById('target')");
        let handle = handle_of(scope, target).unwrap();
        assert!(sample_presented_effects(scope, document, &[handle]).is_some());
        browser_env::run_engine_script(
            scope,
            r#"
            __tsNs.refreshCssEffectInputs=__tsNs.__inspectionOriginal;
            delete __tsNs.__inspectionOriginal;
            if(!__tsNs.__nestedInspectionRefused)throw Error('nested inspection was not refused');
            delete __tsNs.__nestedInspectionRefused;
        "#,
        )
        .unwrap();
        assert!(sample_presented_effects(scope, document, &[handle]).is_some());
    });
}

#[test]
fn public_get_animations_interruption_recovers_in_same_document() {
    inspection_recovers_after_termination_mode("noteCssUpdatePass", true, true);
}

#[test]
fn termination_recovery_preserves_live_effects_and_inert_document_aliases() {
    with_dom(|scope| {
        eval(scope, "globalThis.kept=document.getElementById('target').animate([{width:'100px'},{width:'200px'}],{duration:1000,fill:'both'});kept.pause();kept.currentTime=500;globalThis.keptReady=kept.ready;globalThis.keptFinished=kept.finished;globalThis.inert=new DOMParser().parseFromString('<p>inert</p>','text/html')");
        recover_after_termination(scope).unwrap();
        assert!(eval(scope, "kept.currentTime===500&&kept.playState==='paused'&&kept.ready===keptReady&&kept.finished===keptFinished&&document.getElementById('target').getAnimations().includes(kept)&&inert.querySelector('p').textContent==='inert'").is_true());
        let document = object(scope, "document");
        let sample = sample_presented_effects(scope, document, &[]).unwrap();
        assert_eq!(animated_value(&sample.targets[0], "width"), "150px");
    });
}

fn attempted_live_recovery(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut result: v8::ReturnValue,
) {
    result.set_bool(
        recover_after_termination(scope) == Err(TerminationRecoveryError::ActiveJavaScript),
    );
}

#[test]
fn termination_recovery_refuses_live_reentry_and_uncanceled_termination() {
    with_dom(|scope| {
        let callback = v8::Function::new(scope, attempted_live_recovery).unwrap();
        let key = v8::String::new(scope, "__attemptRecovery").unwrap();
        scope
            .get_current_context()
            .global(scope)
            .set(scope, key.into(), callback.into());
        assert!(eval(scope, "__attemptRecovery()").is_true());
        scope.terminate_execution();
        {
            v8::tc_scope!(let catch, scope);
            let code = v8::String::new(catch, "0").unwrap();
            let script = v8::Script::compile(catch, code, None);
            assert!(script.and_then(|script| script.run(catch)).is_none());
            assert!(catch.has_terminated());
            assert_eq!(
                recover_after_termination(catch),
                Err(TerminationRecoveryError::StillTerminating)
            );
            catch.cancel_terminate_execution();
            catch.reset();
        }
        recover_after_termination(scope).unwrap();
    });
}

#[test]
fn termination_recovery_keeps_simultaneous_isolate_worlds_separate() {
    with_dom(|outer| {
        eval(outer, "globalThis.kept=document.getElementById('target').animate([{width:'100px'},{width:'200px'}],{duration:1000,fill:'both'});kept.pause();kept.currentTime=500");
        with_dom_lifecycle(true, false, |inner| {
            recover_after_termination(inner).unwrap();
            eval(
                inner,
                "new DOMParser().parseFromString('<p>other inert</p>','text/html')",
            );
        });
        recover_after_termination(outer).unwrap();
        assert!(eval(outer, "kept.currentTime===500&&kept.playState==='paused'").is_true());
    });
}

#[test]
fn termination_recovery_visits_retained_non_browsing_realms() {
    with_dom(|scope| {
        let retained = v8::Context::new(scope, Default::default());
        let before;
        {
            let inner = &mut v8::ContextScope::new(scope, retained);
            browser_env::install_independent_html(inner, "<!doctype html><p>retained</p>");
            let _owner = enter_context_document(retained).unwrap();
            before = style_revisions();
            retire_context(retained);
            assert!(!is_browsing_context());
        }
        recover_after_termination(scope).unwrap();
        let _owner = enter_context_document(retained).unwrap();
        assert_ne!(
            style_revisions(),
            before,
            "retained realm cleanup must invalidate incomplete cascade caches"
        );
        assert!(!is_browsing_context());
    });
}

#[test]
fn termination_recovery_refuses_an_uninstalled_isolate_before_weak_localization() {
    with_dom(|_outer| {
        let isolate = &mut v8::Isolate::new(Default::default());
        isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
        v8::scope!(let inner, isolate);
        let context = v8::Context::new(inner, Default::default());
        let inner = &mut v8::ContextScope::new(inner, context);
        assert_eq!(
            recover_after_termination(inner),
            Err(TerminationRecoveryError::IsolateUnavailable)
        );
    });
}

#[test]
fn termination_recovery_never_runs_queued_guest_microtasks() {
    with_dom(|scope| {
        eval(
            scope,
            "globalThis.recoveryMicrotasks=0;Promise.resolve().then(()=>{recoveryMicrotasks++})",
        );
        let before = style_revisions();
        recover_after_termination(scope).unwrap();
        assert_ne!(style_revisions(), before);
        assert!(eval(scope, "recoveryMicrotasks===0").is_true());
        scope.set_microtasks_policy(v8::MicrotasksPolicy::Auto);
        let before = style_revisions();
        assert_eq!(
            recover_after_termination(scope),
            Err(TerminationRecoveryError::ImplicitMicrotasks)
        );
        // Restore the host's original policy before evaluating assertions: the
        // rejected API must neither clear caches nor enter JS and drain the queue.
        scope.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
        assert_eq!(style_revisions(), before);
        assert!(eval(scope, "recoveryMicrotasks===0").is_true());
        scope.perform_microtask_checkpoint();
        assert!(eval(scope, "recoveryMicrotasks===1").is_true());
    });
}
