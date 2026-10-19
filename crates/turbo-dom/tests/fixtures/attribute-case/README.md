# Attribute-selector case control

`chrome155.json` is a genuine normal, headed, installed Chrome 155 control, captured
with a fresh profile and direct CDP. `probe.js` constructs the fixture and records
64 selector cases through document/element/fragment/shadow queries, `matches` and
`closest`, plus five XML controls. The captured outcomes are observations, not
handwritten expectations.

Run with Node 22 or later (global `WebSocket` required), using a **new** output path:

```sh
node crates/turbo-dom/tests/fixtures/attribute-case/collect.mjs /tmp/attribute-case-new.json
```

The collector defaults to installed macOS Chrome; `TURBO_SURF_CHROME_PATH` can
select another installed executable. It records actual browser/V8/Node versions,
probe/collector/fixture hashes, sanitized launch flags and uncaught page errors.
It refuses to overwrite evidence, bounds startup/socket/CDP/readiness waits and
checks that its executed probe and collector bytes remain unchanged. Expected
selector exceptions are caught per API and do not appear as uncaught errors.

The six comparison operators are measured with default, `i` and `I` matching.
Other controls cover ASCII-only folding, quoted/unquoted/escaped values and flags,
comments, invalid and duplicate modifiers, HTML versus SVG/XML attribute behavior,
and preservation of literal UTF-16 code units. A literal lone surrogate matches
its literal attribute; a CSS hexadecimal surrogate escape resolves to U+FFFD.
`fixtureHtmlUnits`, `mutations[].valueUnits` and `selectorUnits` preserve these
values without lossy UTF-8 conversion. In schema version 2 an ill-formed scalar
`selector` display is `null`; `selectorEscapedUtf16` labels its code units safely.
The collector still executes the original unmodified probe and original selector.

Chrome rejects `s`/`S` and their escaped spelling in ordinary selector lists and
`:not`; forgiving `:is`/`:where` discard that invalid arm. Public Rust selector
support for standards-defined `s` matching is a separate policy and must not be
claimed as Chrome agreement. The bounded public Rust default policy covers the
three measured HTML `input[type]`, `button[type]` and `ol[type]` cases, with
ASCII-normalized HTML attribute names; SVG/XML behavior remains case-sensitive by default. This does
not claim all HTML enumerated attributes. Tests identify 57 Chrome-comparable
cases and retain seven `s`-policy cases separately for browser-binding admission.

This control covers attribute matching and these specific API boundaries. It does
not certify the complete selector grammar or all HTML enumerated attributes.
