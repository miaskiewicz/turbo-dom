#!/usr/bin/env bash
# Shared helpers for scripts/agent/*.
# Source, don't exec: `. "$(dirname "$0")/lib.sh"`.
#
# Exports:
#   REPO           absolute repo root (the dir with package.json)
#   CACHE_ROOT     ~/.cache/turbo-consolidation
#   CHROME156      Chrome for Testing 156 headless-shell binary (macOS arm64 cache)
#   V8_ARCHIVE     prebuilt rusty_v8 150.4 static lib (.a.gz)
#   TARGET_DIR     shared CARGO_TARGET_DIR under the cache
#   RUSTY_V8_ARCHIVE / CARGO_TARGET_DIR  set in the environment for child processes
#
# Also prints, when run as a script with --help, a one-line probe of each path.
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CACHE_ROOT="${CACHE_ROOT:-$HOME/.cache/turbo-consolidation}"
CHROME156="${CHROME156:-$CACHE_ROOT/chrome156/chrome-headless-shell-mac-arm64/chrome-headless-shell}"
V8_ARCHIVE="${V8_ARCHIVE:-$CACHE_ROOT/v8cache/librusty_v8_simdutf_release_aarch64-apple-darwin.a.gz}"
TARGET_DIR="${TARGET_DIR:-$CACHE_ROOT/target/turbo-dom}"

export RUSTY_V8_ARCHIVE="${RUSTY_V8_ARCHIVE:-$V8_ARCHIVE}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$TARGET_DIR}"

# die MESSAGE — print to stderr and exit non-zero.
die() { printf '%s\n' "$*" >&2; exit 1; }

# need_path LABEL PATH — fail if PATH does not exist.
need_path() { [ -e "$2" ] || die "missing $1: $2"; }

if [ "${1:-}" = "--help" ] && [ "${BASH_SOURCE[0]}" = "${0}" ]; then
  cat <<EOF
scripts/agent/lib.sh — shared env for the agent helpers.

  REPO         $REPO
  CACHE_ROOT   $CACHE_ROOT
  CHROME156    $CHROME156  $( [ -x "$CHROME156" ] && echo '(ok)' || echo '(MISSING)' )
  V8_ARCHIVE   $V8_ARCHIVE  $( [ -f "$V8_ARCHIVE" ] && echo '(ok)' || echo '(MISSING)' )
  TARGET_DIR   $TARGET_DIR

Override any of these with env vars of the same name before invoking a helper.
EOF
fi
