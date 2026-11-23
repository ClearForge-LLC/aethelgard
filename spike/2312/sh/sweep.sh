#!/bin/bash
# AET-WO-2312 §1.1: every CLI-made archive (measure.sh's, under RUNS/camp) replayed and attributed by one CLI
# binary ($A), on the host, nice 10; one line per archive and command: its path under RUNS, the exit status and
# the whole output. Usage: A=... sweep.sh RUNS > out.txt
set -u
A=${A:?}; RUNS=$1
for d in $(find "$RUNS/camp" -name manifest.cbor -printf '%h\n' | sort); do
  for c in replay attribute; do
    out=$(nice -n 10 "$A" $c --archive "$d" 2>&1); echo "${d#$RUNS/} $c rc=$? $out"
  done
done
