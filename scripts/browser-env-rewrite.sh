#!/bin/sh
# The one scripted rewrite of the turbo-test -> turbo-dom-browser-env binding move.
#
# Applied once (commit "browser-env: make the moved binding a crate") and re-applied
# by scripts/verify-copied.sh when it re-derives the move; it is not a sync tool.
#
# Rewrite: append the crate-surface hook to browser_env.rs. crate_surface.rs is a
# child module of `browser_env`, so the crate's public additions (document_html,
# install_independent_srcdoc_units, host_internals, test_support) can reach the
# binding's private items without making them public. No moved line changes.
#
# Usage: scripts/browser-env-rewrite.sh <binding-src-dir>
set -eu

src=${1:?usage: browser-env-rewrite.sh <binding-src-dir>}
file=$src/browser_env.rs
if grep -q '^mod crate_surface;$' "$file"; then
  echo "browser-env-rewrite: $file already rewritten" >&2
  exit 1
fi
cat >>"$file" <<'EOF'

// turbo-dom-browser-env crate surface (scripts/browser-env-rewrite.sh).
#[path = "crate_surface.rs"]
mod crate_surface;
pub use crate_surface::*;
EOF
