#!/bin/bash
# AET-WO-4400i (2): `simulate --foreign` with a declared input, by this worktree's CLI, inside the tools image.
# Usage: cli.sh NAME INPUT [BOARD] [ARCHIVE]   BOARD s02 (sentinel-02-own, CI ELF 5eace4ca) or s03 (Sentinel-03, dbf598d8)
set -u
name=$1; in=$2; board=${3:-s02}; arch=${4:-}
O=/cache/cli/$name; rm -rf "$O"; mkdir -p "$O"
case $board in
  s02) spec=crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json; elf=/cache/boards/s02/arducopter.elf;;
  s03) spec=/cache/boards/s03/sentinel-03-foreign.json; elf=/cache/boards/s03/arducopter.elf;;
esac
A=target/debug/aethelgard
$A generate --spec "$spec" --out "$O/gen" > "$O/generate.json"
extra=(); [ -n "$arch" ] && { mkdir -p "$arch"; extra=(--archive "$arch"); }
$A simulate --platform "$(ls "$O"/gen/board/*.repl | head -1)" --spec "$spec" --firmware "$elf" --foreign "${WINDOW:-5000ms}" \
  --input "$in" "${extra[@]}" > "$O/verdict.json" 2> "$O/stderr.txt"
echo "simulate exit $?"
