#!/usr/bin/env bash
# AET-WO-1216: walk 1 (spec.py) over every committed board; solve output from run.sh's scratch.
# usage: walk.sh <scratch dir>   -> out/spec/<name>.tsv
set -uo pipefail
W=$1; here=$(cd "$(dirname "$0")" && pwd); root=$(git -C "$here" rev-parse --show-toplevel)
mkdir -p "$here/out/spec"
bash "$here/universe.sh" | while read -r f; do
  n=$(basename "$f" .json); d=$(basename "$(dirname "$f")"); [ "$d" = solve ] && n="solve-$n"
  sj="$W/solve/$n.json"; [ -f "$sj" ] || sj=-
  python3 -I "$here/spec.py" "$root/$f" "$sj" "$root/crates/aethelgard-ir/data/power-parts.json" \
    "$root/crates/aethelgard-ir/data/parts.json" > "$here/out/spec/$n.tsv" || echo "FAILED $f"
done
