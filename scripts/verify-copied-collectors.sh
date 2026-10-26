#!/bin/sh
# Verify the ONE-TIME MOVE (consolidation S5) of turbo-test's Chrome-control
# collectors and the live-css-host doc into crates/turbo-dom-browser-env.
#
# A review aid, NOT a sync tool: it never writes into the repository. turbo-test
# deletes its copies in its consumer step; this crate is their only home after that.
#
# Source: miaskiewicz/turbo-test@39b5911 (refactor/turbo-dom-consolidation)
#   scripts/*-chrome-control.mjs, scripts/typed-om-registry.mjs,
#   scripts/trusted-types-control/   -> crates/turbo-dom-browser-env/scripts/
#   docs/live-css-host.md            -> crates/turbo-dom-browser-env/docs/live-css-host.md
#
# Checks
#   1. the verbatim-copy commit (message "from miaskiewicz/turbo-test@39b5911") holds
#      exactly those files, byte-for-byte;
#   2. at REV (default HEAD) every moved file still exists; files edited since the
#      copy are printed as diffs (the path-repointing commit and later reviewed edits).
#
# Usage: scripts/verify-copied-collectors.sh [--turbo-test DIR] [REV]
#   DIR: a turbo-test clone containing 39b5911 (default: $TURBO_TEST_REPO, else
#   ../turbo-test next to this repository). TURBO_TEST_FETCH=1 fetches the commit
#   from https://github.com/miaskiewicz/turbo-test into a temp clone instead.
set -eu

SOURCE_SHA=39b59111d5f30fc2f00a2f98c1b33703f92b7e23
CRATE=crates/turbo-dom-browser-env

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
turbo_test=${TURBO_TEST_REPO:-$repo_root/../turbo-test}
if [ "${1:-}" = "--turbo-test" ]; then
  turbo_test=$2
  shift 2
fi
rev=${1:-HEAD}

work=$(mktemp -d "${TMPDIR:-/tmp}/verify-copied-collectors.XXXXXX")
trap 'rm -rf "$work"' EXIT INT TERM

if [ "${TURBO_TEST_FETCH:-0}" = 1 ]; then
  turbo_test=$work/turbo-test.git
  git init -q --bare "$turbo_test"
  git -C "$turbo_test" fetch -q --depth 1 https://github.com/miaskiewicz/turbo-test "$SOURCE_SHA"
fi
git -C "$turbo_test" cat-file -e "$SOURCE_SHA^{commit}" 2>/dev/null || {
  echo "turbo-test commit $SOURCE_SHA not found in $turbo_test" >&2
  echo "(pass --turbo-test DIR, set TURBO_TEST_REPO, or TURBO_TEST_FETCH=1)" >&2
  exit 2
}

# The moved set, laid out at its destination paths.
mkdir -p "$work/source/$CRATE"
git -C "$turbo_test" archive "$SOURCE_SHA" scripts docs/live-css-host.md | tar -x -C "$work/source/$CRATE"
(
  cd "$work/source/$CRATE/scripts"
  find . -type f ! -name '*-chrome-control.mjs' ! -name typed-om-registry.mjs \
    ! -path './trusted-types-control/*' -exec rm -f {} +
  find . -type d -empty -delete
)
(cd "$work/source" && find . -type f | sed 's|^\./||' | LC_ALL=C sort) >"$work/moved.list"
echo "source: miaskiewicz/turbo-test@$SOURCE_SHA, $(wc -l <"$work/moved.list" | tr -d ' ') moved files"

extract() { # $1 = rev, $2 = destination
  mkdir -p "$2"
  git -C "$repo_root" archive "$1" "$CRATE/scripts" "$CRATE/docs/live-css-host.md" | tar -x -C "$2"
}

copy=$(git -C "$repo_root" log --format=%H --grep="from miaskiewicz/turbo-test@39b5911" -- "$CRATE/scripts" | tail -n 1)
[ -n "$copy" ] || { echo "no verbatim-copy commit found" >&2; exit 2; }
extract "$copy" "$work/copy"
if diff -r "$work/source" "$work/copy" >"$work/diff.txt"; then
  echo "OK: $(git -C "$repo_root" rev-parse --short "$copy") is a byte-for-byte copy of the moved files"
else
  cat "$work/diff.txt"
  echo "FAIL: the verbatim-copy commit $copy differs from the source" >&2
  exit 1
fi

extract "$rev" "$work/rev"
status=0
edited=0
while IFS= read -r path; do
  if [ ! -f "$work/rev/$path" ]; then
    echo "FAIL: moved file missing at $rev: $path" >&2
    status=1
  elif ! cmp -s "$work/source/$path" "$work/rev/$path"; then
    edited=$((edited + 1))
    echo "== edited since the move: $path"
    diff -u "$work/source/$path" "$work/rev/$path" | sed "s|$work/[a-z]*/||" || true
  fi
done <"$work/moved.list"
[ "$status" = 0 ] && echo "OK: all moved files present at $rev ($edited edited since the move, diffs above)"
exit "$status"
