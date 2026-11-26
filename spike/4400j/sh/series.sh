#!/bin/bash
# AET-WO-4400j (1): the CLI-path series, one container per run (sh/img.sh, 4 cores pinned, 8 GiB cap), rows appended
# to ~/.cache/aet-4400j/cli/rows.jsonl. An input run takes arm12.json cut to its window (py/cut.py), arm12.json at 12 s. Usage: series.sh TAG BOARD "5000ms 8000ms ..." N [input]   (LIFT=1 passes through)
set -u
tag=$1; board=$2; windows=$3; n=$4; inp=${5:-}
K=/home/forge/aethelgard-4400j/spike/4400j/sh
for r in $(seq 1 "$n"); do
  for w in $windows; do
    name=$tag-$board-${w%ms}${inp:+-in}-$r
    arg=""; [ -n "$inp" ] && { arg=/cache/in/arm12-${w%ms}.json; [ -f "$HOME/.cache/aet-4400j/in/arm12-${w%ms}.json" ] || arg=/cache/in/arm12.json; }
    CPUS=${CPUS:-0-3} MEM=${MEM:-8G} "$K/img.sh" tools "LIFT=${LIFT:-0} HOLD=${HOLD:-0} bash /spike/sh/cli.sh $name $w $board $arg" 2>&1 | tail -1 \
      | tee -a ~/.cache/aet-4400j/cli/rows.jsonl
  done
done
