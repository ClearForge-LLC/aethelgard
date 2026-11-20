#!/bin/bash
# AET-WO-4240a P13: generate (board crate + KiCad emission) per committed specification. Usage: gen-sweep.sh BIN REPO OUT
A=$1; R=$2; O=$3; rm -rf "$O"; mkdir -p "$O"; cd "$R" || exit 1
while read -r s; do
  n=$(echo "$s" | tr '/' '_' | sed 's/\.json$//')
  "$A" generate --spec "$s" --out "$O/$n" > "$O/$n.out" 2>&1; echo "$s generate=$?" >> "$O/rows.txt"
done < ~/.cache/aet-4240a/specs.txt
