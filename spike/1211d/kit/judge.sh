#!/usr/bin/env bash
# AET-WO-1211d: the CLI's four judges on one drawn board, inside the image (run through img.sh).
# Usage (in the container): judge.sh <dir holding spec.json and <project>.kicad_sch/.kicad_pcb>
set -u
cd "$1"; A=/bin-host/aethelgard
P=$(basename "$(ls *.kicad_sch)" .kicad_sch)
v() { python3 -I -c 'import json,sys; d=json.loads(sys.stdin.read()); print(d.get("verdict", d.get("ok")), json.dumps(d.get("findings", d.get("error", "")))[:900])'; }
echo "erc:     $($A check erc --schematic $P.kicad_sch | v)"
echo "drc:     $($A check drc --board $P.kicad_pcb --spec spec.json --schematic $P.kicad_sch | v)"
echo "support: $($A check support --schematic $P.kicad_sch --board $P.kicad_pcb --spec spec.json | v)"
echo "extract: $($A extract --board $P.kicad_pcb --spec spec.json | v)"
