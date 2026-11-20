#!/bin/bash
# AET-WO-4240a §1.1: the CLI's validate and solve stdout and exit, per committed specification.
# Usage: cli-sweep.sh <aethelgard binary> <repo root> <out dir>
A=$1; R=$2; O=$3; mkdir -p "$O"; : > "$O/rows.tsv"
cd "$R" || exit 1
while read -r s; do
  n=$(echo "$s" | tr '/' '_' | sed 's/\.json$//')
  "$A" validate --file "$s" > "$O/$n.validate" 2>/dev/null; v=$?
  "$A" solve --file "$s" > "$O/$n.solve" 2>/dev/null; so=$?
  printf '%s\tvalidate=%s\tsolve=%s\n' "$s" "$v" "$so" >> "$O/rows.tsv"
done < ~/.cache/aet-4240a/specs.txt
