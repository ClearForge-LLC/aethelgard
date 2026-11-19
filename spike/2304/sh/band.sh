#!/bin/bash
# Runs added after the predictions (unpredicted): I1 (0.4 A while CEN) either side of D1's band
# (1.27-1.43 ohm at declared eta), 1 s at 100 us under (b), and (a) at both.
set -u
L=/cache/target/release/lockstep
for R in 1.20 1.35 1.45; do
  bash /repo/spike/2304/sh/runb.sh "I1-r$R-100us" 0.4 100000 1000000000 "r$R"
  O=/cache/runs/a-I1-r$R; rm -rf $O; mkdir -p $O
  I_RUN_A=0.4 $L declared r$R 1000000000 $O > $O/run.json 2> $O/run.err; echo "a-I1-r$R exit=$?"
done
