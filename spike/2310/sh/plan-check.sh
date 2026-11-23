#!/bin/bash
# AET-WO-2310, inside the tools image (img-rw.sh, REPO = a writable copy of the branch): ci/shard.sh plan's listing and
# assignment, without its "the shared build compiles nothing" check (a local tree compiles), so a stale or missing
# weights row is found before CI's planner refuses it.
set -u
cd /repo
O=target/ci-shard-local; rm -rf $O; mkdir -p $O
cargo build -q --locked --offline --features aethelgard-cli/with-tools --workspace --all-targets > $O/build.log 2>&1 || { tail -5 $O/build.log; exit 9; }
cargo test --workspace --locked --offline --features aethelgard-cli/with-tools -- --list > $O/list-ws.log 2>&1 || { tail -5 $O/list-ws.log; exit 8; }
cargo test --manifest-path bridge/Cargo.toml --locked --offline -- --list > $O/list-bridge.log 2>&1 || { tail -5 $O/list-bridge.log; exit 7; }
{ bash ci/shard.sh parse ws $O/list-ws.log; bash ci/shard.sh parse bridge $O/list-bridge.log; } > $O/entries.tsv
n=$(tr -d '[:space:]' < ci/SHARDS)
bash ci/shard.sh assign "$n" ci/test-weights.tsv $O/entries.tsv $O 37 ci/colocate.tsv 'tests/board_tier2.rs tests/against_fab_tier2.rs tests/verify_2k2_tier2.rs tests/twin_tier2.rs' && echo "assign rc=0"
bash ci/shard.sh guard "$n" $O/entries.tsv $O && echo "guard rc=0"
