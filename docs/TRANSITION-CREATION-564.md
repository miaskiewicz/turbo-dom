# CSS transition creation parity (#564)

The public `getAnimations()` baseline disagreed with Chrome 159.0.8154.6 in six
of 24 static creation cases. Normal intrinsic-size and visibility pairs created
five effects Chrome did not create; an exiting `display` transition using
`allow-discrete` was missing. The independent failing test checkpoint is
`a9290094dcc6b57f87082ecb13cd4bf7407c764d`.

The correction distinguishes proven noninterpolable pairs from unmodeled values.
Normal `auto`/numeric size pairs respect inherited `interpolate-size`; discrete
behavior uses the transition property's list index. Actual effects involving
unsupported interpolation remain explicit `Unknown` samples. Replacing a running
transition with a nontransitionable pair cancels its canonical record without
marking it as script modified.

An exiting display transition requires a rendered before-change style and a valid
own display. Existing private display frames may retain that box through the
canonical scalar sampler at the pinned clock. This does not compose transforms or
sizes, invoke geometry providers, or bypass a hidden ancestor. CSS animation,
transition creation and presentation switches remain independent.

The source-hashed 24-case inputs and Chrome observations are retained in
`crates/turbo-dom-browser-env/tests/fixtures/transition_creation_*.json`.
`tests/transition_creation.rs` also covers inherited policy overrides, indexed
behavior lists, replacement cancellation, hidden ancestors, poisoned geometry
providers with a mixed display/transform effect, and independent switches.

Validation uses Rust 1.98.0, one compilation job, and the pinned V8 archive. The
final focused suite passes all seven tests, including all 24 oracle cases. The
full host crate suite passes 1,035 tests with four existing ignored tests. The
old none-to-length unsupported-interpolation test now explicitly enables
`allow-discrete`, preserving its `Unknown` assertion on a real Chrome effect.
No application success or unsupported-interpolation claim follows from these tests.
