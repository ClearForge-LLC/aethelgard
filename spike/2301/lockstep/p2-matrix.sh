#!/bin/bash
# P2: every case at 10 µs and 1 µs quanta, ngspice alone, 3 ms span (the droop is at 1 ms),
# then the cost of 100 ms of rail at both quanta, at two step caps (ngspice's default and 1 µs).
set -eu
L=/cache/target/release/lockstep
for c in tolerance bor3 pdr dip-between dip-straddle; do
  for q in 10000 1000; do $L rail $c $q 3000000 /cache/p2/$c-$q; done
done
for tm in "" 1000; do
  for q in 10000 1000; do if [ -n "$tm" ]; then export TMAX_NS=$tm; else unset TMAX_NS; fi; $L rail bor3 $q 100000000 /cache/p2/cost-$q-${tm:-def}; done
done
