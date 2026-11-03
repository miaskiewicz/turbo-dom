#!/usr/bin/env bash
# Fail if any given ELF binary requires a GLIBC_ symbol version above the ceiling
# (default 2.35 = ubuntu-22.04, the oldest supported consumer glibc).
#
# Every argument must be an existing dynamically linked linux-gnu binary. The guard never
# passes vacuously: a missing file (or a glob that matched nothing and reached us
# literally), a readelf failure, or a binary with no GLIBC_ symbol versions at all is an
# error. readelf (not objdump) reads any ELF machine, so the x64 publish runner can check
# the arm64 addon (host objdump -T says "file format not recognized" there).
# Self-test: scripts/test_check_glibc_ceiling.py.
set -euo pipefail
ceiling="${GLIBC_CEILING:-2.35}"
[ "$#" -gt 0 ] || { echo "usage: $0 <binary>..." >&2; exit 2; }
status=0
for bin in "$@"; do
  if [ ! -f "$bin" ]; then
    echo "::error::$bin: no such file (expected a linux-gnu addon)"
    status=1
    continue
  fi
  if ! symbols=$(readelf -W --dyn-syms -V "$bin"); then
    echo "::error::$bin: readelf failed"
    status=1
    continue
  fi
  max=$(printf '%s\n' "$symbols" | { grep -o 'GLIBC_[0-9.]*' || true; } | sed 's/GLIBC_//' | sort -uV | tail -1)
  if [ -z "$max" ]; then
    echo "::error::$bin: no GLIBC_ symbol versions (not a dynamically linked linux-gnu binary?)"
    status=1
    continue
  fi
  echo "$bin: highest required GLIBC_$max"
  if [ "$(printf '%s\n%s\n' "$max" "$ceiling" | sort -V | tail -1)" != "$ceiling" ]; then
    echo "::error::$bin requires GLIBC_$max (> GLIBC_$ceiling)"
    status=1
  fi
done
exit "$status"
