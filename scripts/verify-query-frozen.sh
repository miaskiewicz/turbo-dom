#!/usr/bin/env bash
# The `selector-dual` gate's FROZEN selector engine (turbo-dom consolidation S3a; see
# docs/selector-dual-gate.md): crates/turbo-dom/src/rtdom/query_frozen.rs and
# query_frozen_utf16.rs are B-main's query.rs / query_utf16.rs (turbo-dom a49793a,
# v0.5.8) after ONLY the scripted rewrites in scripts/verify-query-frozen.rules
# (fns in `impl Tree` prefixed `frozen_`, own caches, nested #[path], test module
# renamed). This script re-derives both files from git and diffs them.
#
# usage: scripts/verify-query-frozen.sh            # check (exit 1 on any difference)
#        scripts/verify-query-frozen.sh --write    # regenerate the two files
#
# Temporary: the frozen copy, this script and its rules are deleted with the gate in
# the last S3b commit.
set -euo pipefail

FROZEN_SHA=4d5a63df36f3f94fcfb22b184039b25afd485640
root=$(cd "$(dirname "$0")/.." && pwd)
rules=$root/scripts/verify-query-frozen.rules
mode=${1:-check}

python3 - "$root" "$rules" "$FROZEN_SHA" "$mode" <<'PY'
import subprocess, sys

root, rules_path, sha, mode = sys.argv[1:5]
pairs = [
    ("crates/turbo-dom/src/rtdom/query.rs", "crates/turbo-dom/src/rtdom/query_frozen.rs"),
    ("crates/turbo-dom/src/rtdom/query_utf16.rs", "crates/turbo-dom/src/rtdom/query_frozen_utf16.rs"),
]
rules = []
for line in open(rules_path):
    line = line.rstrip("\n")
    if line.strip() and not line.lstrip().startswith("#"):
        rules.append(tuple(line.split("\t", 1)))

bad = 0
for src, dst in pairs:
    data = subprocess.run(["git", "-C", root, "show", f"{sha}:{src}"],
                          capture_output=True, check=True).stdout
    for target, expr in rules:
        if target == dst:
            data = subprocess.run(["perl", "-0777", "-pe", expr], input=data,
                                  capture_output=True, check=True).stdout
    path = f"{root}/{dst}"
    if mode == "--write":
        open(path, "wb").write(data)
        print(f"wrote {dst}")
        continue
    try:
        have = open(path, "rb").read()
    except FileNotFoundError:
        have = None
    if have != data:
        bad += 1
        print(f"MISMATCH {dst} (regenerate with --write and review the diff)")
    else:
        print(f"ok {dst} == {src}@{sha[:7]} + rules")
sys.exit(1 if bad else 0)
PY
