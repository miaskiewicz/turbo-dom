# XPath value admission, v0.8.7 (#548)

## Causal baseline

At release `29760c8c403b4a42d4276bb9a2e7e4a43c0d927c`, the public document XPath
adapter eagerly copied every node value and attribute value from the owning
native document before evaluating an expression. An unrelated 1 MiB SCRIPT or
STYLE caused even `..` and `count(.)` on a small field to throw
`NotSupportedError: XPath text exceeds its byte budget`. An unrelated 1 MiB
attribute caused `XPath attributes exceed the byte budget`. The same CSS field
query succeeded. Both errors spend one shared cumulative byte counter: the error
names whichever field crosses the counter, not necessarily a single large field.

The exact MUI date-picker helper's relative ancestor/descendant union also failed:

```xpath
descendant-or-self::div[contains(@class,"MuiPickersSectionList-root")] | ancestor::*[contains(@class,"MuiPickersInputBase-root")][1]//div[contains(@class,"MuiPickersSectionList-root")]
```

A standalone native diagnostic and the new regression failed against that
baseline. An independent offline Chromium 159.0.8154.6 oracle returned the same
parent, count 1 and date-list count 1 in all four modes (control, SCRIPT, STYLE,
attribute). The portable collector is `scripts/xpath-value-admission-chrome-control.mjs`;
its source-hashed probe and control are `xpath_value_admission_probe.js` and
`xpath_value_admission_chrome159.json`. The native test replays that exact probe
and verifies its hash. This is a new Chromium 159 control, not a new 156 claim.

Surf's onboarding helpers explicitly use this XPath union and `xpath=..`.
Ordinary CSS/getByRole/getByText paths do not inherently invoke the producer
XPath provider. The full-suite receipts establish the same error class, but
contain no exact failing selector; the isolated reduction proves the producer
admission defect without claiming that every live-suite failure has one cause.

## Fix and retained bounds

The existing bounded snapshot retains topology, names and genuine native Attr/
Node identity. Value columns are placeholders. The private provider requests a
node/attribute value only when XPath reads it, reuses it within that evaluation,
and subtracts its UTF-8 length from the same remaining byte allowance. Native
reads check the captured native document/version before admitting a value.
The provider and its intrinsics remain in the existing private V8 context; no
new public DOM API, guest accessor or host-heap reference is exposed.

No SCRIPT/STYLE content is omitted from XPath. Explicit string/attribute reads
still include it and fail when they exceed the allowance. Two 600 kB values exceed
the cumulative bound; two reads of the same 600 kB attribute charge it once, as the
old snapshot did. UTF-8 byte boundaries are checked separately from JS UTF-16
length. Stale and foreign-owner stamps fail with InvalidStateError.

All original bounds remain: 4,096 nodes including attributes, ancestor/tree depth 128,
1 MiB total UTF-8 metadata plus demanded values, 4,096 UTF-16 expression units,
256 lexical tokens, nesting 32, and 200,000 operation reads. Large required metadata
still fails even for count(.). The whole-owner-document topology capture is
unchanged: this is value admission, not a general lazy-node XPath rewrite.

Existing byte-budget tests now explicitly demand the oversized strings/attributes
instead of assuming a structural count must copy unrelated values. Their exact
error assertions remain, supplemented by cumulative, metadata-only, repeated-value,
mutation, native identity, foreign-document and owner/version controls. No limit,
coverage floor, execution budget or resource allowance was increased.

## Validation

- Full workspace with `host-internals,test-support`: **3,187 passed, zero failed,
  13 ignored**, across 226 summaries. Two subsequently added probe/metadata tests
  also pass in the final native run; the final focused XPath suite is **17/17**.
- Final native-binding LLVM tarpaulin, offline/locked, original CI feature/source
  filter and 2,400-second timeout: **97.64%, 22,681/23,229 lines**, unchanged
  `--fail-under 97.6`. Its final native unit binary passed **846 tests, zero failed,
  four existing ignored**, alongside all binding integration/control binaries.
  `document_xpath.rs` is 183/186 covered lines; the new value-admission branch and
  owner/version/UTF-8 guards are covered.
- Strict workspace/all-target clippy with `-D warnings`, workspace fmt,
  repository Rust/JS complexity limits, new control/probe lint/format, version
  agreement and single-V8 graph: passed.
- Isolated churn: **372 allocations/op, 82,270 bytes/op**, below unchanged
  380/85,000 ceilings. Existing sibling-rank regression remains **zero allocations**.
- Independent Chromium control: **159.0.8154.6**; probe SHA-256
  `71090e718fe76baf1a0edb5bb2c6fa27ded5b55b9f63d0e4ae3d886d5597221a`.

Local baseline/diagnostic,
red/green, native gate and oracle receipts are retained under
`.evidence/xpath-budget-diagnosis/`; committed native tests/probe/control are the
portable reproductions. Toolchain Rust 1.98.0; at most two compilation jobs;
unchanged one-V8 graph. No Surf source, dependency pins, addon, services or
continuously leased fixtures were changed. Consumer replay requires a coherent
DOM-family repin and actual rebuilt addon after producer integration.

Implementation checkpoint: `909b897e139e6114be25606db7f01696d1ba2998` on
`codex/fix-087-xpath-value-admission`, based on
`29760c8c403b4a42d4276bb9a2e7e4a43c0d927c`. Integration is a normal merge
into `release/0.8.7`; tagging/publication/main integration are outside this change.
