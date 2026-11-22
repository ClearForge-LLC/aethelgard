#!/usr/bin/env bash
# AET-WO-1321 §1.3, the fold's bytes: the sweep's only specification with a
# declared `bulk` (d1-drv8876.json) declares `board.drawn: false`, so `emit`
# refuses it (DrawsNoBoard) and the sweep never draws `declared_bulk`. This
# emits it on a drawn board, as pin_devices_drawn_tier1.rs's d1_drawn() makes
# it (the `drawn` key removed), with its ceramic bulk and with the electrolytic
# that test declares, using two binaries, and judges the two output trees by
# `diff -r`. Prints the verdict; exits 1 on any difference.
# Usage: bulk.sh <base aethelgard> <branch aethelgard> <repo> <out dir>
set -uo pipefail
a=$1 b=$2 repo=$3 out=$4
mkdir -p "$out/in"
python3 -I - "$repo/crates/aethelgard-ir/tests/fixtures/d1-drv8876.json" "$out/in" <<'PY'
import json, sys
src, out = sys.argv[1], sys.argv[2]
for name, part in (("ceramic", None), ("electrolytic", "UCZ1H470MCL1GS")):
    v = json.load(open(src))
    del v["board"]["drawn"]
    if part:
        v["pin_devices"][0]["bulk"]["part"] = part
    json.dump(v, open(f"{out}/{name}.json", "w"), indent=2)
PY
bad=0
for c in ceramic electrolytic; do
  for side in a b; do
    bin=${!side}
    d="$out/$side/$c"; mkdir -p "$d"
    "$bin" emit --file "$out/in/$c.json" --out "$d/emit" > "$d/emit.out" 2>&1; echo $? > "$d/emit.exit"
    sed -i "s#$out/$side/$c/emit#OUT#g" "$d/emit.out"
  done
  note=$(grep -rl 'CVM2 declared, TI SLVSDS7B DRV8876 states no value' "$out/a/$c/emit" | wc -l)
  if diff -r "$out/a/$c" "$out/b/$c" > /dev/null; then
    echo "$c identical: emit exit $(cat "$out/a/$c/emit.exit"), $(find "$out/a/$c/emit" -type f | wc -l) files, the note in $note"
  else
    echo "$c DIFFERS: $(diff -rq "$out/a/$c" "$out/b/$c" | wc -l) entries"; bad=1
  fi
done
exit "$bad"
