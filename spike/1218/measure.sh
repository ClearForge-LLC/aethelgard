#!/usr/bin/env bash
# AET-WO-1218 §1.1. Usage: measure.sh <aethelgard binary> <repository root> <out dir>
# 1. the baseline: spike/1211d/measure/sweep.sh over every committed specification;
# 2. the scratch boards (probes.py): validate, solve, emit twice; kind@path per stage, the two emits
#    compared byte for byte, and the emitted references (refs.py).
set -euo pipefail
bin=$1 repo=$2 out=$3
here=$(cd "$(dirname "$0")" && pwd)
mkdir -p "$out"
kind() { python3 -I -c 'import json,sys
try: d=json.loads(sys.argv[1].splitlines()[-1])
except (ValueError, IndexError): print("PARSE"); sys.exit()
e=d.get("error"); print("ok" if e is None else e["kind"]+"@"+str(e.get("path"))+" "+e.get("message","")[:120])' "$1"; }
bash "$repo/spike/1211d/measure/sweep.sh" "$bin" "$repo" "$out/sweep" > /dev/null
for d in "$out"/sweep/*/; do n=$(basename "$d"); printf "%s\t%s\t%s\t%s\n" "$n" "$(cat "$d/validate.exit")" "$(cat "$d/solve.exit")" "$(cat "$d/emit.exit")"; done > "$out/baseline-exits.tsv"
mkdir -p "$out/probes"
python3 -I "$here/probes.py" "$repo" "$out/probes" > /dev/null
for f in "$out"/probes/*.json; do
  n=$(basename "$f" .json)
  v=$(kind "$("$bin" validate --file "$f" 2>&1)")
  s=$(kind "$("$bin" solve --file "$f" 2>&1)")
  rm -rf "$out/emit/$n" "$out/emit2/$n"
  e=$(kind "$("$bin" emit --file "$f" --out "$out/emit/$n" 2>&1)")
  "$bin" emit --file "$f" --out "$out/emit2/$n" > /dev/null 2>&1 || true
  same=$(diff -r -q "$out/emit/$n" "$out/emit2/$n" > /dev/null 2>&1 && echo same || echo DIFFER)
  [ -d "$out/emit/$n" ] || same=-
  printf "%s\tvalidate=%s\tsolve=%s\temit=%s\ttwice=%s\n" "$n" "$v" "$s" "$e" "$same"
done > "$out/probes.tsv"
python3 -I "$here/refs.py" "$out/emit" > "$out/refs.tsv"
