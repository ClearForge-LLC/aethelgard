#!/bin/bash
# AET-WO-1919: the two lane-process tests as CI runs them (root, in the pinned image, 4 cores): d507f8fb's, then this head's.
set -u
echo "uid=$(id -u)"
f=crates/aethelgard-cli/tests/cli_backlog_tier1.rs
cp "$f" /tmp/new.rs
cp /repo/target/old_cli_backlog.rs "$f"; touch "$f"
echo "== d507f8fb's tests"
taskset -c 0-3 cargo test --locked --offline -q -p aethelgard-cli --test cli_backlog_tier1 -- leaves_no_lane killed_outright 2>&1 | grep -E "test result|panicked|never started" | cut -c1-200
cp /tmp/new.rs "$f"; touch "$f"
echo "== this head's tests, twice"
for i in 1 2; do taskset -c 0-3 cargo test --locked --offline -q -p aethelgard-cli --test cli_backlog_tier1 -- leaves_no_lane killed_outright --nocapture 2>&1 | grep -E "ANSWER|test result|panicked" | cut -c1-200; done
