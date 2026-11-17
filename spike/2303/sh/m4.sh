#!/bin/bash
# Inside the image: the samples shape just under 256 MiB, replayed; then 1 s of each case.
set -u
L=/cache/target/release/lockstep
echo "== synth 44 s"; $L synth 44 /cache/runs/weak-100ms-1 /cache/synth/s44 | cut -c1-200; tail -1 <($L synth 44 /cache/runs/weak-100ms-1 /cache/synth/s44)
rm -rf /cache/synth
for c in weak healthy; do echo "== $c 1 s"; bash /repo/spike/2303/sh/run.sh $c 1000000000 /cache/runs/$c-1s; done
