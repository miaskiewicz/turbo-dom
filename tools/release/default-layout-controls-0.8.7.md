# Default layout boundary controls

Test-only controls preserve the existing grid capacity (exact1000 tracks vs an additional clipped track), zero-span auto placement vs span1, avoid-page/avoid-column precedence in both directions and forced-page dominance. A private gradient diagnostic proves that a nonfinite direction emits no invalid tile while a normal direction does; public CSS admission already rejects nonfinite authored angles.

The public raw cascade test retains a lower valid fallback declaration before the important unresolved variable winner, plus an empty-input inverse. Separate release controls cover broader origin/encapsulation rank ordering; these assertions deliberately do not duplicate them.

A public empty multicol control preserves200px width and zero content height. Its inverse has a12px-tall child balanced across two columns, yielding6px column height. Independent offline Chromium159.0.8154.6 confirms both exact dimensions (`.evidence/default-controls/empty-multicol-chrome.json`). The initial new test incorrectly expected12px without balancing; that expectation was corrected using the independent oracle, with the initial failure retained.

No production behavior, cfg/features, floors, limits or coverage exemptions change. Focused public and private controls, strict default-feature all-target layout Clippy, fmt and CC9 are recorded under `.evidence/default-controls`. Combined coverage is owned by the release coordinator; a focused pass is not a floor claim.
