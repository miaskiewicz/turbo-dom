# Undefined Error stack hook — #570

Issue: https://github.com/miaskiewicz/turbo-dom/issues/570

A public paused width animation sampled successfully until ordinary
`Error.prepareStackTrace = undefined` added an own data property. The exact app
witness has writable/enumerable/configurable all true. The all-built-in shape guard
then refused canonical sampling, despite no callable stack formatter or accessor.

The fix permits only that one added key on the captured original Error constructor,
only when absent from its baseline and still undefined own data with those exact
assignment flags. Original global binding, parent identity, all other keys/order
and all descriptors remain checked. It does not forgive a preexisting hook change,
Error.prototype, other undefined keys, inherited shape changes, callable hooks,
accessors or other flag combinations. No new public intrinsic is called.

Validation on Rust1.98 with one compiler job:

- Public producer reduction: RED0/1 → GREEN; complete presented-effects binary96/96.
- Wrong-flag, callable/accessor (getter counter0), other-key, Error.prototype,
  Function.prototype and preexisting-baseline descriptor controls remain strict.
- Chromium159.0.8154.6 independently reports identical default stack text and
  geometry width150 before/after assignment; an accessor is not invoked by its
  geometry read. Producer deliberately retains its stricter no-getter contract.
- New/changed JS helpers and Rust tests pass scopedCC≤9, syntax and rustfmt.
  Strict all-target Clippy with host-internals,test-support and -Dwarnings passes.

Public oracle files under `crates/turbo-dom-browser-env/tests/fixtures/`:

| File | SHA-256 |
| --- | --- |
| `error_stack_hook_oracle.mjs` | `8fc7e57570860d00999d567342d768fd53b27b1ded15ed4dfaa7bc960a7fdba3` |
| `error_stack_hook_chrome159.json` | `4b0417f6c6e57ecb2fe3f078dfd19b9989e10d9eb940472d856a43a6fdf95c1e` |

Root's independent PageSession baseline is RED on the same paused width150
control; actual consumer acceptance remains a separate replay. Raw producer logs
are retained in `.worktrees/presentation-stack-hook-087/.evidence/stack-hook/`.
This production branch starts at `3b1b6e82`; private diagnostic ancestry is absent.
