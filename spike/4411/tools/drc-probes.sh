#!/bin/bash
# Runs inside the tools image: raw kicad-cli DRC (refill, JSON) and aethelgard check drc on each probe board.
set -u
mkdir -p /cache/probe3/r
for b in /cache/probe3/b/*.kicad_pcb; do
  n=$(basename "$b" .kicad_pcb); w=/cache/probe3/w/$n; rm -rf "$w"; mkdir -p "$w"; cp "$b" "$w/"
  kicad-cli pcb drc --refill-zones --save-board --format json --severity-all --exit-code-violations -o /cache/probe3/r/$n.kicad.json "$w/$n.kicad_pcb" > /cache/probe3/r/$n.kicad.log 2>&1; echo "$n kicad rc=$?"
  cp "$w/$n.kicad_pcb" /cache/probe3/r/$n.filled.kicad_pcb
  /cache/target/release/aethelgard check drc --board "$b" > /cache/probe3/r/$n.ae.json 2>&1; echo "$n ae rc=$?"
done
