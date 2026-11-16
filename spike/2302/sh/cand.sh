#!/bin/bash
# cand.sh: candidates (a), (b), (c) on the Sentinel-02-shaped fixture, three runs each, with peak RSS.
set -u
f=spike/2302/fixtures/sentinel-02-shaped.json; n=sentinel-02-shaped
OFF="logLevel 3 sysbus.gpioPortA.pa8obs;;logLevel 3 sysbus.gpioPortA.pa9obs;;logLevel 3 sysbus.gpioPortA.pa10obs;;logLevel 3 sysbus.gpioPortA.pa11obs"
common="AET2302_MAX_NS=150000000 AET2302_WALL_S=300 AET2302_INJECT_NS=39100000"
for k in 1 2 3; do
  echo "== (a) k=$k: boot to start_outputs + 2 ms (41.1 ms), observers logged, frames injected at 39.1 ms"
  env $common AET2302_TAG=a$k AET2302_WINDOW_NS=41100000 /cache/rss.sh /cache/m/$n/rss-a$k /cache/m.sh $f $n 41100us
  echo "== (b) k=$k: 100 ms boot, observers logged until 41.1 ms then silenced"
  env $common AET2302_TAG=b$k AET2302_WINDOW_NS=100000000 AET2302_AT_NS=41100000 AET2302_AT_CMDS="$OFF" /cache/rss.sh /cache/m/$n/rss-b$k /cache/m.sh $f $n 100ms
  echo "== (c-device) k=$k: 100 ms device boot, observers silenced throughout"
  env $common AET2302_TAG=c$k AET2302_WINDOW_NS=100000000 AET2302_NO_OBS=1 /cache/rss.sh /cache/m/$n/rss-c$k /cache/m.sh $f $n 100ms
done
