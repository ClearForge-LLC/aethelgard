#!/bin/bash
# AET-WO-1314b §1.1: validate, emit and extract one variant, each step under its own capped scope
# (MemoryMax=8G, no swap), nice -n 10, with /usr/bin/time -v for wall time and peak RSS.
# Usage: spike/1314b/run.sh <variant> <aethelgard binary>   (variant: logger-spi-only-cap[-tol])
# Outputs: ~/.cache/aet-1314b/runs/<variant>/<step>.{json,time}, and a row per step on rows.tsv
# (variant, step, exit, wall s, peak RSS kB).
set -u
cd "$(dirname "$0")/../.."
V=$1; A=$2
SPEC=spike/1314b/$V.json
O=$HOME/.cache/aet-1314b/runs/$V
rm -rf "$O"; mkdir -p "$O"
export XDG_RUNTIME_DIR=/run/user/$(id -u)
step() { local name=$1; shift
  systemd-run --user --scope --quiet -p MemoryMax=8G -p MemorySwapMax=0 -- \
    nice -n 10 /usr/bin/time -v -o "$O/$name.time" "$A" "$@" > "$O/$name.json"
  local rc=$?
  local wall peak
  wall=$(awk -F': ' '/Elapsed \(wall clock\)/ {print $2}' "$O/$name.time")
  peak=$(awk -F': ' '/Maximum resident set size/ {print $2}' "$O/$name.time")
  printf '%s\t%s\t%s\t%s\t%s\n' "$V" "$name" "$rc" "$wall" "$peak" | tee -a "$O/rows.tsv"
}
step validate validate --file "$SPEC"
step emit emit --file "$SPEC" --out "$O/board"
PCB=$(python3 -I -c 'import json,sys; print(json.load(open(sys.argv[1]))["board"]["path"])' "$O/emit.json" 2>/dev/null)
if [ -n "$PCB" ] && [ -f "$PCB" ]; then
  step extract extract --board "$PCB" --spec "$SPEC"
else
  printf '%s\textract\tnot_run\t-\t-\temit wrote no board\n' "$V" | tee -a "$O/rows.tsv"
fi
