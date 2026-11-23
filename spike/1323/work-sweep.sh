#!/usr/bin/env bash
# AET-WO-1323 §1.1 (1): every committed specification emitted by the shimmed
# binary (inst.py), one work log per specification (one line per `route_with`
# call), and the emitted bytes beside it (they must equal the base's).
# Usage: work-sweep.sh <shimmed aethelgard> <repo> <out dir>
set -euo pipefail
bin=$1 repo=$2 out=$3
mkdir -p "$out/work"
bash "$repo/spike/1211d/measure/sweep.sh" /bin/true "$repo" "$out/list" > /dev/null || true
cd "$repo"
while read -r f; do
  n=${f//\//__}
  d="$out/emit/$n"
  mkdir -p "$d"
  s=$(date +%s.%N)
  set +e
  AET1323_WORK_LOG="$out/work/$n.tsv" "$bin" emit --file "$f" --out "$d/emit" > "$d/emit.out" 2>&1
  echo $? > "$d/emit.exit"
  set -e
  echo "$n $(echo "$(date +%s.%N) - $s" | bc)" >> "$out/emit-seconds.txt"
done < "$out/list/specs.txt"
