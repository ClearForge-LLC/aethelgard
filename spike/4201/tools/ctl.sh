#!/bin/bash
# AET-WO-4201 control: in the image, generate, build and run the bench campaign
# (the five scenarios verify --fab-out runs), N times, printing each line.
# usage (inside /repo): bash ctl.sh <out-dir-name> <runs>
set -u
A=target/debug/aethelgard; O=target/$1; N=${2:-1}
SC="scenarios/sentinel-01-bench/pullup-3k3.json scenarios/sentinel-01-bench/droop-bor3.json scenarios/sentinel-01-bench/spi-absent-u4.json scenarios/sentinel-01-bench/pullup-2k2.json scenarios/sentinel-01-bench/serial-rx-open-console.json"
$A generate --spec examples/sentinel-01.json --out "$O/gen" > "$O.generate.json" || { echo "generate failed"; exit 1; }
$A build --crate "$O/gen/board" --out "$O/fw.elf" | tail -c 300; echo
for i in $(seq 1 "$N"); do
  echo "== campaign run $i"
  $A campaign --scenarios $SC --firmware "$O/fw.elf" --archive "$O/arch$i" 2>&1 | cut -c1-400
done
