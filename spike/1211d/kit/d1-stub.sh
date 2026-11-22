#!/usr/bin/env bash
# AET-WO-1211d: D1 drawn past its CVM2 refusal, in a scratch copy only, so the authored DRV8876 symbol
# and PWP0016A footprint meet ERC and DRC. CVM2's `value: None` becomes the literal
# "UNSTATED-SPIKE-ONLY", which no product path carries (S1: nothing invents a value). Usage:
# d1-stub.sh <worktree> <scratch dir>; prints the drawer's binary path.
set -euo pipefail
src=$1 dir=$2
rm -rf "$dir"; mkdir -p "$dir"
git -C "$src" archive HEAD | tar -x -C "$dir"
f="$dir/crates/aethelgard-eda/src/schematic_pin_device.rs"
python3 -I - "$f" <<'PY'
import sys
p = sys.argv[1]
s = open(p).read()
a = 'name: "CVM2",\n            value: None,'
assert s.count(a) == 1, "the CVM2 entry moved"
open(p, "w").write(s.replace(a, 'name: "CVM2",\n            value: Some("UNSTATED-SPIKE-ONLY"),'))
PY
cd "$dir/spike/1211d/draw"
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/.cache/aet-1211d/target-stub}" nice -n 10 cargo build -q --release
echo "${CARGO_TARGET_DIR:-$HOME/.cache/aet-1211d/target-stub}/release/aet-1211d-draw"
