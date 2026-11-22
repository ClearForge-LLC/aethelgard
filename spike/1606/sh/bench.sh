#!/bin/bash
# AET-WO-1606 §2 (a fab/ byte that moves is a stop): CI's bench-package step, inside the image, with the
# CLI and bridge in $BIN: verify --against-fab on the committed package, or, with FABOUT=<dir>, --fab-out.
set -u
cd /repo
if [ -n "${FABOUT:-}" ]; then flag=(--fab-out "$FABOUT"); else flag=(--against-fab fab/sentinel-01-bench/package); fi
s=$(date +%s)
"${BIN:?}/aethelgard" verify --spec examples/sentinel-01.json \
  --scenarios scenarios/sentinel-01-bench/pullup-3k3.json scenarios/sentinel-01-bench/droop-bor3.json \
  scenarios/sentinel-01-bench/spi-absent-u4.json scenarios/sentinel-01-bench/pullup-2k2.json \
  scenarios/sentinel-01-bench/serial-rx-open-console.json "${flag[@]}" \
  > "${OUT:?}/bench-verify.json" 2> "$OUT/bench-verify.err"
echo "verify rc=$? wall=$(( $(date +%s)-s ))s"
