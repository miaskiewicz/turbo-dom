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

The fresh packed `b2eed4ed` package, including projection reuse and the refined
template envelope, was compared with its own JavaScript mode in seven ABBA blocks
per protocol. All 29 tests passed in every run. Ordinary wire Rust measured
+8.95% wall time and +12.38% child CPU; context-template requests +10.65% and
+14.71%; native-state results +7.61% and +11.06%. Large timing outliers are retained.
Owned builds were paused, but shared desktop/VM activity remains a limitation.
Separate windows do not rank protocols or establish a regression versus the
[earlier package](benchmarks/css-protocols-vs-js-2026-11-28.json).
JavaScript stays the default and the 2% wall-time target remains unmet. See the
[fresh receipt](benchmarks/css-accepted-b2-vs-js-2026-11-28.json) for raw timings,
artifact identities and the unmerged combined-protocol experiment.

## Exact projection reuse

Each retained immutable state can keep one successful projection response, keyed
by its exact projection wire. Every call still checks the prepared owner and
matching token and performs projection syntax, ABI and metadata validation.
An exact successful wire and the same immutable state certify the reused values
and quota outcome; changed options or limits take the ordinary projection path.
This also preserves the existing geometry-before-projection-error ordering.

Projection keys and response bytes consume only free space in the existing
shared result payload allowance. They do not evict typed results to fit. A late
publication cannot resurrect a retired state or cache mode. JavaScript still
receives a fresh declaration snapshot; current callbacks and errors remain
observable.

In the recorded 118-request replay, projection materializations fell from 118 to 68
while resolution hits/misses stayed 50/68. Native CPU fell 1.32%; packed wall time
fell 0.87% and CPU 1.20%, each with six wins across seven ABBA blocks. This is an
independent comparison against the preceding Rust build, not a new JS parity
measurement. See [the complete receipt](benchmarks/css-projection-reuse-2026-11-28.json).
