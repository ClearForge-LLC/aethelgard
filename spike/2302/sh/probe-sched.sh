#!/bin/bash
# probe-sched.sh: can a watchpoint hook on TIM1 CR1 (CEN) schedule an action 2 ms later, and can that action pause the run?
set -u
f=spike/2302/fixtures/sentinel-02-shaped.json; n=sentinel-02-shaped
H='sysbus AddWatchpointHook 0x40010000 DoubleWord Write "if (value & 1): from Antmicro.Renode.Time import TimeInterval; m = self.Machine; self.Log(LogLevel.Warning, \"AETCEN %d\" % cpu.ExecutedInstructions); m.ScheduleAction(TimeInterval.FromMicroseconds(2000), lambda t: (self.Log(LogLevel.Warning, \"AETSCHED %d\" % cpu.ExecutedInstructions), m.PauseAndRequestEmulationPause()))"'
for mode in log pause; do
  if [ $mode = log ]; then HH=${H/, m.PauseAndRequestEmulationPause()/}; else HH=$H; fi
  echo "== $mode"
  AET2302_TAG=sched-$mode AET2302_WINDOW_NS=100000000 AET2302_NO_OBS=1 AET2302_WALL_S=120 AET2302_EXTRA="$HH;;sysbus AddWatchpointHook 0x20000000 DoubleWord Write \"self.Log(LogLevel.Warning, \\\"AETBW %d\\\" % cpu.ExecutedInstructions)\"" /cache/m.sh $f $n 100ms
  grep -E "AETCEN|AETSCHED|AETBW|rror|xception" /cache/m/$n/sched-$mode.log | cut -c1-220 | head -12
  python3 -I -c "import json; v=json.load(open('/cache/m/$n/sched-$mode.verdict.json')); b=v.get('boot',{}); print('instructions', b.get('instructions'), 'error', v.get('error'))"
done
