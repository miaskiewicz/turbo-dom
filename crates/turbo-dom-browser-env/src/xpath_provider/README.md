# XPath provider

`xpath.js` and `LICENSE` are unmodified files from the published MIT-licensed
[`xpath@0.0.35`](https://www.npmjs.com/package/xpath/v/0.0.35) package, maintained at
[`goto100/xpath`](https://github.com/goto100/xpath).

- Source SHA-256: `4ee74ef09301a2c5c5ce194367286fda65b65e589150456d5a9d19bc6b517224`
- Package integrity: `sha512-8O+m2N04SwLpGw/1Grz2HD8ejnWDV7M1dMPgKKuOjhYh5W9H4pel/+06kInohIdpS2FeSbL4i8ID8c4tpxwq9Q==`
- Source size: 175,034 bytes. No npm dependency or browser is required at runtime.

`document_xpath.rs` installs the provider in a private V8 context, prevents its
legacy automatic installation on the page document, and snapshots only the public
rtdom Tree surface. `document_xpath.js` supplies HTML name matching, correct
following/preceding axes, native Attr/Node identity, private cross-realm expression
and result brands, actual owner-realm prototypes, and mutation-invalidated iterators.
Neither provider internals nor snapshot nodes become page-visible. Only genuine
native nodes are returned through the public result facade.

The hard limits are 4,096 nodes including attributes, ancestor/tree depth 128,
1 MiB of UTF-8 snapshot strings, expression length 4,096 UTF-16 units, 256 lexical
tokens, nesting 32, and 200,000 snapshot reads per evaluation. Traversal executes
inside V8 and remains interruptible by the embedding runtime's guest watchdog.
Limits fail with `NotSupportedError`; they do not change the document.

Namespace-axis result nodes and fragment/shadow-root contexts are explicitly
unsupported. DOM XPath variables and extension functions are syntax errors. The
native HTML document binding supplies its existing namespace-aware foreign nodes;
this does not add an XML parser. The separate turbo-surf view crate's deliberately
minimal XPath helper is unchanged.

Eight deterministic Rust/V8 fixtures cover the authored Chrome 155 path/scalar/
axis matrix; native attributes/text; namespace callbacks and case; snapshot and
iterator mutation/adoption; receiver/result brands; hard budgets and recovery;
page builtin poisoning; real XPathEvaluator behavior; and borrowed foreign
interfaces with owner-realm results. The reusable Chrome/native collector lives
in turbo-surf's `harness/detection/document-factories-xpath-probe.js`.
