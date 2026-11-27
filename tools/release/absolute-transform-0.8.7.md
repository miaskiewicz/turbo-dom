# #566: context-independent transform resolution

The public computed-style regression on89b88d8b was red: reading opacity on an element with literal matrix() entered the used-size provider once; public document.getAnimations discovered the real CSS opacity animation but entered its throwing provider twice. Relative percentage and inherited-em controls passed. The initial2FAIL/1PASS log is retained at `.evidence/absolute-transform/RED.log`.

The same resolved-transform parser now attempts a strictly context-independent parse before requesting layout. Its existing length parser refuses em/rem under that policy; percent bases are unknown, not zero, and the font basis is unknown, not16px. Parsed lengths and resulting matrix components must be finite. The existing matrix snapping/serialization is shared by both paths. Basis-dependent/uncertain lists retain the original provider path; inherited em, percentage and rem controls preserve that behavior. Local resolved CSS transforms remain local when ancestors are transformed.

Final validation is based onfabee8fc (the validated transition-creation integration). New5 controls plus animation-presentation5, presented-effects62 and transition-creation7 all pass:79 tests, zero failures/ignored. Strict host-internals all-target browser-env Clippy, Rust fmt/CC9 on new tests, JS syntax and diff checks pass. Logs live under `.evidence/absolute-transform`. This receipt does not claim the separate actual421-element Surf timing replay or combined coverage floor; those require the release coordinator's immutable consumer rebuild.


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
