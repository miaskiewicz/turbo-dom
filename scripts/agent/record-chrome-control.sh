#!/usr/bin/env bash
# Run a Chrome 156 control probe and write the fixture.
#
# All in-repo controls live at scripts/*-chrome-control.mjs and share the same
# contract:
#   node scripts/FOO-chrome-control.mjs [--chrome PATH] [--out FILE]
# with --chrome defaulting to the cached headless-shell and --out defaulting to
# the fixture path the Rust test reads.
#
# This wrapper:
#   1. biome-formats the probe first (CI's lint runs on repo files too, so a
#      hand-edited probe needs formatting before the commit lands),
#   2. ensures the cached Chrome 156 is on disk,
#   3. launches the probe with the locked `--use-mock-keychain --password-store=basic`
#      flags (the probe reads these from its own chromium.launch args; this wrapper
#      only sets the executable path),
#   4. writes to OUT or the probe's default.
#
# Usage: scripts/agent/record-chrome-control.sh PROBE.mjs [OUT.json]
#   PROBE   a scripts/*-chrome-control.mjs file or short name (its basename
#           without the `-chrome-control.mjs` suffix).
#   OUT     optional output path (overrides the probe's default).
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
. "$HERE/lib.sh"

[ $# -ge 1 ] || { sed -n '1,/^set -euo/p' "$0" | sed '$d'; exit 1; }
probe="$1"; out="${2-}"

# Resolve the probe path.
case "$probe" in
  */*|*.mjs) ;;
  *) probe="scripts/${probe}-chrome-control.mjs" ;;
esac
[ -f "$REPO/$probe" ] || die "probe not found: $probe"

need_path "CHROME156" "$CHROME156"

cd "$REPO"

# Biome-format the probe first. Only touches the one file so unrelated
# formatting drift does not sneak in.
if command -v npx >/dev/null 2>&1 && [ -f node_modules/@biomejs/biome/package.json ]; then
  npx --no-install biome format --write "$probe" >/dev/null || true
fi

args=(--chrome "$CHROME156")
[ -n "$out" ] && args+=(--out "$out")

set -x
exec node "$probe" "${args[@]}"
