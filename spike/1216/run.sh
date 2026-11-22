#!/usr/bin/env bash
# AET-WO-1216: validate, solve, emit and walk the drawn netlist of every committed board.
# usage: run.sh <aethelgard binary> <scratch dir>   (writes out/boards.tsv and out/drawn/*.tsv)
set -uo pipefail
AE=$1; W=$2
here=$(cd "$(dirname "$0")" && pwd); root=$(git -C "$here" rev-parse --show-toplevel)
mkdir -p "$here/out/drawn" "$W/solve" "$W/emit"
printf 'file\tvalidate\tsolve\temit\tdrawn_rows\tnote\n' > "$here/out/boards.tsv"
bash "$here/universe.sh" | while read -r f; do
  n=$(basename "$f" .json); d=$(basename "$(dirname "$f")"); [ "$d" = solve ] && n="solve-$n"
  v=$("$AE" validate --file "$root/$f" 2>&1); vrc=$?
  s=$("$AE" solve --file "$root/$f" 2>/dev/null); src=$?; [ $src -eq 0 ] && printf '%s' "$s" > "$W/solve/$n.json"
  note=""
  [ $vrc -ne 0 ] && note=$(printf '%s' "$v" | python3 -I -c 'import json,sys; e=json.load(sys.stdin).get("error",{}); print(e.get("kind","?"))' 2>/dev/null)
  erc=skip; rows=-
  if [ $src -eq 0 ]; then
    rm -rf "$W/emit/$n"
    e=$(cd "$W/emit" && nice -n 10 "$AE" emit --file "$root/$f" --out "$n" 2>&1); erc=$?
    if [ $erc -eq 0 ]; then
      sup=$(python3 -I -c 'import json,sys; print(json.load(open(sys.argv[1]))["mcu"]["supply"])' "$root/$f")
      pcb=$(ls "$W/emit/$n"/*.kicad_pcb)
      python3 -I "$here/drawn.py" "$pcb" "$root/spike/1211e/out/table8-visual.tsv" "$sup" > "$here/out/drawn/$n.tsv"
      rows=$(wc -l < "$here/out/drawn/$n.tsv")
    else
      note="$note emit:$(printf '%s' "$e" | python3 -I -c 'import json,sys; e=json.load(sys.stdin).get("error",{}); print(e.get("kind","?"))' 2>/dev/null)"
    fi
  fi
  printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$f" "$vrc" "$src" "$erc" "$rows" "$note" >> "$here/out/boards.tsv"
done
