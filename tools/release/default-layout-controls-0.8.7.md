# Default layout boundary controls

Test-only controls preserve the existing grid capacity (exact1000 tracks vs an additional clipped track), zero-span auto placement vs span1, avoid-page/avoid-column precedence in both directions and forced-page dominance. A private gradient diagnostic proves that a nonfinite direction emits no invalid tile while a normal direction does; public CSS admission already rejects nonfinite authored angles.

The public raw cascade test retains a lower valid fallback declaration before the important unresolved variable winner, plus an empty-input inverse. Separate release controls cover broader origin/encapsulation rank ordering; these assertions deliberately do not duplicate them.

A public empty multicol control preserves200px width and zero content height. Its inverse has a12px-tall child balanced across two columns, yielding6px column height. Independent offline Chromium159.0.8154.6 confirms both exact dimensions (`.evidence/default-controls/empty-multicol-chrome.json`). The initial new test incorrectly expected12px without balancing; that expectation was corrected using the independent oracle, with the initial failure retained.

No production behavior, cfg/features, floors, limits or coverage exemptions change. Focused public and private controls, strict default-feature all-target layout Clippy, fmt and CC9 are recorded under `.evidence/default-controls`. Combined coverage is owned by the release coordinator; a focused pass is not a floor claim.

## Combined text, cascade and Unicode controls

Additional tests preserve native text sources through context marking; verify raw
ordering across origins, importance, encapsulation, inline ranking and specificity;
reject malformed Unicode wildcards; and refuse fabricated placement after a real
imported text run loses glyph provenance. Native line-break membership, fontless
sticky fragments, uppercase expansion UTF16 cells and wrapped RTL collapsed-space
affinities have positive/inverse controls. These are tests only.

The combined default layout suite before the last four text controls passed
1,291 tests / 0 failed / 0 ignored. The latest default library run including those
four controls passed565 / 0 failed / 0 ignored. Strict Rust1.98 all-target layout
Clippy with -Dwarnings, fmt, configured global CC9 and the new-file CC9 check pass.
Logs: `.evidence/presented-effects/default-controls-combined.log`,
`default-reachable-lib.log`, `default-controls-clippy.log` and
`default-controls-complexity.log`.

The prior default coverage result99.71% remains a failed receipt until the exact
combined measurement completes. No floor, feature, timeout or assertion was weakened.
The unreachable duplicate-scope/caret and missing-inline-box defenses remain intact;
no malformed memory or invented geometry was used to force their counters.
