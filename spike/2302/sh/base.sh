#!/bin/bash
# base.sh: today's cost (env unset) three times per fixture, then (a-i) on the two other DShot boards.
set -u
F=crates/aethelgard-cli/tests/fixtures
for k in 1 2 3; do
  for spec in examples/sentinel-01-minimal.json:100ms $F/motors-dshot600.json:2ms $F/imu-baro-1v8.json:100ms $F/motors-pwm.json:100ms; do
    f=${spec%%:*}; t=${spec##*:}; n=$(basename $f .json)
    echo "== today $n $t k=$k"; AET2302_TAG=base$k /cache/rss.sh /cache/m/$n.rss-base$k /cache/m.sh $f $n $t
  done
done
ACT() { # timer base, observers
  local a="(self.Log(LogLevel.Warning, 'AETSTOP %d CR1=%x' % (cpu.ExecutedInstructions, self.ReadDoubleWord($1)))"
  for o in pa8obs pa9obs pa10obs pa11obs; do a="$a, monitor.Parse('logLevel 3 sysbus.gpioPortA.$o')"; done
  echo "$a, self.WriteDoubleWord($1, self.ReadDoubleWord($1) & 0xFFFFFFFE))"
}
H="sysbus AddWatchpointHook 0x40010000 DoubleWord Write \"if (value & 1): from Antmicro.Renode.Time import TimeInterval; m = self.Machine; m.ScheduleAction(TimeInterval.FromMicroseconds(2000), lambda t: $(ACT 0x40010000))\""
for spec in $F/motors-dshot600.json:0 $F/serial-dshot-9600.json:100000; do
  f=${spec%%:*}; inj=${spec##*:}; n=$(basename $f .json)
  for k in 1 2 3; do
    echo "== (a-i) $n k=$k"
    if [ $inj = 0 ]; then unset AET2302_INJECT_NS; else export AET2302_INJECT_NS=$inj; fi
    AET2302_TAG=ai$k AET2302_WINDOW_NS=100000000 AET2302_WALL_S=60 AET2302_EXTRA="$H" /cache/rss.sh /cache/m/$n.rss-ai$k /cache/m.sh $f $n 100ms
    grep -E "AETSTOP" /cache/m/$n/ai$k.log | cut -c1-120
  done
done
