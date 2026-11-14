#!/bin/bash
# S1 round 2 in the image: what KiCad writes when it saves 4/6-layer boards (upgrade --force),
# the inner-layer short through the gate, the stackup's effect on raw DRC and Gerbers, and
# Gerbers of every copper layer exported 5 times across wall-clock seconds (--check-zones).
set -u
O=/work/s1/out2; mkdir -p $O
SPEC=/repo/examples/sentinel-01-minimal.json
for n in b4_short b4_stackup; do
  /work/aethelgard check drc --board /work/s1/v/$n.kicad_pcb --spec $SPEC > $O/gate-$n.json 2>&1; echo "gate $n exit $?" >> $O/summary.txt
done
for n in b4 b6 b4_stackup; do
  mkdir -p $O/save-$n; cp /work/s1/v/$n.kicad_pcb $O/save-$n/board.kicad_pcb
  kicad-cli pcb upgrade --force $O/save-$n/board.kicad_pcb > $O/save-$n/log 2>&1; echo "save $n exit $?" >> $O/summary.txt
done
for n in b4 b4_stackup; do
  d=$O/drc-$n; mkdir -p $d; cp /work/s1/v/$n.kicad_pcb $d/board.kicad_pcb
  kicad-cli pcb drc --refill-zones --exit-code-violations --format json --severity-all -o $d/drc.json $d/board.kicad_pcb > $d/log 2>&1; echo "rawdrc $n exit $?" >> $O/summary.txt
done
CU4=F.Cu,In1.Cu,In2.Cu,B.Cu,Edge.Cuts
CU6=F.Cu,In1.Cu,In2.Cu,In3.Cu,In4.Cu,B.Cu,Edge.Cuts
for n in b4 b6 b4_stackup; do
  L=$CU4; [ $n = b6 ] && L=$CU6
  for k in 1 2 3 4 5; do
    d=$O/gbr-$n-$k; mkdir -p $d; cp /work/s1/v/$n.kicad_pcb $d/input.kicad_pcb
    kicad-cli pcb export gerbers --layers $L --check-zones -o $d/ $d/input.kicad_pcb > $d/log 2>&1; echo "gbr $n $k exit $? $(date +%T)" >> $O/summary.txt
    sleep 1.3
  done
done
# Everything KiCad writes by default (is a job file among it?)
d=$O/gbr-all; mkdir -p $d; cp /work/s1/v/b4.kicad_pcb $d/input.kicad_pcb
kicad-cli pcb export gerbers --check-zones -o $d/ $d/input.kicad_pcb > $d/log 2>&1; echo "gbr-all exit $?" >> $O/summary.txt
sleep 1.3
d=$O/gbr-all2; mkdir -p $d; cp /work/s1/v/b4.kicad_pcb $d/input.kicad_pcb
kicad-cli pcb export gerbers --check-zones -o $d/ $d/input.kicad_pcb > $d/log 2>&1; echo "gbr-all2 exit $?" >> $O/summary.txt
