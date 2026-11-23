#!/usr/bin/env bash
# AET-WO-1607: compares two sweep.sh output directories byte for byte, after replacing each
# directory's own name in the printed lines (a verdict names the paths it judged).
# Prints each differing file; exits 0 when there is none.
# Usage: same.sh <sweep dir A> <sweep dir B> <scratch dir>
set -euo pipefail
a=$1 b=$2 t=$3
rm -rf "$t"; mkdir -p "$t"
for side in a b; do
  src=${!side}
  cp -r "$src" "$t/$side"
  find "$t/$side" -type f \( -name '*.out' -o -name '*.err' \) -print0 |
    xargs -0 -r sed -i "s#$src/#SWEEP/#g; s#/cache/sw/$(basename "$src")/#SWEEP/#g"
done
diff -rq "$t/a" "$t/b" && echo "identical: $(find "$t/a" -type f | wc -l) files"
