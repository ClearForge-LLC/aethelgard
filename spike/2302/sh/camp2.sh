#!/bin/bash
# camp2.sh: OQ-03 trace bytes and wall for every candidate on the Sentinel-02-shaped board, as campaign archives,
# three runs each (patched CLI, spike/2302/measure.patch). (c) is two campaign runs: its device boot and its output boot.
set -u
A=/cache/target-spike/debug/aethelgard; M=/cache/m; S=spike/2302/scenarios/sentinel-02-shaped-base.json; E=$M/sentinel-02-shaped/fw.elf
mkdir -p $M/arch2; rm -rf $M/arch2/cout*
OFF="logLevel 3 sysbus.gpioPortA.pa8obs;;logLevel 3 sysbus.gpioPortA.pa9obs;;logLevel 3 sysbus.gpioPortA.pa10obs;;logLevel 3 sysbus.gpioPortA.pa11obs"
ACT="(self.Log(LogLevel.Warning, 'AETSTOP %d CR1=%x' % (cpu.ExecutedInstructions, self.ReadDoubleWord(0x40010000))), monitor.Parse('logLevel 3 sysbus.gpioPortA.pa8obs'), monitor.Parse('logLevel 3 sysbus.gpioPortA.pa9obs'), monitor.Parse('logLevel 3 sysbus.gpioPortA.pa10obs'), monitor.Parse('logLevel 3 sysbus.gpioPortA.pa11obs'), self.WriteDoubleWord(0x40010000, self.ReadDoubleWord(0x40010000) & 0xFFFFFFFE))"
STOP="sysbus AddWatchpointHook 0x40010000 DoubleWord Write \"if (value & 1): from Antmicro.Renode.Time import TimeInterval; m = self.Machine; m.ScheduleAction(TimeInterval.FromMicroseconds(2000), lambda t: $ACT)\""
run() { # tag, env...
  t=$1; shift
  s0=$(date +%s%N)
  env AET2302_MAX_NS=150000000 AET2302_WALL_S=300 AET2302_DUMP=$M/arch2/$t.log "$@" $A campaign --scenarios $S --firmware $E --archive $M/arch2/$t > $M/arch2/$t.out 2>&1; rc=$?
  w=$(( ($(date +%s%N)-s0)/1000000 )); tb=$(find $M/arch2/$t -name trace.cbor -printf '%s\n' | head -1)
  echo "$t rc=$rc campaign_wall_ms=$w renode_wall_ms=$(grep -o 'wall_ms [0-9]*' $M/arch2/$t.log.meta | cut -d' ' -f2) trace_bytes=$tb summary=$(tail -1 $M/arch2/$t.out | cut -c1-160)"
  rm -f $M/arch2/$t.log
}
for k in 1 2 3; do run cout$k AET2302_WINDOW_NS=41100000; done; exit 0
for k in 1 2 3; do
  run a$k AET2302_WINDOW_NS=44100000 AET2302_INJECT_NS=39100000
  run as$k AET2302_WINDOW_NS=44100000 AET2302_INJECT_NS=39100000 AET2302_AT_NS=41100000 "AET2302_AT_CMDS=$OFF"
  run ai$k AET2302_WINDOW_NS=100000000 AET2302_INJECT_NS=39100000 "AET2302_EXTRA=$STOP"
  run b$k AET2302_WINDOW_NS=100000000 AET2302_INJECT_NS=39100000 AET2302_AT_NS=41100000 "AET2302_AT_CMDS=$OFF"
  run cdev$k AET2302_WINDOW_NS=100000000 AET2302_INJECT_NS=39100000 AET2302_NO_OBS=1
  run cout$k AET2302_WINDOW_NS=41100000 AET2302_NO_INJECT=1
done
