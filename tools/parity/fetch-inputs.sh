#!/bin/sh
# Fetch the upstream parity inputs at their pinned commits into tools/parity/.cache
# (gitignored). Nothing is copied into this repo: the inputs are read straight from
# the source repos with `git archive`, so the corpus always renders the same bytes.
#
# Usage: tools/parity/fetch-inputs.sh [--from-local DIR]
#   --from-local DIR  read from local clones DIR/turbo-html2pdf and DIR/turbo-surf
#                     (default: fetch from github.com/miaskiewicz)
set -eu
here=$(cd "$(dirname "$0")" && pwd)
cache=${PARITY_CACHE:-$here/.cache}
# Pinned input revisions: the released mains the B-main baseline was captured on.
HTML2PDF_REV=e2d7c4dc6719dbd87fcf56d75c719fecf5bf50ba
SURF_REV=b1de4ebf74e87d73ed128ee3b36e13a40000c3fc
local_root=
if [ "${1:-}" = --from-local ]; then local_root=$2; fi

fetch() {
  repo=$1 rev=$2
  shift 2
  out="$cache/$repo"
  rm -rf "${out:?}"
  mkdir -p "$out"
  if [ -n "$local_root" ]; then
    src="$local_root/$repo"
  else
    src="$cache/.git-$repo"
    [ -d "$src" ] || git clone -q --bare "https://github.com/miaskiewicz/$repo" "$src"
    git -C "$src" fetch -q origin "+refs/heads/*:refs/heads/*"
  fi
  git -C "$src" archive "$rev" -- "$@" | tar -x -C "$out"
  echo "fetched $repo@$(git -C "$src" rev-parse --short "$rev") -> $out"
}

fetch turbo-html2pdf "$HTML2PDF_REV" crates/turbo-html2pdf-core/tests/fixtures benches/conformance/fixtures
fetch turbo-surf "$SURF_REV" harness ':(glob)rust/crates/turbo-surf-render/**/*.html'
