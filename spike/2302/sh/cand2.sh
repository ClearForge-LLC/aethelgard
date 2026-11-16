#!/bin/bash
# cand2.sh: candidate (a) on the Sentinel-02-shaped fixture, with the window holding today's wire budget.
set -u
f=spike/2302/fixtures/sentinel-02-shaped.json; n=sentinel-02-shaped
OFF="logLevel 3 sysbus.gpioPortA.pa8obs;;logLevel 3 sysbus.gpioPortA.pa9obs;;logLevel 3 sysbus.gpioPortA.pa10obs;;logLevel 3 sysbus.gpioPortA.pa11obs"
common="AET2302_MAX_NS=150000000 AET2302_WALL_S=300 AET2302_INJECT_NS=39100000 AET2302_WINDOW_NS=44100000"
for k in 1 2 3; do
  echo "== (a) k=$k: boot 44.1 ms, observers logged to 41.1 ms then silenced, frames injected at 39.1 ms"
  env $common AET2302_TAG=as$k AET2302_AT_NS=41100000 AET2302_AT_CMDS="$OFF" /cache/rss.sh /cache/m/$n/rss-as$k /cache/m.sh $f $n 44100us
done
echo "== (a-log): boot 44.1 ms, observers logged throughout"
env $common AET2302_TAG=al1 /cache/rss.sh /cache/m/$n/rss-al1 /cache/m.sh $f $n 44100us
