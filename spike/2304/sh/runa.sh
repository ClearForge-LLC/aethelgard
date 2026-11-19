#!/bin/bash
# Inside the image: candidate (a), declared, for the three cases at 1.0 ohm (weak), 1 s.
# Output under /cache/runs/a-<case>; run.json is the probe's line.
set -u
L=/cache/target/release/lockstep
for c in "I1 0.4" "I2 2.0" "T10 10"; do
  set -- $c
  O=/cache/runs/a-$1; rm -rf $O; mkdir -p $O
  s=$(date +%s%N)
  I_RUN_A=$2 $L declared weak 1000000000 $O > $O/run.json 2> $O/run.err
  echo "$1 exit=$? wall_ms=$(( ($(date +%s%N)-s)/1000000 ))"
  $L replay $O > $O/replay.jsonl 2>&1; echo "  replay exit=$?"
done
