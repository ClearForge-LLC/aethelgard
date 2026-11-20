#!/bin/bash
# AET-WO-4240c A7: `aethelgard extract` stdout and exit code on every board a sweep emitted, against its
# specification. Usage: extract-sweep.sh BIN REPO SWEEP OUT (SWEEP from emit-sweep.sh; the board paths are
# normalised so two sweeps compare)
set -u
A=$1; R=$2; S=$3; O=$4; rm -rf "$O"; mkdir -p "$O"; cd "$R" || exit 1
while read -r s rc; do
  [ "$rc" = "emit=0" ] || continue
  n=$(echo "$s" | tr '/' '_' | sed 's/\.json$//')
  b=$(ls "$S/$n"/*.kicad_pcb)
  r=0; "$A" extract --board "$b" --spec "$s" > "$O/$n.out" 2>&1 || r=$?
  sed -i "s#$S/#SWEEP/#g" "$O/$n.out"; echo "$s extract=$r" >> "$O/rows.txt"
done < "$S/rows.txt"
