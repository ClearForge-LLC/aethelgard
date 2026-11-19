#!/bin/bash
# Inside the image: run one CLI step, stdout to $OUT/<step>.json, stderr to $OUT/<step>.err; print
# one TSV row: spec, step, exit, wall milliseconds, the scope's memory.peak (bytes), memory.max (bytes).
# Usage: step.sh <spec-name> <step-name> <out-dir> <command...>
set -u
SPEC=$1 STEP=$2 OUT=$3; shift 3
mkdir -p "$OUT"
s=$(date +%s%N)
"$@" < /dev/null > "$OUT/$STEP.json" 2> "$OUT/$STEP.err"
rc=$?
e=$(date +%s%N)
peak=$(cat /sys/fs/cgroup/memory.peak 2>/dev/null || echo NA)
max=$(cat /sys/fs/cgroup/memory.max 2>/dev/null || echo NA)
printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$SPEC" "$STEP" "$rc" "$(( (e - s) / 1000000 ))" "$peak" "$max" | tee -a "$OUT/rows.tsv"
