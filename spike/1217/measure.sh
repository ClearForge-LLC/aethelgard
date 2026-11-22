#!/usr/bin/env bash
# AET-WO-1217 §1.1. Usage: measure.sh <aethelgard binary> <repository root> <out dir>
# 1. the baseline: spike/1211d/measure/sweep.sh over every committed specification;
# 2. F4: every solved pin on PC2/PC3 (the PC2_C/PC3_C pads), by pc23.py;
# 3. F2, F9, F10: each probe's validate and solve refusal, kind@path;
# 4. F10's scope: every committed specification with `mcu.part` set to a part with no power-parts row.
set -euo pipefail
bin=$1 repo=$2 out=$3
here=$(cd "$(dirname "$0")" && pwd)
mkdir -p "$out"
kind() { python3 -I -c 'import json,sys
try: d=json.loads(sys.argv[1])
except ValueError: print("PARSE"); sys.exit()
e=d.get("error"); print("ok" if e is None else e["kind"]+"@"+str(e.get("path")))' "$1"; }
bash "$repo/spike/1211d/measure/sweep.sh" "$bin" "$repo" "$out/sweep" > /dev/null
for d in "$out"/sweep/*/; do n=$(basename "$d"); printf "%s\t%s\t%s\t%s\n" "$n" "$(cat "$d/validate.exit")" "$(cat "$d/solve.exit")" "$(cat "$d/emit.exit")"; done > "$out/baseline-exits.tsv"
python3 -I "$here/pc23.py" "$out/sweep" > "$out/pc2-pc3.tsv"
mkdir -p "$out/probes"
python3 -I "$here/probes.py" "$repo" "$out/probes" > /dev/null
for f in "$out"/probes/*.json; do
  printf "%s\t%s\t%s\n" "$(basename "$f" .json)" "$(kind "$("$bin" validate --file "$f" 2>&1)")" "$(kind "$("$bin" solve --file "$f" 2>&1)")"
done > "$out/probes.tsv"
mkdir -p "$out/f10"
while read -r f; do
  o="$out/f10/$(echo "$f" | tr / _)"
  python3 -I -c 'import json,sys;s=json.load(open(sys.argv[1]));s["mcu"]["part"]="STM32H743ZITx";json.dump(s,open(sys.argv[2],"w"))' "$repo/$f" "$o"
  printf "%s\t%s\n" "$f" "$(kind "$("$bin" validate --file "$o" 2>&1)")"
done < "$out/sweep/specs.txt" > "$out/f10-scope.tsv"
