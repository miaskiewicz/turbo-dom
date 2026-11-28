# Rust CSS request protocols

The CSS backend remains `js` or `rust`. Request and result protocols change transport, not the compiler, cascade or CSS implementation. Both protocols default to `wire-v1`, and JavaScript remains the default backend.

```js
const env = createEnvironment(html, {
  parser: 'native',
  cssBackend: 'rust',
  cssRequestProtocol: 'context-template-v1',
  cssResultProtocol: 'wire-v1',
});
```

The same options work under Vitest's `test.environmentOptions.turboDom` and Jest's `testEnvironmentOptions.turboDom`. `installGlobals` forwards them and reset preserves them. Unknown request protocols, selecting templates with JavaScript, or combining templates with `cssResultProtocol: 'native-state-v1'` throws before environment creation. The two optimized protocols have been tested independently; their combination is not supported. Neither option silently overrides the other.

## Callback contract

Every computed-style read still captures current input and performs the existing descriptor, scalar, cycle and quota validation. The default request protocol preserves the existing full-wire callbacks. Explicit `context-template-v1` uses `resolveRequestTemplateV1(fullWire, matchingToken)` for the first successful request with a new context bundle. Later reads use `resolveTemplateV1(compactWire, matchingToken, templateToken)`, or the original `resolveMatchingV1` when the completely freshly encoded request exactly equals the first full request. Normal CSSOM serialization remains unchanged. The three methods are invoked normally; current overrides and their original errors remain observable.

A missing retained-matching capability or template method on a selected computed-style request raises an explicit error. Internal unassociated scene/provider requests retain their existing full-wire route. Older native or WASM artifacts remain usable with the default protocol; the explicit template protocol requires the corresponding native methods.

## Lifetime and limits

Templates contain only contexts from a successful complete request. A fresh miss moves validated contexts from the original native parse, after target checks, cascade, result quotas and complete serialization. A full-wire cache hit still parses once to obtain contexts. No early context validation or duplicate typed cache changes error precedence.

At most 16 contexts are retained. The template token and context payload share the existing minimum of 64 KiB result payload and remaining 256 KiB matching allowance with all successful wire results. Oversized contexts do not retain a template. The adapter separately bounds its encoded context/shortcut strings to 32768 UTF16 units. Changed context, limits, ordering, matching owner or token cannot reuse an unrelated request. Errors do not become successful cache entries.

Native wire results and native-style-state results occupy exclusive cache modes. A successful state publication releases wire entries and template tokens; a later use of an old template fails. A failed resolve preserves the previous mode. An installation or projection error after successful resolution may leave that successful resolution published, preserving the established operation order. Concurrent compact results are retained only if the same template token remains current. Disposal and matching replacement revoke all associated retention.

`retainedBytes` reports logical payload, not allocator overhead. Old JavaScript declaration snapshots remain independent of native cache lifetime.

## Measured scope

The independently tested v1.2 candidate reduced request transport from 177716 to 127504 bytes across the authentic 118-request corpus: 14 installations, 93 compact calls and 11 exact-full-wire reuses. Exact output bytes, errors and owner/lifetime controls matched. The packed 29-test suite was 0.705% faster in seven of seven ABBA blocks; packed CPU was effectively flat. Native replay CPU did not improve (+0.617% with dynamic compact assembly; +1.285% with predetermined compact wires). These are workload-specific results, not a claim of native CPU improvement or parity with JavaScript. The integrated release-build comparison against JavaScript is recorded below.

## Integrated comparison against JavaScript

The committed `9576936a` package was measured against its own legacy JS mode in
seven ABBA blocks per supported protocol, with all 29 tests passing every run.
Context-template requests measured +8.51% whole-suite wall time and +11.90% child
CPU; native-state results measured +8.59% wall time and +12.20% child CPU. Ordinary
wire Rust measured +6.15% wall time and +9.46% child CPU. Each protocol used a
separate comparison window; these figures do not establish a ranking between
protocols. The initial opt-in comparisons encountered an unrelated compiler and
were repeated. Shared desktop/VM activity remains a limitation. JavaScript stays
the default, and the 2% wall-time gap target remains unmet. See the
[complete receipt](benchmarks/css-protocols-vs-js-2026-11-28.json) for every block,
CPU/callback metrics, artifact identities and qualifications.
