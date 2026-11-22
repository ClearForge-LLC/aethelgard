#!/usr/bin/env bash
# AET-WO-4300h: validate, solve and emit every committed board (the universe is
# spike/1216/universe.sh, the one home of board names), then both walks.
# usage: run.sh <aethelgard binary> <scratch dir>
#   writes out/boards.tsv, out/facing/<board>.tsv (walk 1), out/inward/<board>.tsv (walk 2)
set -uo pipefail
AE=$1; W=$2
here=$(cd "$(dirname "$0")" && pwd); root=$(git -C "$here" rev-parse --show-toplevel)
pp="$root/crates/aethelgard-ir/data/power-parts.json"
vis="$root/spike/1211e/out/table8-visual.tsv"
mkdir -p "$here/out/facing" "$here/out/inward" "$W/solve" "$W/emit"
printf 'name\tfile\tvalidate\tsolve\temit\tnote\n' > "$here/out/boards.tsv"
bash "$root/spike/1216/universe.sh" | while IFS=$'\t' read -r n f; do
  v=$("$AE" validate --file "$root/$f" 2>&1); vrc=$?
  s=$("$AE" solve --file "$root/$f" 2>/dev/null); src=$?
  sj=-; [ $src -eq 0 ] && { printf '%s' "$s" > "$W/solve/$n.json"; sj="$W/solve/$n.json"; }
  kind() { python3 -I -c 'import json,sys; print(json.load(sys.stdin).get("error",{}).get("kind","?"))' 2>/dev/null; }
  note=""; [ $vrc -ne 0 ] && note=$(printf '%s' "$v" | kind)
  erc=skip
  if [ $src -eq 0 ]; then
    rm -rf "$W/emit/$n"
    e=$(cd "$W/emit" && nice -n 10 "$AE" emit --file "$root/$f" --out "$n" 2>&1); erc=$?
    if [ $erc -eq 0 ]; then
      python3 -I "$here/inward.py" "$W/emit/$n"/*.kicad_pcb "$vis" > "$here/out/inward/$n.tsv"
    else
      note="$note emit:$(printf '%s' "$e" | kind)"
    fi
  fi
  python3 -I "$here/facing.py" "$root/$f" "$sj" "$pp" "$root/spike/4300h/sidecars/undeclared-offboard.tsv" "$n" \
    > "$here/out/facing/$n.tsv" || echo "facing FAILED $n"
  printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$n" "$f" "$vrc" "$src" "$erc" "$note" >> "$here/out/boards.tsv"
done
