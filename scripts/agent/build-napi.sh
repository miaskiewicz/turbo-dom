#!/usr/bin/env bash
# Build the native html5ever addon (`turbo-dom-parser.<platform>.node` + the
# napi codegen `index.js`/`index.d.ts`) with the shared cache env already set.
#
# Pre-commit on this repo ONLY runs when src/ or test/ is staged — for other
# commits you do not need the addon, but any src/test change needs it present
# (npm test loads it). Run this before `git commit` in those cases.
#
# Usage: scripts/agent/build-napi.sh [--debug] [--ci]
#   --debug  napi build without --release (faster, slower runtime)
#   --ci     also run `npm ci` instead of a no-op install
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
. "$HERE/lib.sh"

mode=--release
install=none
for a in "$@"; do
  case "$a" in
    --help|-h) sed -n '1,/^set -euo/p' "$0" | sed '$d'; exit 0 ;;
    --debug) mode= ;;
    --ci) install=ci ;;
    *) die "unknown arg: $a" ;;
  esac
done

cd "$REPO"
case "$install" in
  ci) npm ci ;;
  none) [ -d node_modules ] || npm ci ;;
esac

# napi-rs CLI lives under node_modules; invoke via npx so a pinned version wins.
exec npx --no-install napi build --platform ${mode:+--release}
