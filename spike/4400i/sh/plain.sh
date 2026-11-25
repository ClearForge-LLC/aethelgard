#!/bin/bash
# AET-WO-4400i (2): `simulate --foreign` with no input (the control), as cli.sh. Usage: plain.sh NAME [BOARD]
set -u
name=$1; board=${2:-s02}
O=/cache/cli/$name; rm -rf "$O"; mkdir -p "$O"
case $board in
  s02) spec=crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json; elf=/cache/boards/s02/arducopter.elf;;
  s03) spec=/cache/boards/s03/sentinel-03-foreign.json; elf=/cache/boards/s03/arducopter.elf;;
esac
A=target/debug/aethelgard
$A generate --spec "$spec" --out "$O/gen" > "$O/generate.json"
$A simulate --platform "$(ls "$O"/gen/board/*.repl | head -1)" --spec "$spec" --firmware "$elf" --foreign "${WINDOW:-5000ms}" \
  > "$O/verdict.json" 2> "$O/stderr.txt"
echo "simulate exit $?"
