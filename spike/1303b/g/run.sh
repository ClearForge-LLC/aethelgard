#!/bin/bash
# AET-WO-1303b, after the stop's ruling: every board and net at the gate's
# pitch (62.5 um), the hole anywhere within +-75 um per axis (S1)
set -u
X=${SUPPLY_STOP:-target/release/examples/supply_stop}
B=${BOARDS:-$HOME/.cache/aet-1303b/boards}
F=crates
O=spike/1303b/g
$X $B/sentinel-02-power/sentinel-02-power.kicad_pcb $F/aethelgard-ir/tests/fixtures/sentinel-02-power.json VDD_3V3 L2.2 U1 62500 5 > $O/s02-vdd.out 2>&1
$X $B/s024l/s024l.kicad_pcb $B/s024l.json VDD_3V3 L2.2 U1 62500 5 > $O/s024l-vdd.out 2>&1
$X $B/s024l/s024l.kicad_pcb $B/s024l.json GND U11.1 U1 62500 5 > $O/s024l-gnd.out 2>&1
$X $B/sentinel-03/sentinel-03.kicad_pcb $F/aethelgard-cli/tests/fixtures/sentinel-03.json VDD_3V3 L2.2 U1 62500 5 > $O/s03-vdd.out 2>&1
$X $B/sentinel-03/sentinel-03.kicad_pcb $F/aethelgard-cli/tests/fixtures/sentinel-03.json GND U11.1 U1 62500 5 > $O/s03-gnd.out 2>&1
echo done
