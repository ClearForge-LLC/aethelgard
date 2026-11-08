#!/bin/bash
# Usage: determinism.sh <cmdfile>... : each is run 10 times; prints the distinct HASH values with counts,
# the decoded blocks of run 1, and the mean wall time per run (s).
cd "$(dirname "$0")"
for c in "$@"; do
  hs=(); t0=$(date +%s.%N)
  for n in 1 2 3 4 5 6 7 8 9 10; do
    s=$(date +%s.%N)
    out=$(./run-renode.sh "$c" | python3 -I summarize.py)
    e=$(date +%s.%N)
    hs+=("$(echo "$out" | grep '^HASH' | cut -d' ' -f2)")
    ts+=("$(echo "$e - $s" | bc -l)")
    [ $n = 1 ] && first=$(echo "$out" | grep -v "Unhandled write\|Received 1 bytes\|Not enough data" | cut -c1-250)
  done
  echo "=== $c"; echo "$first"
  echo "DISTINCT_HASHES $(printf '%s\n' "${hs[@]}" | sort | uniq -c | tr '\n' ';')"
  echo "WALL_s_per_run $(printf '%s\n' "${ts[@]}" | awk '{s+=$1;if(min==""||$1<min)min=$1;if($1>max)max=$1}END{printf "mean=%.2f min=%.2f max=%.2f",s/NR,min,max}')"
  unset ts hs
done
