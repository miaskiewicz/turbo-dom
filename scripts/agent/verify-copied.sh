#!/usr/bin/env bash
# Run the three verify-copied guards in one go:
#   - scripts/verify-copied.sh            (turbo-dom-browser-env @cb5c64e)
#   - scripts/verify-copied-layout.sh     (turbo-dom-layout S1 copy)
#   - scripts/verify-copied-collectors.sh (turbo-dom-browser-env/scripts S5 copy)
#
# Each is already idempotent; this just chains them so an agent can validate
# all of the one-time copies with a single command.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
. "$HERE/lib.sh"
cd "$REPO"
for g in verify-copied.sh verify-copied-layout.sh verify-copied-collectors.sh; do
  echo "[verify] scripts/$g"
  bash "scripts/$g"
done
