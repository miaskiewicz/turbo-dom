# Bounded underlying ancestor reuse in one presentation sample

Each selected native target previously rebuilt its shared ancestors during
`sample_presented_effects`. A public four-case counter reproduction measured
7/56/11/176 fresh cascades for (depth, targets) (4,1)/(4,8)/(8,1)/(8,16), while
all current-name and literal-contribution assertions passed.

The private presentation reader now shares only validated underlying ancestor
snapshots within one synchronous batch. Every selected root remains fresh. It
uses the existing collector and exact world, native font/composition probe,
media-answer and dependency checks. Volatile, failed, missing or changed entries
recompute. Inspection never populates ordinary global style/base sharing.

Reader ownership uses weak opaque capabilities and passed-map identity, with no
global strong active reader. An aborted call cannot root/reuse its batch in the
next call; the pre-existing termination/inspection flag recovery issue remains
separate. No ordinary composed-style or public discovery mode is changed here.

Afterward the same public counts are 7/14/11/26: N selected roots plus D shared
ancestors and HTML/BODY. Five focused controls also preserve real ancestor hiding
between root reads, selected ancestor/descendant order, DOM/CSSOM/custom-validity/
media/composed-parent changes, and native font-default changes with unchanged
Tree.version. No budget, assertion, coverage floor, guard, or provenance is relaxed.

The explicit-font/color fixture isolates ancestor recomputation. Lazy inherited
defaults still have separate fresh-map paths; these counts do not claim every
application cost is solved. The normalized 421-element auth cost-only fixture
and live Surf outcomes are consumer acceptance checks, not pixel-parity claims.

Durable local logs and source hashes: `.evidence/batch-counts/`.

## Preserve the original local-read tier

The first implementation incorrectly applied cross-root eligibility checks to
hits inside the same root read. An animated ancestor is volatile for sharing,
but the original per-read map still resolved it only once. A public synthetic
control proved that regression with exact values: active-ancestor cases grew
to 60/2126/7568 cascades against original per-root bounds 12/25/46.

Each selected root now owns a fresh ordinary local map with its original hit
semantics. Only importing an ancestor into that map consults the separate shared
entry validation. The same cases use 8/17/32 cascades and 5/14/32 validation
visits (previously 645/23371/83242). Static inherited-context visits fall from
144/252/540 to 12/18/30. No memo, cross-call cache, or weaker world/probe/media
check was introduced. Test-only validation counters use the existing private
native-counter pattern and are absent from the shipped native API.

These source-derived work controls address a matched-debug-fixture regression;
they are not an optimized-release timing floor or proof of live app success.


## Optimized consumer acceptance

The immutable optimized Surf addon consuming producer
`410045a001a80ee0098fd30259e11b3604a629a6` has native SHA256
`708d3eff002ffc878e48e80d99194b3e4804abc3cda41716522a31480c7a893a`.
On the unchanged normalized 421-element offline fixture, public animation
discovery completes in 1600.106 ms on the first read and 66.439 ms on the second.
Both return eight animations, matching the Chromium clock-zero oracle. The old
optimized 89b artifact exceeded the unchanged 10000 ms read budget. No budget or
capability guard changed. Consumer receipt:
`turbo-surf/.recovery/sampler-offline-auth421/RESULT-DOM410-RELEASE.json`.

This accepts the combined producer corrections, including the original per-root
cache tier and context-independent transform path. It does not isolate either
change's individual share of elapsed time or claim live application acceptance.
The producer default workspace passes 3249 tests with zero failures and 13 ignored;
strict workspace all-target Clippy and formatting pass. Binding coverage refresh
and full consumer native/image/auth gates remain separate pending checks.
