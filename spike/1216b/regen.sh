#!/usr/bin/env bash
# AET-WO-1216b: regenerate the committed board crates the specification bump
# moves, by `generate` (never by hand), and show each moved line.
# usage: regen.sh <aethelgard binary> <scratch dir>   (run from the repository root)
set -euo pipefail
AE=$1; W=$2
rm -rf "$W/regen"; mkdir -p "$W/regen"
for pair in "examples/sentinel-01-minimal.json:boards/sentinel-01" "examples/sentinel-01.json:boards/sentinel-01-bench"; do
  spec=${pair%%:*}; dir=${pair##*:}; n=$(basename "$dir")
  "$AE" generate --spec "$spec" --out "$W/regen/$n" > "$W/regen/$n.json"
  # The board crate is generate's `board/`; the committed tree has no `kicad/`.
  diff -r "$dir" "$W/regen/$n/board" | grep -E '^[<>]' | sort | uniq -c | sort -rn | head -6 || true
  rsync -a --checksum "$W/regen/$n/board/" "$dir/"
  git status --short -- "$dir" | wc -l
done
