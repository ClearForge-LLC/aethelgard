#!/bin/bash
# P4: the fold's memory, with the real replay_dir, each trace in a fresh process, three times.
set -eu
L=/cache/target/release/lockstep
for d in sched-tolerance-r1/trace-samples bor3-10us-r1/trace-events bor3-10us-r1/trace-samples bor3-10us-r1/trace-records; do
  for r in 1 2 3; do $L replay /cache/p26/$d; done
done
