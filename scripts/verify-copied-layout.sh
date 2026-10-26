#!/usr/bin/env bash
# Re-derive the code that the consolidation's S1 step MOVED into turbo-dom-layout and
# turbo-dom-raster from its source repos (via `git archive` at the pinned SHAs) and
# diff it against this tree, allowing only the scripted rewrites listed in
# scripts/verify-copied-layout.rules.
#
# usage: scripts/verify-copied-layout.sh [--list-new]
#   env HTML2PDF_REPO  checkout of miaskiewicz/turbo-html2pdf (default ../turbo-html2pdf)
#   env SURF_REPO      checkout of miaskiewicz/turbo-surf     (default ../turbo-surf)
#
# Mapping: scripts/verify-copied-layout.map, one entry per line:
#   <mode> <repo>:<source path> <destination path>
# where <repo> is h2p (turbo-html2pdf @ H2P_SHA) or surf (turbo-surf @ SURF_SHA) and a
# directory source maps file-by-file. Modes:
#   verbatim  destination == source after the rewrite rules for that destination
#   subseq    destination is a subsequence of the (rewritten) source lines: lines were
#             only deleted (a test file split between turbo-dom-layout and html2pdf
#             core, or a function split out of a larger file)
# Every other file under the two crates is printed as NEW (hand-written; review it).
# Exit status is non-zero on any mismatch.
set -euo pipefail

H2P_SHA=e2d7c4dc6719dbd87fcf56d75c719fecf5bf50ba
SURF_SHA=b1de4ebf74e87d73ed128ee3b36e13a40000c3fc

root=$(cd "$(dirname "$0")/.." && pwd)
h2p_repo=${HTML2PDF_REPO:-$root/../turbo-html2pdf}
surf_repo=${SURF_REPO:-$root/../turbo-surf}
map=$root/scripts/verify-copied-layout.map
rules=$root/scripts/verify-copied-layout.rules

tmp=$(mktemp -d "${TMPDIR:-/tmp}/verify-copied-layout.XXXXXX")
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$tmp/h2p" "$tmp/surf"
git -C "$h2p_repo" archive "$H2P_SHA" LICENSE crates/turbo-html2pdf-core | tar -x -C "$tmp/h2p"
git -C "$surf_repo" archive "$SURF_SHA" rust/crates/turbo-surf-raster | tar -x -C "$tmp/surf"

python3 - "$root" "$tmp" "$map" "$rules" "${1:-}" <<'PY'
import fnmatch, os, re, subprocess, sys

root, tmp, map_path, rules_path, flag = sys.argv[1:6]

rules = []  # (glob, perl expression)
for line in open(rules_path):
    line = line.rstrip("\n")
    if not line.strip() or line.lstrip().startswith("#"):
        continue
    glob, expr = line.split("\t", 1)
    rules.append((glob, expr))

def rewrite(dst, data):
    for glob, expr in rules:
        if fnmatch.fnmatch(dst, glob):
            data = subprocess.run(["perl", "-pe", expr], input=data,
                                  capture_output=True, check=True).stdout
    return data

def subseq(small, big):
    it = iter(big)
    return all(any(line == b for b in it) for line in small)

entries = []
for line in open(map_path):
    line = line.split("#", 1)[0].strip()
    if not line:
        continue
    mode, src, dst = line.split()
    repo, path = src.split(":", 1)
    base = os.path.join(tmp, repo, path)
    if os.path.isdir(base):
        for d, _, files in os.walk(base):
            for f in files:
                rel = os.path.relpath(os.path.join(d, f), base)
                entries.append((mode, os.path.join(base, rel), os.path.join(dst, rel)))
    else:
        entries.append((mode, base, dst))

bad = 0
covered = set()
for mode, src, dst in entries:
    covered.add(dst)
    full = os.path.join(root, dst)
    if not os.path.exists(full):
        print(f"MISSING  {dst}")
        bad += 1
        continue
    want = rewrite(dst, open(src, "rb").read())
    have = open(full, "rb").read()
    if mode == "verbatim":
        ok = want == have
    elif mode == "subseq":
        ok = subseq(have.splitlines(), want.splitlines())
    else:
        raise SystemExit(f"unknown mode {mode}")
    if not ok:
        bad += 1
        print(f"DIFF     {mode:8} {dst}")
        if mode == "verbatim":
            p = subprocess.run(["diff", "-u", "-", full], input=want, capture_output=True)
            sys.stdout.write(p.stdout.decode(errors="replace")[:4000])

tracked = subprocess.run(
    ["git", "-C", root, "ls-files", "crates/turbo-dom-layout", "crates/turbo-dom-raster"],
    capture_output=True, check=True, text=True).stdout.split()
new = [f for f in tracked if f not in covered]
if flag == "--list-new":
    for f in new:
        print(f"NEW      {f}")
print(f"checked {len(entries)} copied files, {len(new)} new files, {bad} mismatches")
sys.exit(1 if bad else 0)
PY
