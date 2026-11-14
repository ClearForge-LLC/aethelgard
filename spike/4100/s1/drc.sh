#!/bin/bash
# S1 in the image: the gate's own DRC (check drc --spec) on each board, then KiCad directly
# (pcb upgrade; pcb drc --refill-zones on the gate-prepared copy) so a gate refusal does not hide KiCad's answer.
# Runs as: img.sh bash /repo/spike/4100/s1/drc.sh   (writes /work/s1/out)
set -u
O=/work/s1/out; mkdir -p $O
SPEC=/repo/examples/sentinel-01-minimal.json
kicad-cli version > $O/kicad-version.txt
for b in /work/s1/ref/sentinel-01.kicad_pcb /work/s1/v/*.kicad_pcb; do
  n=$(basename $b .kicad_pcb)
  /work/aethelgard check drc --board $b --spec $SPEC > $O/gate-$n.json 2> $O/gate-$n.err; echo "gate $n exit $?" >> $O/summary.txt
  d=$O/raw-$n; mkdir -p $d; cp $b $d/input.kicad_pcb
  kicad-cli pcb upgrade $d/input.kicad_pcb > $d/upgrade.log 2>&1; echo "upgrade $n exit $?" >> $O/summary.txt
  cp $b $d/drc.kicad_pcb
  kicad-cli pcb drc --refill-zones --exit-code-violations --format json --severity-all -o $d/drc.json $d/drc.kicad_pcb > $d/drc.log 2>&1
  echo "rawdrc $n exit $?" >> $O/summary.txt
done
