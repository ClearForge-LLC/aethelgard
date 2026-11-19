#!/bin/bash
# AET-WO-4201: Sentinel-02 (U7 BMP581 on main) through the bench's BOR droop, unchanged main.
set -u
A=target/debug/aethelgard; O=target/ctl02
$A generate --spec crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json --out "$O/gen" > "$O.generate.json" || { echo "generate failed"; head -c 400 "$O.generate.json"; exit 1; }
$A build --crate "$O/gen/board" --out "$O/fw.elf" | tail -c 200; echo
$A campaign --scenarios scenarios/scratch4201/s02-droop-bor3.json --firmware "$O/fw.elf" --archive "$O/arch" 2>&1 | grep -E '"check":"campaign"|ToolError|"scenario"' | cut -c1-600
