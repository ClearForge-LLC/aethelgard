#!/bin/bash
# AET-WO-4300 spike: build every probe firmware, then run the named cases (default: all), in one
# container of the pinned image (img.sh: 8g, swap 0, caps read back). Results append to
# spike/4300/out/runs.jsonl; firmware hashes to spike/4300/out/fw.txt.
# Usage (host): spike/4300/run.sh [case ...]
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
python3 "$HERE/plat.py"
python3 "$HERE/cases.py"
mkdir -p "$HERE/out"
CASES=${*:-$(cd "$HERE/cases" && ls *.json | sed "s/\.json$//" | tr "\n" " ")}
exec "$HERE/img.sh" -- bash -c '
set -eu
cd /repo/spike/4300
for v in $(python3 -c "import fw; print(\" \".join(fw.VARIANTS))"); do python3 fw.py "$v"; done > out/fw.txt
for c in '"$CASES"'; do python3 drive.py cases/$c.json out/runs.jsonl 1; echo "done $c"; done'
