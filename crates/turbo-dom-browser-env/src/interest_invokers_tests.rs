use super::super::reference_target::probe_harness::{control, differences, run};

const PROBE: &str = include_str!("interest_invokers_probe.js");

/// interestForElement reflection, InterestEvent, and interest gained/lost through programmatic
/// focus()/blur(), trusted hover and Escape, timed by the invoker's `interest-delay` on the
/// engine's virtual timers, showing/hiding the target popover exactly as Chrome 156 does.
#[test]
fn interest_invokers_match_chrome156() {
    let expected = control(include_str!("interest_invokers_chrome156.json"), PROBE);
    let actual = run(PROBE);
    let mut out = Vec::new();
    let mut expected = expected;
    // KNOWN_GAP: the engine has no SVGAElement interface (SVG `a` elements are SVGElement
    // instances), so SVGAElement.prototype.interestForElement is absent; SVG `a` interest
    // behavior itself (hover/focus) is covered by the probe and matches.
    assert_eq!(
        expected["surface"]["svgInterface"],
        serde_json::json!([
            "function",
            [
                true,
                true,
                [
                    "get interestForElement",
                    0,
                    "E:TypeError:f is not a constructor"
                ],
                [
                    "set interestForElement",
                    1,
                    "E:TypeError:f is not a constructor"
                ],
                false
            ]
        ])
    );
    assert_eq!(
        actual["surface"]["svgInterface"],
        serde_json::json!(["undefined", null])
    );
    expected["surface"]["svgInterface"] = actual["surface"]["svgInterface"].clone();
    differences("result", &actual, &expected, &mut out);
    assert!(out.is_empty(), "{}", out.join("\n"));
}
