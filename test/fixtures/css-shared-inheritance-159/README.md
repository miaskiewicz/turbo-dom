These are frozen Chrome 159 controls; they do not claim Chrome 158 identity.
`fixture.mjs` and `cases.json` retain their measured bytes. The receipt hashes the
fixture, cases and actual page probe function. The portable launcher only replaces
machine-local dependency/executable paths with explicit development inputs:
`CHROME159_PLAYWRIGHT_MODULE` and `CHROME159_EXECUTABLE`. Changing the launcher did
not remeasure or change its page probe function.

Real adapter tests require three independently built ABI1 CSS payload paths:
`TURBO_DOM_CSS_TEST_NATIVE`, `TURBO_DOM_CSS_TEST_WASM_NODE` and
`TURBO_DOM_CSS_TEST_WASM_WEB`. Missing payloads fail explicitly. The CSS backend is
the Root crate's optional `css-backend` feature, separate from an injected minimal
HTML ParserBinding; these tests do not add methods to that binding or switch the
published runtime default. Each forced WASM test runs in its own Node test process
and asserts that no native artifact was loaded.

The portable normal tests deliberately use the TEST-ONLY producer in
`test/helpers/css-shared-public-facts.mjs`. It reads genuine public DOM nodes and
exports mechanical attributes/text/IDs to the real canonical matcher and CSS
backend. It does not replace production capture, implement CSS grammar, create a
second DOM, or certify native brands, hostile capture, closed shadows or capture
performance. The production adapter remains unwired and requires its caller's
trusted capture producer. Production private-capture receipts are separate.

The `TURBO_DOM_CSS_TEST_*` inputs override the existing package conventions:
`native-css/turbo-dom-css.<host suffix>.node`, `pkg-css/turbo_dom_css.js` and
`pkg-css-web/turbo_dom_css.js`. Host suffix/musl detection and artifact ABI/version
validation reuse the existing resolver helpers. An explicit missing override
fails; it does not retry another artifact or the HTML parser binding.

CSS-kernel mode and HTML-parser mode are recorded separately by the test helper.
The tests explicitly call and verify `setParserMode('native'|'wasm')`. Both CSS
WASM formats use the ordinary Node HTML parser's `wasm` mode; the CSS web-ESM case
is not a claim that the HTML parser's browser glue ran in a browser.
