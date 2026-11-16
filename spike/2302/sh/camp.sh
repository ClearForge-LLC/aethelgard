#!/bin/bash
# camp.sh: one campaign per board for OQ-03 trace bytes (patched CLI; env AET2302_* unset is today's behaviour).
set -u
A=/cache/target-spike/debug/aethelgard; M=/cache/m; mkdir -p $M/arch; rm -rf $M/arch/s02a* $M/arch/s9600a*
run() { # name scenario elf [env...]
  n=$1; s=$2; e=$3; shift 3
  s0=$(date +%s%N)
  env "$@" $A campaign --scenarios $s --firmware $e --archive $M/arch/$n > $M/arch/$n.out 2>&1; rc=$?
  echo "== $n rc=$rc wall_ms=$(( ($(date +%s%N)-s0)/1000000 ))"; tail -1 $M/arch/$n.out | cut -c1-400
  for t in $(find $M/arch/$n -name trace.cbor); do echo "trace $(stat -c %s $t) $t"; done
}
#run ref scenarios/sentinel-01/pullup-2k2.json $M/sentinel-01-minimal/fw.elf
#run motors scenarios/motors/dshot600.json $M/motors-dshot600/fw.elf
#run imubaro spike/2302/scenarios/imu-baro-1v8-base.json $M/imu-baro-1v8/fw.elf
run s02a spike/2302/scenarios/sentinel-02-shaped-base.json $M/sentinel-02-shaped/fw.elf AET2302_WINDOW_NS=44100000 AET2302_INJECT_NS=39100000 AET2302_MAX_NS=150000000 AET2302_WALL_S=300
run s9600a spike/2302/scenarios/serial-dshot-9600-base.json $M/serial-dshot-9600/fw.elf AET2302_WINDOW_NS=23400000 AET2302_MAX_NS=150000000 AET2302_WALL_S=300 AET2302_AT_NS=2100000 "AET2302_AT_CMDS=logLevel 3 sysbus.gpioPortA.pa8obs;;logLevel 3 sysbus.gpioPortA.pa9obs;;logLevel 3 sysbus.gpioPortA.pa10obs;;logLevel 3 sysbus.gpioPortA.pa11obs"
