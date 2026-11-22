#!/bin/bash
# M3 for D1: main's CLI with draws_a_board alone skipped in verify (coverage-d1.patch), in a scratch
# copy of the base commit (never committed), built on the host; then verify --fab-out on D1 and its
# baseline, which refuses CampaignCoverageShort before any gate or tool.
set -eu
K=$(cd "$(dirname "$0")" && pwd); C=$HOME/.cache/aet-1211c; S=$C/cov-src
rm -rf "$S"; mkdir -p "$S"
git -C "$K/../../.." archive 7fc81640 | tar -x -C "$S"
sed -i 's/    if let Err(r) = crate::draws_a_board(&spec, command) {/    if let Err(r) = crate::draws_a_board(\&spec, command).or_else(|_| Ok(())) {/' "$S/crates/aethelgard-cli/src/verify.rs"
grep -c 'or_else(|_| Ok(()))' "$S/crates/aethelgard-cli/src/verify.rs"
(cd "$S" && CARGO_TARGET_DIR=$C/target-cov nice -n 10 cargo build -q -p aethelgard-cli)
cd "$C/m"; rm -rf d1/fab
rc=0; "$C/target-cov/debug/aethelgard" verify --spec d1.json --scenarios sc/d1-baseline.json --fab-out d1/fab > d1/verify-fab-cov.json 2>&1 || rc=$?
echo "rc=$rc"; cat d1/verify-fab-cov.json
