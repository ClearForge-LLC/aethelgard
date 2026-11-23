#!/bin/bash
# AET-WO-2309 §1.1 sweep B, on the host: every archive measure.sh made (RUNS/camp/<group>/archive/<run>),
# replayed by main's CLI and by the branch's. One line per archive: its path under RUNS, each CLI's
# exit status, and whether the two outputs are byte-identical; any difference prints both outputs.
# Usage: sweep-b.sh RUNS MAIN_CLI BRANCH_CLI
set -u
runs=$1; main=$2; branch=$3
for d in $(find "$runs/camp" -name manifest.cbor -printf '%h\n' | sort); do
  m=$("$main" replay --archive "$d" 2>&1); mr=$?
  b=$("$branch" replay --archive "$d" 2>&1); br=$?
  if [ "$m" = "$b" ] && [ $mr = $br ]; then same=same; else same=DIFFERENT; fi
  echo "${d#$runs/} main=$mr branch=$br $same"
  [ $same = same ] || { echo "  main:   $m"; echo "  branch: $b"; }
done
