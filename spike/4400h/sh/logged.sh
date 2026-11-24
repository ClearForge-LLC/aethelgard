#!/bin/bash
# AET-WO-4400h (3): the four logged replays of one capture, outside the CLI's bounds, each under capped-run at 4 GiB
# (ruling 2026-11-24T15:50Z): no injection, zero frames, the parameters without the arm, the arm. Then each run's
# queue (queue.sh) and whether the run reached its end (the last console timestamp).
# Usage: logged.sh TAG CAP      e.g. logged.sh s02 c16/2
set -u
tag=$1; cap=$2; S=/home/forge/aethelgard-4400h/spike/4400h/sh
for p in none-s pad-s params-s arm-s; do
  free -g | sed -n 2p
  MEM=4G bash $S/inject.sh "$tag-$p" "$p" "$cap" 1 2>&1 | grep -v '^frames'
  d=$HOME/.cache/aet-4400h/run/$tag-$p
  echo "$tag-$p last=$(zcat $d/console.txt.gz | grep -a -o '^\[00:00:[0-9.]*\]' | tail -1) $(cat $d/cost.txt)"
  bash $S/queue.sh "$d"
done
