#!/bin/bash
# probe-stop.sh: (a3) a 100 ms boot in which the judge's own action, 2 ms after TIM1's CEN, silences the observers
# and clears CEN (after reading TIM1's CR1 back into the log); frames injected at 39.1 ms. Three runs, peak RSS.
set -u
f=spike/2302/fixtures/sentinel-02-shaped.json; n=sentinel-02-shaped
ACT="(self.Log(LogLevel.Warning, 'AETSTOP %d CR1=%x' % (cpu.ExecutedInstructions, self.ReadDoubleWord(0x40010000))), monitor.Parse('logLevel 3 sysbus.gpioPortA.pa8obs'), monitor.Parse('logLevel 3 sysbus.gpioPortA.pa9obs'), monitor.Parse('logLevel 3 sysbus.gpioPortA.pa10obs'), monitor.Parse('logLevel 3 sysbus.gpioPortA.pa11obs'), self.WriteDoubleWord(0x40010000, self.ReadDoubleWord(0x40010000) & 0xFFFFFFFE))"
H="sysbus AddWatchpointHook 0x40010000 DoubleWord Write \"if (value & 1): from Antmicro.Renode.Time import TimeInterval; m = self.Machine; m.ScheduleAction(TimeInterval.FromMicroseconds(2000), lambda t: $ACT)\""
for k in 1 2 3; do
  echo "== (a3) k=$k"
  AET2302_TAG=stop$k AET2302_WINDOW_NS=100000000 AET2302_INJECT_NS=39100000 AET2302_WALL_S=60 AET2302_EXTRA="$H" /cache/rss.sh /cache/m/$n/rss-stop$k timeout 120 /cache/m.sh $f $n 100ms
  grep -E "AETSTOP|xception|rror" /cache/m/$n/stop$k.log | cut -c1-200 | head -4
done
grep -c "pa8obs" /cache/m/$n/stop1.log
