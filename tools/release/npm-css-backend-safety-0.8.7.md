# Shared CSS backend safety checkpoint

The canonical token preflight now accounts for the pinned backend’s extra `env()` fallback edge before recursive admission. Caller limits remain independent. New raw condition AST helpers also refuse backend nesting overflow without inventing a source-size budget. Existing legacy APIs and runtime backend selection remain unchanged.

Default Core tests: 574 passed, 9 ignored. Feature Core tests: 785 passed, 9 ignored. Strict Clippy, formatting and CC9 passed. Structured backend-error conversion controls preserve typed errors without running known unsafe recursive input.

Coverage remains **red** at the unchanged 100% floors: default 14195/14326 (99.09%), feature 17111/17128 (99.90%). The default report includes 58 lines in a feature-disabled grammar module, under investigation; no coverage exclusion or gate adjustment was made. Remaining genuine helper branches still need meaningful controls.

This is a durable validated source increment, not completion of #296 or approval to switch the npm computed-style default. Artifact/runtime tests from earlier stages do not certify this newer source. Exact source and local evidence hashes are in the adjacent JSON. Clean committed-source locked validation will be recorded separately.
