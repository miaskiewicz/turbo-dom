# Interrupted frame bootstrap and isolate recovery — v0.8.7

The native DOM context registry is populated before the trusted browser bootstrap
runs. The private termination-recovery callback is captured later, after the
animation machinery is installed. A real interruption in between left a retained,
incomplete frame in that registry. Whole-isolate recovery incorrectly treated it
as a complete realm and returned `CallbackUnavailable`, preventing recovery of
otherwise complete documents.

The native regression creates a complete parent and an independent child, then
uses the existing deterministic native install-script watchdog to terminate the
child's first bootstrap script. The existing `SetupTerminated` record proves the
interruption even after its inner TryCatch clears V8's flag. After host cancellation,
recovery was RED with `CallbackUnavailable` instead of success.

With `host-internals`, installation now records an explicit native admission state.
Only a fresh bootstrap known to have actually terminated before callback capture
can be skipped. This does not repair that realm or make it publishable: the
embedder must still discard it under the existing `SetupTerminated` contract.
A complete parent is still visited and its computed state invalidated. Ready
realms never downgrade during reinstall; unavailable callbacks in ready or unknown
realms remain errors. An ordinary failed retry cannot reuse a previous interruption's
permission. Snapshot callbacks imply ready admission on reinstall; ordinary snapshot
recovery remains unchanged.

Because an install-script TryCatch can already have cleared V8 termination,
registered embedder init scripts also consult the existing actual
`SETUP_TERMINATED` marker. They must not execute guest initialization in an aborted
bootstrap. Successful installations retain their normal initialization order.

No guest property, callback, or registry lookup decides admission. The small native
context slot adds no context roots and is absent from the default build. Recovery
still requires explicit microtasks, no active JavaScript, host cancellation and all
watchdogs joined; it never cancels execution, advances time or drains microtasks.

Validation uses Rust 1.98.0 and the pinned V8 archive. The direct partial-child
regression and queued-init regression are both recorded RED before their respective
fixes. Seven admission controls then pass, including actual native interruption
immediately before and after capture, ready-slot/collected-weak refusal, unknown
lifecycle refusal, interrupted and ordinary retry behavior, and the snapshot-shaped
callback/no-native-marker reinstall case. Existing init-script controls pass 5/5,
all binding termination controls pass 17/17, and presented/recovery controls pass
73/73. The broader admission implementation passed the complete host profile with
1063 tests, zero failures and four existing ignores before the final narrow
init-script guard; that guard is covered by the causal regression and affected
init/termination groups. Final default compilation and strict all-target Clippy
are recorded in `.evidence/frame-recovery/PIPELINE.json`.

Raw local evidence remains under `.evidence/frame-recovery/`. The existing
`recover_after_termination` function has CC10 on both the baseline and this
candidate; new admission helpers and termination controls are within CC9. No
complexity limit, allow-list, timeout, assertion or coverage floor changed.


## Unchanged consumer regression acceptance

Surf copied its exact original `host_input.rs` into an isolated consumer and
selected immutable implementation `e6b9ee3dc610100857fb85cdddd8180dffe47453`
for all four producer dependencies. The original interrupted-frame test changes
from `CallbackUnavailable` on410 to **1 passed, zero failed**. The complete
unchanged host-input binary then passes **26/26**, in10.61 seconds. Source hash
equality is recorded in `turbo-surf/.recovery/recovery-e6b9-private/IDENTITY.json`;
receipts are `ROOT-HOST-INPUT.log` and `ROOT-HOST-INPUT-FULL.log` in that directory.

The pushed release integration is `e433aeb76d7c16a4cd8b0a3de05e9b55b841fe9a`,
with binding source identical to the tested implementation. Issue567 is closed
with the exact producer and consumer receipts. Surf's actual package remains410
until the separately validated badge-transition follow-up is combined; these
focused results do not claim a rebuilt-package/authentication-suite pass.
