# Color transition tokens — #569

Issue: https://github.com/miaskiewicz/turbo-dom/issues/569

Implementation: `26c1a447056007bd41947f467b8c93a16c0f66df`, based on
`2e38c893d120cd9a7a4ecfa15e820c42ba4d0a2b`.

The public fixture changes disabled button colors from `rgba(0,0,0,.26)` and
`rgba(0,0,0,.12)` to inherited custom-property colors. Before the fix, the canonical
transition retained `var()` endpoints and returned Unknown. The fix resolves and
freezes supported color longhands through the existing bounded custom-token
resolver before transition creation. It preserves endpoint snapshots and existing
retargeting, cascade priority, composed inheritance and uncertainty guards. It adds
no interpolation engine, resource allowance, geometry/provider call or admission
fallback. Invalid/cyclic/rollback values, registered or animated custom-property
contexts and unproved deferred shorthands remain strict.

## Source identities

Paths below are under `crates/turbo-dom-browser-env/tests/fixtures/`.

| File | SHA-256 |
| --- | --- |
| `mui_color_transition.html` | `306b3ece109afcc95cff36410084c3278a83c9bb97efed0fb267863b7c245381` |
| `mui_color_transition_chrome159.json` | `843013c08735ca761175f5759e89bdf553f212a5e84267235f1e06c88a564114` |
| `mui_color_transition_oracle.mjs` | `3cdc3fa17a41b49465798a6523b05cf2d6206c051eb940578270c4c94615482a` |

The portable oracle records Chromium `159.0.8154.6`, canonical frame endpoints,
computed color values and rectangles at 0/125/250ms, and original frames after a
custom-property edit. It accepts `PLAYWRIGHT_MODULE` and `TURBO_CHROME_HEADLESS`
for an installed oracle; it does not download a browser or use a live site.

## Validation

Rust 1.98.0, one compiler job, pinned V8 archive:

- Public producer reduction: 0 passed / 1 failed before the fix.
- Complete presented-effects binary: **93 passed / 0 failed**.
- Strict all-target browser-env Clippy with `host-internals,test-support` and
  `-D warnings`: passed; rustfmt, JS syntax and scoped new-helper CC≤9 passed.
- Controls cover inherited/inline tokens, fallback/currentcolor, frozen old
  frames, exact token-versus-literal midpoint equivalence, invalid values,
  registered/dynamic contexts and deferred shorthand refusal. Canonical sampling
  makes zero layout/provider calls.
- Surf's unchanged public PageSession control was RED on `ad55a261` at the first
  enabled rectangle read, with the two original unmodified transitions. Its
  immutable private combined `a8d` replay is **1 passed / 0 failed**: width remains
  100 before/after and at 125ms, with two real transitions and resolved frame text.
  Consumer receipt: `turbo-surf/.recovery/presentation-diagnostic-a8d/ROOT-COLOR-GREEN.log`.

Producer raw logs and hashed receipt are retained locally at
`.worktrees/presented-color-tokens-087/.evidence/color-tokens/`. Parent release
integration owns the final workspace gates and remote branch identity; these
focused receipts do not claim a complete application/auth run.

## Separate existing color precision difference

At 125ms, literal and token versions give exactly the same canonical result.
For `.26` alpha black → white, that existing result is
`rgba(202, 202, 202, 0.63)` versus Chromium's
`rgba(203, 203, 203, 0.63)`. The source-hashed Chromium midpoint and original strict
comparison failure (`FOCUSED-03.log`) are retained. This fix changes no color math
and introduces no tolerance assertion: tests separately require exact boundary
values, frozen frame text and exact token/literal equivalence. The one-channel
precision discrepancy is not reported as fixed by #569.

## Integrated workspace gate

The production integration completed the default Rust workspace gate on Rust
1.98.0: 3,249 reported passes, zero failures and 13 ignored tests across 242
result summaries. Raw log: `.evidence/presented-effects/workspace-color-569.log`;
aggregate receipt: `WORKSPACE-COLOR-569.json`. This count is the Cargo summary
aggregate, not a distinct-case claim. Full application/auth acceptance remains
with the Surf consumer.

Strict Rust workspace all-target Clippy (`-D warnings`) and fmt also pass.
Surf subsequently passed the original delegated-write case unchanged on the
optimized private combined candidate (1/1, 117.56 seconds). A distinct employee
404-case prototype-admission refusal remains under separate investigation; this
receipt does not claim all 46 auth cases pass.
