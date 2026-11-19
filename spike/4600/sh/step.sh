#!/bin/bash
# Inside the image: run one CLI step, stdout to $OUT/<step>.json, stderr to $OUT/<step>.err; print
# one TSV row: spec, step, exit, wall seconds, the scope's memory.peak (bytes), memory.max (bytes).
# Usage: step.sh <spec-name> <step-name> <out-dir> <command...>   (command may be "-stdin <file> ...")
set -u
SPEC=$1 STEP=$2 OUT=$3; shift 3
mkdir -p "$OUT"
IN=/dev/null
if [ "$1" = -stdin ]; then IN=$2; shift 2; fi
s=$(date +%s%N)
"$@" < "$IN" > "$OUT/$STEP.json" 2> "$OUT/$STEP.err"
rc=$?
e=$(date +%s%N)
peak=$(cat /sys/fs/cgroup/memory.peak 2>/dev/null || echo NA)
max=$(cat /sys/fs/cgroup/memory.max 2>/dev/null || echo NA)
printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$SPEC" "$STEP" "$rc" "$(( (e - s) / 1000000 ))" "$peak" "$max" | tee -a "$OUT/rows.tsv"
