#!/bin/bash
# AET-WO-1307 S2/S3 runner: one binary over the five inputs, one TSV row per input.
# Usage: run.sh <label> <aethelgard binary> <out dir>     (host; run inside a capped user scope)
# Columns: label input exit routed seconds vias track_mm ripups board_sha256 same_as_base bounds_fF
# `same_as_base` compares the board with $BASE_OUT/<input>/*.kicad_pcb when BASE_OUT is set.
# `bounds_fF` is `aethelgard extract`'s c_bound_femtofarads per line, or its error kind.
set -u
LABEL=$1; BIN=$2; OUT=$3
REPO=$(cd "$(dirname "$0")/../.." && pwd)
FIX=$REPO/crates/aethelgard-cli/tests/fixtures
declare -A IN=(
  [ref]=$REPO/examples/sentinel-01-minimal.json
  [2x5]=$HOME/.cache/aet-1307/specs/2x5.json
  [four]=$FIX/four-layer.json
  [D1]=$FIX/d1-24-i2c.json
  [D3]=$FIX/d3-24-i2c-12-spi.json
)
mkdir -p "$OUT"
for k in ref 2x5 four D1 D3; do
  o=$OUT/$k; rm -rf "$o"; mkdir -p "$o"
  /usr/bin/time -f "%e" -o "$o/seconds" "$BIN" emit --file "${IN[$k]}" --out "$o" > "$o/verdict.json" 2> "$o/stderr"
  s=$(tail -1 "$o/seconds")
  code=$(python3 -I -c "import json,sys; v=json.load(open(sys.argv[1])); print(v.get('ok'), v.get('error',{}).get('id',''), v.get('error',{}).get('kind',''))" "$o/verdict.json" 2>/dev/null)
  b=$(ls "$o"/*.kicad_pcb 2>/dev/null | head -1)
  if [ -n "$b" ]; then
    read vias mm < <(python3 -I "$REPO/spike/1307/stats.py" "$b")
    sha=$(sha256sum "$b" | cut -c1-12)
    same=-
    if [ -n "${BASE_OUT:-}" ] && [ -f "$BASE_OUT/$k/$(basename "$b")" ]; then
      cmp -s "$b" "$BASE_OUT/$k/$(basename "$b")" && same=identical || same=differs
    fi
    bounds=$("$BIN" extract --board "$b" --spec "${IN[$k]}" | python3 -I -c "import json,sys; v=json.load(sys.stdin); print(v.get('verdict','')+':'+','.join(str(l.get('c_bound_femtofarads')) for l in v.get('lines',[])) if 'lines' in v else v.get('error',{}).get('kind'))" 2>/dev/null)
  else vias=-; mm=-; sha=-; same=-; bounds=-; fi
  rips=$(grep -o 'RIPUPS_TOTAL [0-9]*' "$o/stderr" | tail -1 | cut -d' ' -f2)
  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$LABEL" "$k" "$code" "$s" "$vias" "$mm" "${rips:--}" "$sha" "$same" "$bounds"
done
