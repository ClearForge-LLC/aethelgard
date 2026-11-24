#!/bin/bash
# AET-WO-4400h (3), ruled 2026-11-24T15:50Z: unmodified Copter-4.7.2 for Sentinel-03, a board with motors and an IMU.
#   s03.sh hwdef   (tools image)      solve + `generate hwdef` -> /cache/s03/boards/sentinel-03.{topology.json,hwdef.dat}
#   s03.sh build   (ardupilot image, ROOT=1, 6 cores)  CI's own container/ardupilot-build.sh boards sentinel-03, with
#                  /boards and /out linked to /cache/s03 -> /cache/s03/out/sentinel-03/{arducopter.elf,hwdef.h}
set -eu
spec=/cache/s03/sentinel-03-foreign.json   # py/derive.py: Sentinel-03 + sentinel-02-own's foreign_clock and U5 orientation
case $1 in
  hwdef)
    mkdir -p /cache/s03/boards
    python3 -I /spike/py/derive.py crates/aethelgard-cli/tests/fixtures/sentinel-03.json \
      crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json "$spec"
    A=target/debug/aethelgard
    $A solve --file "$spec" > /cache/s03/boards/sentinel-03.topology.json
    $A generate hwdef --topology /cache/s03/boards/sentinel-03.topology.json --out /cache/s03/boards/sentinel-03.hwdef.dat
    sha256sum crates/aethelgard-cli/tests/fixtures/sentinel-03.json "$spec" /cache/s03/boards/sentinel-03.hwdef.dat
    grep -n '^IMU\|^PA8\|^PE1[134]\|DSHOT\|^SERIAL_ORDER\|BATT\|ADC' /cache/s03/boards/sentinel-03.hwdef.dat ;;
  build)
    mkdir -p /cache/s03/out; ln -sfn /cache/s03/boards /boards; ln -sfn /cache/s03/out /out
    taskset -c 0-5 bash /repo/container/ardupilot-build.sh boards sentinel-03
    sha256sum /cache/s03/out/sentinel-03/arducopter.elf /cache/s03/out/sentinel-03/hwdef.h ;;
esac
