//! turbo-dom#422: `break-before` / `break-after` / `break-inside` keyword acceptance parity
//! with Chrome 156. The fixture (`scripts/css-break-webkit-chrome-control.mjs`) records, per
//! property+keyword, `CSS.supports(prop, kw)` and the declaration's held value. Every keyword
//! Chrome rejects must resolve to [`BreakRule::Auto`] through
//! [`turbo_dom_layout::test_support::break_rule_of`] — an invalid value is ignored and the
//! property keeps its initial `auto`. Every keyword Chrome accepts must map to a concrete
//! non-`Auto` rule (except `auto` itself).
//!
//! `break-inside` is scored separately: `BreakRule` collapses column vs page avoids, but
//! the keyword-level accept/reject gate still applies, and `break_inside_*_avoid_of` is the
//! subset used downstream.

use turbo_dom_layout::style::ComputedStyle;
use turbo_dom_layout::test_support::{
    break_inside_avoid_of, break_inside_column_avoid_of, break_rule_of, BreakRule,
};

type BreakRow = (String, String, bool, String);

fn load_breaks() -> Vec<BreakRow> {
    let raw = include_str!("fixtures/css_break_chrome156.json");
    let v: serde_json::Value = serde_json::from_str(raw).expect("fixture parse");
    serde_json::from_value(v["breaks"].clone()).expect("breaks shape")
}

fn style_with(prop: &str, value: &str) -> ComputedStyle {
    ComputedStyle::from_pairs([(prop.to_string(), value.to_string())])
}

#[test]
fn break_rule_of_matches_chrome156_accept_reject() {
    let breaks = load_breaks();
    let mut mismatches: Vec<String> = Vec::new();
    for (prop, kw, chrome_supports, held) in &breaks {
        if prop == "break-inside" {
            let s = style_with(prop, kw);
            let got_avoid = break_inside_avoid_of(&s) || break_inside_column_avoid_of(&s);
            let expected_avoid =
                *chrome_supports && (kw == "avoid" || kw == "avoid-page" || kw == "avoid-column");
            if got_avoid != expected_avoid {
                mismatches.push(format!(
                    "{prop}: {kw}: ours_avoid={got_avoid} expected_avoid={expected_avoid} chrome_held={held:?}"
                ));
            }
            continue;
        }
        let s = style_with(prop, kw);
        let got = break_rule_of(&s, prop);
        let got_is_auto = matches!(got, BreakRule::Auto);
        // Chrome accepted a keyword iff it serializes as itself (not empty). A keyword Chrome
        // rejected (`held == ""`) must land on BreakRule::Auto.
        if !chrome_supports {
            if !got_is_auto {
                mismatches.push(format!(
                    "{prop}: {kw}: Chrome rejects (held={held:?}) but break_rule_of returned {got:?}"
                ));
            }
            continue;
        }
        // Accepted. `auto` -> BreakRule::Auto; everything else -> non-Auto.
        if kw == "auto" {
            if !got_is_auto {
                mismatches.push(format!(
                    "{prop}: auto: expected BreakRule::Auto, got {got:?}"
                ));
            }
        } else if got_is_auto {
            mismatches.push(format!(
                "{prop}: {kw}: Chrome accepts but break_rule_of resolved to Auto"
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "break_rule_of disagrees with Chrome 156 on {} keywords:\n  {}",
        mismatches.len(),
        mismatches.join("\n  "),
    );
}
