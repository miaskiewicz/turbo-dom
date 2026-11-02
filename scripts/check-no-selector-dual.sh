#!/bin/sh
# Guard: the consolidation's temporary selector dual-engine gate (S3a/S3b) is gone and
# must not come back. Fails if any Cargo manifest declares or enables a `selector-dual`
# feature, if a frozen engine copy (`query_frozen*.rs`) or a dual layer (`selector_dual.rs`)
# exists, or if source, build, script or workflow files mention `selector-dual`,
# `selector_dual`, `query_frozen` or `TURBO_DOM_SELECTOR_ENGINE`. Documentation (*.md)
# may record the gate's history, and any file may name this guard script itself.
#
# usage: scripts/check-no-selector-dual.sh [DIR]    (default: this repository)
# Publish workflows (here and in consumers checking their own tree) run it before
# publishing; exit 1 on any hit.
set -eu
dir=$(cd "${1:-$(dirname "$0")/..}" && pwd)
self=$(cd "$(dirname "$0")" && pwd)/$(basename "$0")
status=0
files=$(find "$dir" -name query_frozen\*.rs -o -name selector_dual.rs \
  | grep -v -e /target/ -e /node_modules/ -e /.git/ -e /.cache/ || true)
if [ -n "$files" ]; then
  echo "check-no-selector-dual: dual-gate sources present:"
  echo "$files"
  status=1
fi
hits=$(grep -rIln -E 'selector-dual|selector_dual|query_frozen|TURBO_DOM_SELECTOR_ENGINE' "$dir" \
  --include='*.rs' --include='*.toml' --include='*.sh' --include='*.py' --include='*.mjs' \
  --include='*.js' --include='*.ts' --include='*.yml' --include='*.yaml' --include='*.json' \
  --exclude-dir=target --exclude-dir=node_modules --exclude-dir=.git --exclude-dir=.cache \
  | grep -v -x -F "$self" \
  | while IFS= read -r file; do
      # Naming this guard (check-no-selector-dual.sh, e.g. in a workflow step) is fine.
      sed 's/check-no-selector-dual//g' "$file" \
        | grep -q -E 'selector-dual|selector_dual|query_frozen|TURBO_DOM_SELECTOR_ENGINE' \
        && echo "$file"
    done || true)
if [ -n "$hits" ]; then
  echo "check-no-selector-dual: files mention the removed selector dual gate:"
  echo "$hits"
  status=1
fi
[ "$status" -eq 0 ] && echo "check-no-selector-dual: ok ($dir)"
exit "$status"
