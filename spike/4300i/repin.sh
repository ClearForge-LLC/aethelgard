#!/usr/bin/env bash
# AET-WO-4300i R4/R5: the PA8 re-pin copy of Sentinel-03, its validate/solve, the diff, the reserved-pin list.
# usage: repin.sh <aethelgard binary> <scratch dir>
set -uo pipefail
AE=$1; W=$2
here=$(cd "$(dirname "$0")" && pwd); root=$(git -C "$here" rev-parse --show-toplevel)
mkdir -p "$W/repin" "$here/out"
python3 -I "$here/repin.py" "$AE" "$root" "$W" "$here/out"
