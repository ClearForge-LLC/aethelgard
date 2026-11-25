#!/bin/bash
# AET-WO-4400j: one input run of main's Sentinel-03 with --archive, by the CLI at /repo, for comparing traces between
# builds. Usage: archive.sh NAME WINDOW INPUT   -> /cache/arch/NAME/{verdict.json,archive/}
set -u
name=$1; w=$2; in=$3
O=/cache/arch/$name; rm -rf "$O"; mkdir -p "$O"
spec=/cache/boards/s03/sentinel-03-foreign.json; elf=/cache/boards/s03/arducopter.elf
A=target/debug/aethelgard
$A generate --spec "$spec" --out "$O/gen" > /dev/null || exit 1
mkdir -p "$O/archive"
$A simulate --platform "$(ls "$O"/gen/board/*.repl | head -1)" --spec "$spec" --firmware "$elf" --foreign "$w" --input "$in" \
  --archive "$O/archive" > "$O/verdict.json" 2> "$O/stderr.txt"
echo "simulate exit $?"; find "$O/archive" -type f | sort | head -20
