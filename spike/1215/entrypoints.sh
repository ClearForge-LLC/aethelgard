#!/usr/bin/env bash
# AET-WO-1215 §1.1: every specification-reading entry point, on every file in IN, with one binary.
# Prints one line per (file, entry point): the label, the exit code and the stdout bytes.
# Usage: bash spike/1215/entrypoints.sh BINARY IN_DIR SCRATCH > out/<label>.txt
set -u
bin=$1 in=$2 scratch=$3
mkdir -p "$scratch"
: > "$scratch/empty.kicad_pcb"
: > "$scratch/empty.kicad_sch"
for f in "$in"/*.json; do
  n=$(basename "$f" .json)
  run() {
    local label=$1; shift
    local o
    o=$(cd "$scratch" && timeout 120 "$@" 2>/dev/null); local rc=$?
    printf '%s\t%s\t%s\t%s\n' "$n" "$label" "$rc" "$o"
  }
  rm -rf "$scratch/out"
  run "validate --file" "$bin" validate --file "$f"
  run "validate <stdin" sh -c '"$0" validate < "$1"' "$bin" "$f"
  run "solve" "$bin" solve --file "$f"
  run "emit" "$bin" emit --file "$f" --out "$scratch/out"; rm -rf "$scratch/out"
  run "generate" "$bin" generate --spec "$f" --out "$scratch/out"; rm -rf "$scratch/out"
  run "extract" "$bin" extract --board "$scratch/empty.kicad_pcb" --spec "$f"
  run "check drc" "$bin" check drc --board "$scratch/empty.kicad_pcb" --spec "$f"
  run "check support" "$bin" check support --schematic "$scratch/empty.kicad_sch" --board "$scratch/empty.kicad_pcb" --spec "$f"
  run "check speeds" "$bin" check speeds --spec "$f"
  run "verify" "$bin" verify --spec "$f"
done
