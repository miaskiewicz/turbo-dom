# Rust CSS result protocols

The CSS backend remains `js` or `rust`. Both Rust result protocols use the same compiler, cascade, shared adapter and native model. JavaScript remains the default.

```js
const env = createEnvironment(html, {
  parser: 'native',
  cssBackend: 'rust',
  cssResultProtocol: 'native-state-v1',
});
```

`cssResultProtocol` defaults to `wire-v1`. `native-state-v1` requires `cssBackend: 'rust'`; incompatible or unknown values throw. `cssRequestProtocol` must remain `wire-v1` in this mode; selecting `context-template-v1` together with native-state results throws explicitly. `installGlobals` forwards the option and environment reset preserves it. Select another protocol by creating another environment.

## Callback contract

`wire-v1` preserves the established `resolveMatchingV1` then `serializeCssomV1` callbacks. `native-state-v1` explicitly selects `PreparedStylesV1.resolveStyleStateV1(resolveWire, matchingToken, projectionWire)` for eligible computed-style requests. Its overrides/errors remain observable. It does not invoke the two old callbacks for those requests. Applications instrumenting or replacing those callbacks should retain `wire-v1` unless deliberately implementing the new protocol.

The option is not evidence that an injected backend is genuine or pure. The adapter calls its selected object's method normally, then validates response ABI, target identity, values and used-property metadata. Missing retained-matching support or `resolveStyleStateV1` fails rather than silently selecting the old protocol. Fixed projection quotas are accepted only from a valid string-returning runtime-policy provider with the exact supported quota schema. Object-returning, malformed or extended quota policies do not qualify: explicit native-state requests fail with the original decode error or a capability error. Generic public profiles and ordinary wire requests remain unchanged.

Internal requests without a selected projection target retain the existing full-result operation. Used-property providers execute at their existing point. When projection reports layout-dependent properties, the result retains renderer/ancestor inputs for the existing provider. Computed-only reads return just the selected public/custom snapshot. Transport size changes; CSS property support does not.

## Lifetime, quotas and snapshots

The prepared owner and current matching token are checked before resolution. Matching replacement/disposal revoke cached state. Exact successful requests can reuse immutable native state; changed input or budgets validate again. Projection quotas are checked every call. Successful resolution is published before later projection errors, matching the old two-call sequence.

The native result cache is one exclusive mode: wire results or typed states. Mode switches follow successful resolution, never malformed/failed resolve input. At most eight entries share the existing 64 KiB result payload cap and remaining 256 KiB matching payload allowance. `retainedBytes` is logical payload, not total heap: keys, text, scalars, markers, defaults, ordered IDs and request bytes are charged; allocator/B-tree metadata and spare capacity are excluded. Oversized successes remain transient and clear older result entries without retaining oversized request copies.

CSS declarations own JavaScript snapshots, with no native result handle. They remain usable after later reads, reset or cache eviction. No per-property NAPI calls are introduced.

## Evidence and scope

The isolated 118-request corpus and packed 29-test suite passed. Calls decreased from 236 to 118 and response bytes from 163,348 to 12,194 for this corpus; pages requiring used values can retain full responses. The native CPU replay improved; whole packed wall/CPU were effectively flat. This is not a global speed or browser-parity claim. JavaScript remains the default.

Detailed counters, seven-block ABBA timings and gate results are recorded in [the benchmark receipt](benchmarks/css-native-style-state-2026-11-28.json). Native replay CPU fell 5.12% in all seven blocks; packed wall time was effectively flat (0.17% lower, four wins). Timings used the experimental selector for this protocol; the final public option wiring was separately validated with a fresh packed suite.

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
