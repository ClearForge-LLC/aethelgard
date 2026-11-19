#!/bin/bash
# AET-WO-4201: the seeded I2C fault cases on the reference specification with U2 removed.
set -u
A=target/debug/aethelgard; O=target/ctl47
$A generate --spec examples/sentinel-01-minimal.json --out "$O/gen" > "$O.generate.json" || { echo "generate failed"; exit 1; }
$A build --crate "$O/gen/board" --out "$O/fw.elf" | tail -c 300; echo
$A campaign --scenarios scenarios/sentinel-01/pullup-47k.json scenarios/sentinel-01/pullup-2k2.json scenarios/sentinel-01/seeded/held-low-sda.json --firmware "$O/fw.elf" --archive "$O/arch" 2>&1 | cut -c1-500
