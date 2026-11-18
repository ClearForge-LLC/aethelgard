#!/bin/bash
# AET-WO-1303b measured stop: every board, net and pitch, sequentially
set -u
X=${SUPPLY_STOP:-target/release/examples/supply_stop}
B=${BOARDS:-$HOME/.cache/aet-1303b/boards}
F=crates
O=spike/1303b/b
for h in 62500 31250; do
  $X $B/sentinel-02-power/sentinel-02-power.kicad_pcb $F/aethelgard-ir/tests/fixtures/sentinel-02-power.json VDD_3V3 L2.2 U1 $h 5 > $O/s02-vdd-$h.out 2>&1
  $X $B/s024l/s024l.kicad_pcb $B/s024l.json VDD_3V3 L2.2 U1 $h 5 > $O/s024l-vdd-$h.out 2>&1
  $X $B/s024l/s024l.kicad_pcb $B/s024l.json GND U11.1 U1 $h 5 > $O/s024l-gnd-$h.out 2>&1
  $X $B/sentinel-03/sentinel-03.kicad_pcb $F/aethelgard-cli/tests/fixtures/sentinel-03.json VDD_3V3 L2.2 U1 $h 5 > $O/s03-vdd-$h.out 2>&1
  $X $B/sentinel-03/sentinel-03.kicad_pcb $F/aethelgard-cli/tests/fixtures/sentinel-03.json GND U11.1 U1 $h 5 > $O/s03-gnd-$h.out 2>&1
done
# crop sensitivity at the coarse pitch
$X $B/s024l/s024l.kicad_pcb $B/s024l.json VDD_3V3 L2.2 U1 62500 15 > $O/s024l-vdd-62500-crop15.out 2>&1
$X $B/sentinel-03/sentinel-03.kicad_pcb $F/aethelgard-cli/tests/fixtures/sentinel-03.json GND U11.1 U1 62500 15 > $O/s03-gnd-62500-crop15.out 2>&1
$X $B/sentinel-02-power/sentinel-02-power.kicad_pcb $F/aethelgard-ir/tests/fixtures/sentinel-02-power.json VDD_3V3 L2.2 U1 15625 5 > $O/s02-vdd-15625.out 2>&1
echo done
