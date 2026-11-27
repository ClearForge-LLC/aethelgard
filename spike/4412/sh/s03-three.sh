#!/bin/bash
# AET-WO-4412 (19:38Z ruling): Sentinel-03's three foreign runs on the merged tree, one at a time, in the tools image:
# arm (armread12: MOT_PWM_TYPE read back), dshot (dshot12), reboot (reboot12), 12 s each, on ELF /cache/boards/dshot2.
set -u
cargo build -q --locked -p aethelgard-cli && cargo build -q --locked --manifest-path bridge/Cargo.toml || exit 9
for r in armread12 dshot12 reboot12; do
  bash /spike/sh/cli.sh s03m-$r 12000ms examples/sentinel-03.json /cache/boards/dshot2/out/sentinel-03/arducopter.elf /cache/inputs/$r.json
done
