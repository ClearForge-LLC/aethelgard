#!/bin/bash
# AET-WO-1303b: the worst pin per 4-layer board and net at 31.25 and 15.625 um, each run with that pin
# alone (so both pitches share one crop), for the uncertified reference.
set -u
X=${SUPPLY_STOP:-target/release/examples/supply_stop}
B=${BOARDS:-$HOME/.cache/aet-1303b/boards}
F=crates
O=spike/1303b/b
for h in 31250 15625; do
  $X $B/s024l/s024l.kicad_pcb $B/s024l.json VDD_3V3 L2.2 U1 $h 5 17 > $O/ref-s024l-vdd-17-$h.out 2>&1
  $X $B/s024l/s024l.kicad_pcb $B/s024l.json GND U11.1 U1 $h 5 31 > $O/ref-s024l-gnd-31-$h.out 2>&1
  $X $B/sentinel-03/sentinel-03.kicad_pcb $F/aethelgard-cli/tests/fixtures/sentinel-03.json VDD_3V3 L2.2 U1 $h 5 17 > $O/ref-s03-vdd-17-$h.out 2>&1
  $X $B/sentinel-03/sentinel-03.kicad_pcb $F/aethelgard-cli/tests/fixtures/sentinel-03.json GND U11.1 U1 $h 5 31 > $O/ref-s03-gnd-31-$h.out 2>&1
done
echo done
