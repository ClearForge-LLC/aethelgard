#!/usr/bin/env bash
# AET-WO-1916: a run fetched by spike/1912/fetch.sh (`gh run view --log`, job and step columns first) made into
# spike/1914/held.sh's input, then each shard's test times by ci/shard.sh times (P3), 4 threads (the runner's nproc).
#   stamp.sh RUNDIR   ->  RUNDIR/raw-K.txt, RUNDIR/st-K.txt, RUNDIR/times.tsv (shard<TAB>binary<TAB>test<TAB>seconds)
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
d=$1
: >"$d/times.tsv"
for f in "$d"/shard-*.log; do
  k=${f##*/shard-}; k=${k%.log}
  cut -f3- "$f" >"$d/raw-$k.txt"
  bash "$here/../1914/held.sh" stamp "$d/raw-$k.txt" >"$d/st-$k.txt"
  bash "$here/../../ci/shard.sh" times "$d/st-$k.txt" 4 | sed "s|^|$k\t|" >>"$d/times.tsv"
done
