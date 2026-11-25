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
