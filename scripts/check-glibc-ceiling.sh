#!/usr/bin/env bash
# Fail if any given ELF binary requires a GLIBC_ symbol version above the ceiling
# (default 2.35 = ubuntu-22.04, the oldest supported consumer glibc).
set -euo pipefail
ceiling="${GLIBC_CEILING:-2.35}"
[ "$#" -gt 0 ] || { echo "usage: $0 <binary>..." >&2; exit 2; }
status=0
for bin in "$@"; do
  max=$(objdump -T "$bin" | grep -o 'GLIBC_[0-9.]*' | sed 's/GLIBC_//' | sort -uV | tail -1 || true)
  echo "$bin: highest required GLIBC_${max:-none}"
  if [ -n "$max" ] && [ "$(printf '%s\n%s\n' "$max" "$ceiling" | sort -V | tail -1)" != "$ceiling" ]; then
    echo "::error::$bin requires GLIBC_$max (> GLIBC_$ceiling)"
    status=1
  fi
done
exit "$status"
