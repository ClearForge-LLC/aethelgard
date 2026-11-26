#!/bin/bash
# AET-WO-4400j (1): the replay family, one container each (sh/img.sh, 4 cores pinned, 8 GiB cap). Usage: replays.sh NAME...
#   stats      cap s03-12, probes each second            stats-in   cap s03-12-in, the same with the input
#   fullgc     cap s03-12, FULLGC=1                      dump       cap s03-12, PAUSE="6 10" (ROOT=1: createdump)
#   reads      cap s03-12, end reads every 10 ms from 4 s  reads-in   the same with the input
K=/home/forge/aethelgard-4400j/spike/4400j/sh
for n in "$@"; do
  case $n in
    stats) e=""; c=s03-12;; stats-in) e=""; c=s03-12-in;;
    fullgc) e="FULLGC=1"; c=s03-12;; dump) e="PAUSE='6 10'"; c=s03-12;;
    reads) e="STEP_MS=10 FROM_MS=4000"; c=s03-12;; reads-in) e="STEP_MS=10 FROM_MS=4000"; c=s03-12-in;;
    *) e=${E:-}; c=${C:-s03-12};;
  esac
  root=0; [ "$n" = dump ] && root=1
  ROOT=$root CPUS=${CPUS:-0-3} MEM=8G "$K/img.sh" tools "$e bash /spike/sh/replay.sh /cache/cap/$c/2 /cache/rep/$n" 2>&1 | tail -2
done
