#!/bin/bash
# M1: re-run 1211 §4's six Renode scripts (spike/1211/twin/*.resc, unchanged) on main's image pin.
# Output ~/.cache/aet-1211c/runs/twin-rerun.log; compare.py judges it against spike/1211/twin/twin.log.
set -u
K=$(cd "$(dirname "$0")" && pwd)
mkdir -p ~/.cache/aet-1211c/runs ~/.cache/aet-1211c/home
"$K/img.sh" bash -c '
renode --version
for s in gpio timer adc adc23 order high; do
  echo "=== $s ==="
  timeout 300 renode --disable-gui --console --plain -e "include @/repo/spike/1211/twin/$s.resc; quit" </dev/null 2>&1
  echo "=== $s exit $? ==="
done' > ~/.cache/aet-1211c/runs/twin-rerun.log
