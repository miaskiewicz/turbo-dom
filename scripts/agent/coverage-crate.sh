#!/usr/bin/env bash
# Run cargo-tarpaulin for one crate against its ci.yml floor.
#
# Usage: scripts/agent/coverage-crate.sh CRATE [VARIANT]
#   CRATE    turbo-dom | turbo-dom-parser | turbo-dom-layout | turbo-dom-raster
#            | turbo-dom-browser-env
#   VARIANT  optional: for turbo-dom-raster use 'avif' | 'jxl';
#            for turbo-dom-layout use 'all-features'.
#
# Floors + feature flags match ci.yml's coverage matrix.
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
. "$HERE/lib.sh"

[ $# -ge 1 ] || { sed -n '1,/^set -euo/p' "$0" | sed '$d'; exit 1; }
crate="$1"; variant="${2-}"
floor=100 features=() files="crates/$crate/src/*" timeout=600

case "$crate" in
  turbo-dom) ;;
  turbo-dom-parser) files="src/core.rs" ;;
  turbo-dom-layout)
    floor=99.8
    if [ "$variant" = "all-features" ]; then
      features=(--features turbo-dom-layout/svg,turbo-dom-layout/perf,turbo-dom-layout/serde,turbo-dom-layout/test-support)
    fi ;;
  turbo-dom-raster)
    case "$variant" in
      '') ;;
      avif|jxl) features=(--features "$variant") ;;
      *) die "turbo-dom-raster variant must be '' | avif | jxl" ;;
    esac ;;
  turbo-dom-browser-env)
    floor=97.6; timeout=2400
    features=(--features host-internals,test-support)
    need_path "V8_ARCHIVE" "$V8_ARCHIVE" ;;
  *) die "unknown crate: $crate" ;;
esac

label="${crate}${variant:+-$variant}"
command -v cargo-tarpaulin >/dev/null 2>&1 \
  || die "cargo-tarpaulin not installed. Install: cargo install --locked cargo-tarpaulin@0.37.5"

cd "$REPO"
set -x
exec cargo tarpaulin --engine llvm -p "$crate" "${features[@]}" \
  --include-files "$files" \
  --timeout "$timeout" --out Stdout --output-dir "coverage/$label" \
  --fail-under "$floor"
