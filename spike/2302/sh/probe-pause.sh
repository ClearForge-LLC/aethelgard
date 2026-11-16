#!/bin/bash
# probe-pause.sh: which call, scheduled 2 ms after TIM1's CEN, ends the boot's RunFor early?
set -u
f=spike/2302/fixtures/sentinel-02-shaped.json; n=sentinel-02-shaped
for call in "emulationManager.CurrentEmulation.PauseAll()" "m.Pause()" "monitor.Parse(\\\"pause\\\")"; do
  H="sysbus AddWatchpointHook 0x40010000 DoubleWord Write \"if (value & 1): from Antmicro.Renode.Time import TimeInterval; m = self.Machine; m.ScheduleAction(TimeInterval.FromMicroseconds(2000), lambda t: (self.Log(LogLevel.Warning, \\\"AETSCHED %d\\\" % cpu.ExecutedInstructions), $call))\""
  echo "== $call"
  AET2302_TAG=pause AET2302_WINDOW_NS=100000000 AET2302_NO_OBS=1 AET2302_WALL_S=60 AET2302_EXTRA="$H" timeout 120 /cache/m.sh $f $n 100ms
  grep -E "AETSCHED|rror|xception|AETHELGARD-BOOT-BEGIN" /cache/m/$n/pause.log | cut -c1-200 | head -6
  python3 -I -c "import json; v=json.load(open('/cache/m/$n/pause.verdict.json')); b=v.get('boot',{}); print('instructions', b.get('instructions'), 'error', v.get('error'))"
done
