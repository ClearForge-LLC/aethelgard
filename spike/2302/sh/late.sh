#!/bin/bash
# late.sh: the delayed-start_outputs mutant under (a-i)'s hooks: where is the anchor, and does the judge's stop fire?
set -u
f=spike/2302/fixtures/sentinel-02-shaped.json; n=s02-late
/cache/target-spike/debug/aethelgard build --crate /cache/m/$n/gen/board --out /cache/m/$n/fw.elf > /cache/m/$n/build.json 2>&1 || { head -c 800 /cache/m/$n/build.json; exit 1; }
H="sysbus AddWatchpointHook 0x40010000 DoubleWord Write \"if (value & 1): from Antmicro.Renode.Time import TimeInterval; m = self.Machine; self.Log(LogLevel.Warning, 'AETCEN %d' % cpu.ExecutedInstructions); m.ScheduleAction(TimeInterval.FromMicroseconds(2000), lambda t: self.Log(LogLevel.Warning, 'AETSTOP %d' % cpu.ExecutedInstructions))\";;sysbus AddWatchpointHook 0x20000000 DoubleWord Write \"self.Log(LogLevel.Warning, 'AETBW %d' % cpu.ExecutedInstructions)\""
AET2302_TAG=late AET2302_WINDOW_NS=100000000 AET2302_INJECT_NS=39100000 AET2302_WALL_S=60 AET2302_EXTRA="$H" /cache/rss.sh /cache/m/$n.rss /cache/m.sh $f $n 100ms
grep -E "AETCEN|AETSTOP|AETBW" /cache/m/$n/late.log | cut -c1-120
