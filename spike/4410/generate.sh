#!/usr/bin/env bash
# AET-WO-4410 §1.1: solve each committed specification, then generate hwdef; one TSV row each.
# Usage: generate.sh <aethelgard binary> <out dir>
set -uo pipefail
cd "$(git rev-parse --show-toplevel)"
bin=$1; out=$2; mkdir -p "$out"
bash spike/4410/universe.sh | while IFS=$'\t' read -r n f; do
  "$bin" solve --file "$f" > "$out/$n.topology.json" 2> "$out/$n.solve.err"; s=$?
  if [ $s -ne 0 ]; then
    k=$(python3 -I -c 'import json,sys;e=json.load(open(sys.argv[1])).get("error",{});print(e.get("kind"),e.get("id"))' "$out/$n.topology.json" 2>/dev/null || echo "?")
    printf '%s\tsolve\t%s\t%s\n' "$n" "$s" "$k"; continue
  fi
  rm -f "$out/$n.hwdef.dat"
  "$bin" generate hwdef --topology "$out/$n.topology.json" --out "$out/$n.hwdef.dat" > "$out/$n.gen.json" 2>&1; g=$?
  k=$(python3 -I -c 'import json,sys;d=json.load(open(sys.argv[1]));e=d.get("error") or {};print(e.get("kind","ok"),e.get("id",""))' "$out/$n.gen.json" 2>/dev/null || echo "?")
  printf '%s\tgenerate\t%s\t%s\n' "$n" "$g" "$k"
done
