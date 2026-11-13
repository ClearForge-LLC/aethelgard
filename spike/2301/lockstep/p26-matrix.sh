#!/bin/bash
# P2.6 and P4: lock-step (ioc, Renode as child) and the schedule, end to end, with the generated firmware.
# Step cap 1 µs (P2: the same crossings as ngspice's 10 ns default to 0.001 ns). Each lock-step at
# 10 µs runs twice, for INV-04 (the traces' state hashes must match). Raw output under /cache/p26.
set -eu
export TMAX_NS=1000
L=/cache/target/release/lockstep
for c in tolerance bor3 pdr dip-between; do
  for r in 1 2; do $L lockstep $c 10000 100000000 /cache/p26/$c-10us-r$r; done
done
$L lockstep bor3 1000 10000000 /cache/p26/bor3-1us-10ms
$L lockstep dip-between 1000 10000000 /cache/p26/dip-between-1us-10ms
for c in tolerance bor3 pdr dip-between; do
  for r in 1 2; do $L schedule $c 100000000 /cache/p26/sched-$c-r$r; done
done
