#!/usr/bin/env bash
# Thin wrapper around scripts/check-versions.py (the CI guard that every
# manifest, lockfile and WASM package.json carries the same version).
#
# Usage: scripts/agent/check-versions.sh [--tag vX.Y.Z]
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
. "$HERE/lib.sh"
cd "$REPO"
exec python3 scripts/check-versions.py "$@"
