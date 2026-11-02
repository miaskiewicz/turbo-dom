#!/bin/sh
# Run one labelled `cargo test` invocation for the baseline and record results.
# Usage: baseline-cargo.sh OUT_DIR LABEL REPO_DIR -- <cargo test args...>
set -u
out=$1 label=$2 repo=$3
shift 4
mkdir -p "$out"
scripts=$(cd "$(dirname "$0")" && pwd)
export TZ=UTC LANG=C CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
start=$(date +%s)
(cd "$repo" && cargo +1.98.0 test "$@") >"$out/$label.log" 2>&1
code=$?
echo "$label exit=$code secs=$(( $(date +%s) - start ))" >>"$out/SUMMARY"
python3 "$scripts/cargo-results.py" "$out/$label.log" >"$out/$label.tsv"
grep -ihE '^(SKIP|skip)[: ]|not on PATH' "$out/$label.log" | sort | uniq -c >"$out/$label.skips" || true
