#!/usr/bin/env bash
# Release prep: bump every version, re-lock, run the guards.
# Does NOT publish, does NOT tag — a human still runs the gate in PUBLISHING.md
# and merges the release PR.
#
# What it does:
#   1. Rewrites the version string in package.json, the root turbo-dom-parser
#      Cargo.toml, the four workspace crate manifests (turbo-dom, -layout,
#      -raster, -browser-env) and their intra-workspace path dependencies.
#      turbo-dom-v8 keeps its own upstream-derived version (its own release
#      script handles it).
#   2. Updates package-lock.json (`npm install --package-lock-only`).
#   3. Updates Cargo.lock (`cargo update -w --workspace`).
#   4. Rebuilds both WASM packages so pkg/package.json + pkg-web/package.json
#      carry the new version.
#   5. Runs the guards: check-versions.py, check-no-selector-dual.sh.
#
# After this, read PUBLISHING.md and run the full gate. Do not tag.
#
# Usage: scripts/agent/release-prep.sh X.Y.Z
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
. "$HERE/lib.sh"

v="${1-}"
case "$v" in
  '' | --help | -h) sed -n '1,/^set -euo/p' "$0" | sed '$d'; exit 1 ;;
  v*) die "pass the bare version, not a tag: ${v#v}" ;;
esac
printf '%s' "$v" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$' \
  || die "not a valid X.Y.Z version: $v"

cd "$REPO"

# 1. package.json
node -e '
  const fs = require("node:fs");
  const v = process.argv[1];
  for (const f of ["package.json"]) {
    const j = JSON.parse(fs.readFileSync(f, "utf8"));
    j.version = v;
    fs.writeFileSync(f, JSON.stringify(j, null, 2) + "\n");
  }
' "$v"

# 2. Root parser + workspace crates. A path-dep line like
#      turbo-dom = { path = "../turbo-dom", version = "0.7.0" }
# rewrites to the new version in-place.
for toml in Cargo.toml crates/turbo-dom/Cargo.toml crates/turbo-dom-layout/Cargo.toml \
            crates/turbo-dom-raster/Cargo.toml crates/turbo-dom-browser-env/Cargo.toml; do
  [ -f "$toml" ] || die "missing manifest: $toml"
  # top-level package version
  python3 - "$toml" "$v" <<'PY'
import sys, re, pathlib
f, v = sys.argv[1], sys.argv[2]
p = pathlib.Path(f)
s = p.read_text()
s = re.sub(r'(?m)^(version\s*=\s*")[^"]+(")', lambda m: m.group(1)+v+m.group(2), s, count=1)
# intra-workspace path-deps: a line that names a sibling and has `version = "..."`
for name in ("turbo-dom", "turbo-dom-layout", "turbo-dom-raster", "turbo-dom-browser-env"):
    s = re.sub(
        r'(\b' + re.escape(name) + r'\s*=\s*\{[^\}]*?\bversion\s*=\s*")[^"]+(")',
        lambda m: m.group(1)+v+m.group(2), s)
p.write_text(s)
PY
done

# 3. Lockfiles
npm install --package-lock-only --ignore-scripts
cargo update -w

# 4. WASM packages
npm run build:wasm

# 5. Guards
python3 scripts/check-versions.py
bash scripts/check-no-selector-dual.sh

echo
echo "release prep done for $v — review the diff, then follow PUBLISHING.md."
