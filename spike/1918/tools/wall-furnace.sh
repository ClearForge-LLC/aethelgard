#!/bin/bash
# AET-WO-1918 (2), inside the image at /repo: the tier-2 tests that the 30 s wall bound judges,
# run as a CI shard runs them (libtest's default threads on a 4-vCPU runner), pinned to CPUS
# (default 0-3, four of Furnace's cores; "all" for no pin). Each boot's wall time comes from
# the host probe (tools/ci_probe.py, Renode's lifetime) and from twin_tier2's own
# `ANSWER 2100: turn: ... held it N s` lines (the whole call).
# usage: [CPUS=0-3] bash wall-furnace.sh <test binary>...
set -u
lscpu | grep -E '^(Model name|CPU\(s\)|Thread|Core|Socket)'; nproc
cargo build -q --locked --offline -p aethelgard-cli --features with-tools --tests || exit 2
pin=(); [ "${CPUS:-0-3}" = all ] || pin=(taskset -c "${CPUS:-0-3}")
export RUST_TEST_THREADS=${RUST_TEST_THREADS:-4}
for t in "$@"; do
  echo "WALL1918 start $t $(date -u +%T) cpus=${CPUS:-0-3} threads=$RUST_TEST_THREADS"
  "${pin[@]}" cargo test --locked --offline -p aethelgard-cli --features with-tools --test "$t" 2>&1
  echo "WALL1918 end $t rc=$? $(date -u +%T)"
done
