#!/usr/bin/env bash
# AET-WO-1610 §1.1, F3 (spi_devices[].orientation): what `generate hwdef`
# says of each committed drawn specification whose ICM-42688-P declares no
# orientation, and of one that declares it.
# Usage: bash spike/1610/f3_orientation.sh <aethelgard> <scratch dir>
set -uo pipefail
bin=$1; out=$2
for f in crates/aethelgard-cli/tests/fixtures/{sentinel-03,hse-crystal-16,sentinel-02-shaped,sentinel-02-own}.json; do
  n=$(basename "$f" .json); rm -f "$out/$n.hwdef.dat"
  "$bin" solve --file "$f" > "$out/$n.topo.json"
  echo "== $n (solve exit $?)"
  "$bin" generate hwdef --topology "$out/$n.topo.json" --out "$out/$n.hwdef.dat" | cut -c1-400
  echo "exit ${PIPESTATUS[0]}"
done
