#!/bin/bash
# AET-WO-4300f: the KiCad half of S2 and S4, UNRUN. Every step of it waits on D8 of PR #131 (D10
# here): spike/4300f/img.sh releases nothing while the container's memory.swap.max is not 0.
# Run from the worktree root: bash spike/4300f/img.sh -- bash spike/4300f/gates.sh
# Inside the image, /cache is ~/.cache/aet-4300f: the host-built CLI (/cache/target/debug/aethelgard,
# which runs in the image per AET-WO-1301), the S2 emission (/cache/s2-out) and the S4 boards
# (/cache/s4-out). Each gate's JSON report and exit code go to /cache/gates/.
set -u
A=/cache/target/debug/aethelgard
O=/cache/gates
mkdir -p "$O"
run() { local name="$1"; shift; "$A" "$@" > "$O/$name.json"; echo "$name exit $?" | tee -a "$O/exits.txt"; }
# S2: a device-less SPI bus, three one-pin nets.
run s2-erc check erc --schematic /cache/s2-out/sentinel-01.kicad_sch
run s2-drc check drc --board /cache/s2-out/sentinel-01.kicad_pcb --spec spike/4300f/s2/offboard.json \
  --schematic /cache/s2-out/sentinel-01.kicad_sch
# S4: the reference with J1 hand-placed at the bottom edge. No schematic carries J1, so parity is
# not judged (it would name J1 as missing, a hand-placement artifact).
run s4-drc-A check drc --board /cache/s4-out/sentinel-01-J1-A.kicad_pcb --spec examples/sentinel-01-minimal.json
run s4-drc-B check drc --board /cache/s4-out/sentinel-01-J1-B.kicad_pcb --spec examples/sentinel-01-minimal.json
