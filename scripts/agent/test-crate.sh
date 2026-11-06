#!/usr/bin/env bash
# Run one crate's tests with the correct features and shared cache env.
#
# Usage: scripts/agent/test-crate.sh CRATE [TEST [-- cargo-args...]]
#   CRATE   turbo-dom | turbo-dom-layout | turbo-dom-raster | turbo-dom-browser-env
#   TEST    optional test name / filter (passed as the cargo test <NAME> arg)
#
# Feature flags match ci.yml for each crate.
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
. "$HERE/lib.sh"

[ $# -ge 1 ] || { sed -n '1,/^set -euo/p' "$0" | sed '$d'; exit 1; }
crate="$1"; shift
filter="${1-}"; [ $# -gt 0 ] && shift || true

features=()
case "$crate" in
  turbo-dom|turbo-dom-layout|turbo-dom-raster) ;;
  turbo-dom-browser-env)
    need_path "V8_ARCHIVE" "$V8_ARCHIVE"
    features=(--features host-internals,test-support)
    ;;
  *) die "unknown crate: $crate" ;;
esac

cd "$REPO"
set -x
exec cargo test -p "$crate" "${features[@]}" ${filter:+"$filter"} "$@"
