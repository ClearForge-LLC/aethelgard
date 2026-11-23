#!/bin/bash
# AET-WO-2310 §1.1 sweep B, second half, inside the image: every archive measure.sh made, replayed by main's CLI.
# One line per archive: its path under RUNS, the exit status, and the replay's whole output.
set -u
RUNS=${RUNS:-/cache/runs}; A=${A:-/cache/target-main/debug/aethelgard}
for d in $(find "$RUNS/camp" -name manifest.cbor -printf '%h\n' | sort); do
  out=$("$A" replay --archive "$d" 2>&1); echo "${d#$RUNS/} rc=$? $out"
done
