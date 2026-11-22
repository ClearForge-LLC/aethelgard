#!/usr/bin/env bash
# AET-WO-1319: two run.sh output directories, byte for byte. The sweep is
# judged by spike/1211d/measure/same.sh (each command's printed JSON with its
# own output directory normalised); each harness by `diff -r` (its files name
# no output path). Prints each verdict; exits 1 on any difference.
# Usage: compare.sh <out a> <out b> <repo>
set -uo pipefail
a=$1 b=$2 repo=$3 bad=0
bash "$repo/spike/1211d/measure/same.sh" "$a/sweep" "$b/sweep" || bad=1
for t in d4diff d1319; do
  n=$(find "$a/$t" -type f | wc -l)
  cases=$(find "$a/$t" -mindepth 1 -maxdepth 1 -type d | wc -l)
  if diff -r "$a/$t" "$b/$t" > /dev/null; then
    echo "$t identical: $cases cases, $n files"
  else
    echo "$t DIFFERS: $(diff -rq "$a/$t" "$b/$t" | wc -l) entries, $(diff -rq "$a/$t" "$b/$t" | sed -n 's#.*/'"$t"'/\([^/]*\)/.*#\1#p' | sort -u | wc -l) cases"
    bad=1
  fi
done
exit "$bad"
