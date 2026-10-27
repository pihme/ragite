#!/usr/bin/env bash
# Fail if a Linux shared library needs a newer glibc than the baseline, or if
# its RUNPATH/RPATH points into the Nix store or the CI checkout.
#   check-glibc-baseline.sh <lib.so> [baseline, default 2.34]
set -euo pipefail
so="${1:?usage: check-glibc-baseline.sh <lib.so> [baseline]}"
baseline="${2:-2.34}"
status=0

max=$(objdump -T "$so" | grep -oE 'GLIBC_[0-9]+(\.[0-9]+)+' | sed 's/^GLIBC_//' | sort -Vu | tail -n 1)
if [[ -z "$max" ]]; then
  echo "no GLIBC symbol versions found in $so" >&2
  exit 1
fi
newest=$(printf '%s\n%s\n' "$max" "$baseline" | sort -V | tail -n 1)
if [[ "$newest" != "$baseline" ]]; then
  echo "$so needs GLIBC_$max, newer than the baseline GLIBC_$baseline:" >&2
  objdump -T "$so" | grep -E 'GLIBC_[0-9]' | awk -v b="$baseline" '{
    match($0, /GLIBC_[0-9.]+/); v = substr($0, RSTART + 6, RLENGTH - 6)
    split(v, x, "."); split(b, y, ".")
    if (x[1] > y[1] || (x[1] == y[1] && x[2] > y[2])) print "  " $NF " GLIBC_" v
  }' >&2
  status=1
else
  echo "$so: newest glibc symbol GLIBC_$max (baseline GLIBC_$baseline)"
fi

paths=$(readelf -d "$so" | grep -E '\((RUNPATH|RPATH)\)' || true)
if [[ -n "$paths" ]] && grep -qE '/nix/store|/home/runner|/github/workspace' <<<"$paths"; then
  echo "$so has a build-machine RUNPATH/RPATH:" >&2
  echo "$paths" >&2
  status=1
fi
exit "$status"
