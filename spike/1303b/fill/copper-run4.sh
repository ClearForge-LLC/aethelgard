#!/bin/bash
set -u
cd $HOME/.cache/aet-1303b/cu
X=target/release/examples/supply_stop; B=$HOME/.cache/aet-1303b/boards; F=crates
C=spike/1303b/fill/check_copper.py; G=$HOME/.cache/aet-1303b/fill
for h in 62500 31250; do
 for bd in "s024l $B/s024l.json" "sentinel-03 $F/aethelgard-cli/tests/fixtures/sentinel-03.json"; do
  set -- $bd
  for n in "VDD_3V3 L2.2" "GND U11.1"; do
   set -- $1 $2 $n
   AET_DUMP_CELLS=$1-$3-$h.dump $X $B/$1/$1.kicad_pcb $2 $3 $4 U1 $h 5 >/dev/null
   echo "== $1 $3 $h"
   ~/.local/bin/uv run -q --with shapely --with numpy python -I $C $1-$3-$h.dump $B/$1/$1.kicad_pcb $3 $G/$1/gerbers $HOME/.cache/aet-1303b/s4
   rm -f $1-$3-$h.dump
  done
 done
done
