#!/bin/bash
# AET-WO-4400j: unmodified Copter-4.7.2 for main's Sentinel-03 (py/derive.py adds mcu.foreign_clock only).
#   board.sh hwdef   (tools image)  validate + solve + `generate hwdef` -> /cache/boards/s03/{topology.json,hwdef.dat}
#   board.sh build   (ardupilot image, ROOT=1)  CI's own container/ardupilot-build.sh boards sentinel-03 -> arducopter.elf
set -eu
B=/cache/boards/s03; spec=$B/sentinel-03-foreign.json
case $1 in
  hwdef)
    A=target/debug/aethelgard
    $A validate --file "$spec" | head -c 300; echo
    $A solve --file "$spec" > $B/topology.json
    mkdir -p $B/boards
    $A generate hwdef --topology $B/topology.json --out $B/boards/sentinel-03.hwdef.dat
    cp $B/topology.json $B/boards/sentinel-03.topology.json
    sha256sum "$spec" $B/topology.json $B/boards/sentinel-03.hwdef.dat
    grep -n '^OSCILLATOR\|^IMU\|^PA8\|DSHOT\|^SERIAL_ORDER\|^MCU' $B/boards/sentinel-03.hwdef.dat ;;
  build)
    mkdir -p $B/out; ln -sfn $B/boards /boards; ln -sfn $B/out /out
    taskset -c 0-5 bash /repo/container/ardupilot-build.sh boards sentinel-03
    cp $B/out/sentinel-03/arducopter.elf $B/arducopter.elf
    sha256sum $B/out/sentinel-03/arducopter.elf $B/out/sentinel-03/hwdef.h ;;
esac
