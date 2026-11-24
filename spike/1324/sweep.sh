#!/usr/bin/env bash
# AET-WO-1324 §1.1 (2): every committed specification emitted by the shimmed
# binary (inst.py); one log line per route_measured call (AET1324_LOG):
# rung, outline mm, parts, outcome, counted work, scan pairs, seeds, index updates,
# route ms, scan ms, seed ms, judge ms.
# Usage: sweep.sh <shimmed aethelgard> <repo> <out dir>
set -euo pipefail
bin=$1 repo=$2 out=$3
mkdir -p "$out/log"
bash "$repo/spike/1211d/measure/sweep.sh" /bin/true "$repo" "$out/list" > /dev/null || true
cd "$repo"
while read -r f; do
  n=${f//\//__}
  d="$out/emit/$n"
  mkdir -p "$d"
  set +e
  AET1324_LOG="$out/log/$n.tsv" "$bin" emit --file "$f" --out "$d/emit" > "$d/emit.out" 2>&1
  echo $? > "$d/emit.exit"
  set -e
done < "$out/list/specs.txt"
