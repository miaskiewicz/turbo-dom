# Private live stylesheet snapshots

This describes `turbo-dom-browser-env` (this crate), the DOM binding turbo-test and
turbo-surf link. Paths below are relative to `crates/turbo-dom-browser-env`.

The embedding host can retrieve a document's private native interface record with
`turbo_dom_browser_env::browser_env::get_interface_record`. Its `cssHost.exportSheet(sheet)` function
returns an owned data snapshot of genuine stylesheet state. It is not installed on
Window, Document, a DOM prototype or any guest-visible namespace. A forged sheet
fails with TypeError; the host must preserve this ownership boundary.

The null-prototype result contains `disabled`, normalized `media`, `href`,
`originClean` and `rules`. Rule snapshots retain `type` and current `cssText`;
private grouping lists additionally retain `prelude` and child `rules`, while
imports retain their `href`, `media` and actual attached `styleSheet`. A repeated
sheet on the current import path is represented by `{cycle:true}`. Export is
bounded to 20,000 rules, 1,024 sheets and 2,000,000 UTF-16 serialized text units.

The export reads private native records and captured native DOM accessors, never
public stylesheet/rule/element getters or public collection iterators. It observes
CSSOM declaration edits, media-list edits, native media attributes and disabled
state. Cross-origin sheets remain unreadable through public `cssRules`; the
privileged host snapshot retains their origin-clean flag and real rule contents
for native rendering. Never forward this raw privileged snapshot to page JS.

Stylesheet text uses the real native textContent getter. Page-defined textContent
or getAttribute properties cannot replace the underlying stylesheet. CSSOM edits
retain the sheet until native text changes, which creates a new sheet while the
retained old sheet keeps its rules. Font-face declaration parsing uses descriptor
validation, and direct setProperty additionally preserves Chrome's ordinary
property mutation behavior. Src URLs use the typed parser's quoted serialization.
The descriptor grammar follows [CSS Fonts 4](https://www.w3.org/TR/css-fonts-4/).

## Independent controls

`src/css_live_tests.rs` contains seven offline contracts, including the exact authored
public fixture in `src/css_live_probe.js` compared with the saved installed
Chrome155.0.8059.79 / V8 15.5.35.23 result. Refresh that control from this crate's
directory (the script reads `src/css_live_probe.js` and writes
`src/css_live_chrome155.json` unless `--out` names another file):

```sh
node scripts/css-live-chrome-control.mjs \
  --runner /absolute/path/to/turbo-surf/harness/detection/chrome-cdp-trace.mjs
```

The oracle uses only an authored loopback page and records its explicit
`--disable-extensions` diagnostic policy. Any CDP target setup or evaluation error
fails the control and retains its capture directory.

## Bounds

This is authoritative stylesheet state for an embedding layout engine, not a
layout implementation. It does not add missing grouping-rule interface members,
complete CSS nesting/cascade layers, font-resource loading, all font-descriptor
normalizations or layout geometry. Those remain independently testable engine
work. Exporting a declaration does not establish that a renderer supports it.
