#!/bin/bash
# Inside the image: run one CLI step, stdout to $OUT/<step>.json, stderr to $OUT/<step>.err; print
# one TSV row: spec, step, exit, wall milliseconds, the scope's memory.peak (bytes), memory.max (bytes).
# A watch reads the scope's memory.peak every second and, past 900 per mille of memory.max (the work
# order's stop line), kills the step's process group; the row's exit is then `killed-900`.
# Usage: step.sh <spec-name> <step-name> <out-dir> <command...>
set -u
SPEC=$1 STEP=$2 OUT=$3; shift 3
mkdir -p "$OUT"
CG=/sys/fs/cgroup
max=$(cat $CG/memory.max 2>/dev/null || echo NA)
s=$(date +%s%N)
setsid "$@" < /dev/null > "$OUT/$STEP.json" 2> "$OUT/$STEP.err" &
pid=$!
over=0
while kill -0 "$pid" 2>/dev/null; do
  peak=$(cat $CG/memory.peak 2>/dev/null || echo 0)
  if [ "$max" != NA ] && [ "$max" != max ] && [ $((peak * 1000)) -gt $((max * 900)) ]; then
    over=1; kill -TERM -- "-$pid" 2>/dev/null; sleep 5; kill -KILL -- "-$pid" 2>/dev/null; break
  fi
  sleep 1
done
wait "$pid"; rc=$?
[ $over = 1 ] && rc=killed-900
e=$(date +%s%N)
peak=$(cat $CG/memory.peak 2>/dev/null || echo NA)
printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$SPEC" "$STEP" "$rc" "$(( (e - s) / 1000000 ))" "$peak" "$max" | tee -a "$OUT/rows.tsv"
