#!/bin/bash
# In the image: build the CLI (with-tools) and bridge, then verify Sentinel-03 on main's fixture with every scenario.
set -u
mkdir -p /cache/home
cd /repo && cargo build -q --locked --offline -p aethelgard-cli --features with-tools && cargo build -q --locked --offline --manifest-path bridge/Cargo.toml || exit 1
VFLAGS=${VFLAGS:-}; A=$CARGO_TARGET_DIR/debug/aethelgard
rm -rf /cache/s03-pkg
S=$(ls scenarios/sentinel-03/*.json)
start=$(date +%s)
$A verify --spec crates/aethelgard-cli/tests/fixtures/sentinel-03.json --scenarios $S $VFLAGS > /cache/verify-s03${TAG:-}.json; rc=$?
echo "verify rc=$rc seconds=$(( $(date +%s) - start ))"
