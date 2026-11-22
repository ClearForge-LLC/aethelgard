#!/bin/bash
# AET-WO-2308: CI's bench-package job inside the tools image (run through ci-img.sh, the tree at /repo
# and CARGO_TARGET_DIR=/repo/target/img as CI has it): build the CLI and the analog bridge side by side,
# then the job's verify with --fab-out $FABOUT (or --against-fab the committed package with AGAINST=1).
# Usage (in the container): FABOUT=/cache/fab-x OUT=/cache/bench-x bash bench.sh
set -u
cd /repo
mkdir -p "${OUT:?}"
cargo build --offline --locked -p aethelgard-cli > "$OUT/build.log" 2>&1 \
  && cargo build --offline --manifest-path bridge/Cargo.toml --locked >> "$OUT/build.log" 2>&1 \
  || { echo "build failed"; tail -20 "$OUT/build.log"; exit 1; }
if [ -n "${AGAINST:-}" ]; then flag=(--against-fab fab/sentinel-01-bench/package); else flag=(--fab-out "${FABOUT:?}"); fi
s=$(date +%s)
target/img/debug/aethelgard verify --spec examples/sentinel-01.json \
  --scenarios scenarios/sentinel-01-bench/pullup-3k3.json scenarios/sentinel-01-bench/droop-bor3.json \
  scenarios/sentinel-01-bench/spi-absent-u4.json scenarios/sentinel-01-bench/pullup-2k2.json \
  scenarios/sentinel-01-bench/serial-rx-open-console.json "${flag[@]}" \
  > "$OUT/verify.json" 2> "$OUT/verify.err"
echo "verify rc=$? wall=$(( $(date +%s)-s ))s"
