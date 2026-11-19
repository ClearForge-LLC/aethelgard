#!/bin/bash
# The (b) batch, one container (sequential inside): every case 1 s simulated at the weak pack.
set -u
for c in "I1-100us 0.4 100000" "I1-100us-r2 0.4 100000" "I1-10us 0.4 10000" "I2-100us 2.0 100000" "I2-10us 2.0 10000" "T10-100us 10 100000" "T10-10us 10 10000"; do
  bash /repo/spike/2304/sh/runb.sh $c
done
