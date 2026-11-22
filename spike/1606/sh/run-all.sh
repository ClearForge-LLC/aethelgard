#!/bin/bash
# AET-WO-1606 baseline and differential runs (P3, P5): every bridge mode the CLI drives, then doctor
# and the bridge's own modes. usage: BIN=... OUT=... run-all.sh
set -u
R=/repo/spike/1606/sh/run.sh
bash $R s01min examples/sentinel-01-minimal.json scenarios/sentinel-01/pullup-2k2.json scenarios/sentinel-01/pullup-47k.json scenarios/sentinel-01/droop-tolerated.json scenarios/sentinel-01/droop-bor3.json
bash $R s03 crates/aethelgard-cli/tests/fixtures/sentinel-03.json scenarios/sentinel-03/droop-tolerated.json scenarios/sentinel-03/battery-sag-tolerated.json
mkdir -p "$OUT/bridge"
PATH=$BIN:/opt/cargo/bin:/usr/local/bin:/usr/bin:/bin "$BIN/aethelgard" doctor > "$OUT/doctor.json" 2> "$OUT/doctor.err"; echo "doctor exit $?"
cd "$OUT/bridge" || exit 1
"$BIN/aethelgard-bridge" > usage.out 2>&1; echo "bridge usage exit $?"
"$BIN/aethelgard-bridge" --version > version.out 2>&1; echo "bridge --version exit $?"
"$BIN/aethelgard-bridge" /nonexistent.json > refused.out 2>&1; echo "bridge refused exit $?"
"$BIN/aethelgard-bridge" --renode /bin/true > renode.out 2>&1; echo "bridge --renode exit $?"
