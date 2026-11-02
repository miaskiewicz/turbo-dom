//! The JSON contract of the binding's private CSS export.
//!
//! `browser_env.js`'s private `cssHost.exportSheet(sheet)` (reached through
//! [`host_internals::interface_record`](crate::host_internals::interface_record)
//! on a document) returns one [`SheetSnapshot`]: the sheet with its rules in true
//! sheet/rule order, including grouping-rule children and imported sheets.
//! Hosts deserialize it (e.g. `serde_v8::from_v8`) and evaluate media/supports
//! conditions themselves, rather than trusting a page-replaceable JS getter.
//!
//! Only the exported fields are modelled; hosts that annotate rules keep their
//! own side data (turbo-surf keys its scoped-selector preparation by rule).
use serde::Deserialize;

/// One style sheet as exported by `cssHost.exportSheet`.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SheetSnapshot {
    /// The sheet is part of an `@import` cycle and never applies.
    #[serde(default)]
    pub cycle: bool,
    /// `StyleSheet.disabled`.
    #[serde(default)]
    pub disabled: bool,
    /// The sheet's media list text (`""` = all).
    #[serde(default)]
    pub media: String,
    #[serde(default)]
    pub rules: Vec<RuleSnapshot>,
}

/// One CSS rule as exported by `cssHost.exportSheet`.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RuleSnapshot {
    /// `CSSRule.type` (1 style, 3 import, 4 media, 12 supports, ...; 0 for rule
    /// kinds without a legacy type constant, e.g. `@layer`).
    #[serde(rename = "type")]
    pub kind: u32,
    /// `CSSRule.cssText`.
    pub css_text: String,
    /// Grouping rules: the trusted prelude text (`@media screen`, `@supports ...`).
    #[serde(default)]
    pub prelude: Option<String>,
    /// Grouping rules: their child rules.
    #[serde(default)]
    pub rules: Option<Vec<RuleSnapshot>>,
    /// `@import`: the import's media list text.
    #[serde(default)]
    pub media: String,
    /// `@import`: the imported sheet, once loaded.
    #[serde(default)]
    pub style_sheet: Option<SheetSnapshot>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_the_export_shape_and_ignores_unmodelled_fields() {
        let sheet: SheetSnapshot = serde_json::from_value(serde_json::json!({
            "href": "https://layout.test/main.css", "originClean": true, "media": "screen",
            "rules": [
                {"type": 3, "cssText": "@import url(child.css);", "media": "screen",
                 "styleSheet": {"rules": [{"type": 1, "cssText": ".x{width:10px}"}]}},
                {"type": 4, "cssText": "@media screen {.x{width:20px}}", "prelude": "@media screen",
                 "rules": [{"type": 1, "cssText": ".x{width:20px}"}]}
            ]
        }))
        .unwrap();
        assert_eq!(sheet.media, "screen");
        assert!(!sheet.cycle && !sheet.disabled);
        assert_eq!(sheet.rules[0].kind, 3);
        assert_eq!(
            sheet.rules[0].style_sheet.as_ref().unwrap().rules[0].css_text,
            ".x{width:10px}"
        );
        assert_eq!(sheet.rules[1].prelude.as_deref(), Some("@media screen"));
        assert_eq!(sheet.rules[1].rules.as_ref().unwrap().len(), 1);
        let cyclic: SheetSnapshot =
            serde_json::from_value(serde_json::json!({"cycle": true})).unwrap();
        assert_eq!(
            cyclic,
            SheetSnapshot {
                cycle: true,
                ..Default::default()
            }
        );
    }
}
