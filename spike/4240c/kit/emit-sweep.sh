#!/bin/bash
# AET-WO-4240c §1.1: `aethelgard emit` per committed specification, then the sha256 of every file written.
# Usage: emit-sweep.sh BIN REPO SPECS OUT
set -u
A=$1; R=$2; L=$3; O=$4; rm -rf "$O"; mkdir -p "$O"; cd "$R" || exit 1
while read -r s; do
  n=$(echo "$s" | tr '/' '_' | sed 's/\.json$//')
  rc=0; "$A" emit --file "$s" --out "$O/$n" > "$O/$n.out" 2>&1 || rc=$?
  echo "$s emit=$rc" >> "$O/rows.txt"
  sed -i "s#$O/#OUT/#g" "$O/$n.out"
done < "$L"
( cd "$O" && find . -type f \( -name '*.kicad_sch' -o -name '*.kicad_pcb' -o -name '*.bom.csv' \) -print0 | sort -z | xargs -0 sha256sum ) > "$O/SHA256SUMS"
