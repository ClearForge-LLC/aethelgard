#!/usr/bin/env bash
# AET-WO-1319 §2: the harnesses can still fail. Each mutant is planted in a
# copy of the branch's tree, measured by run.sh, and compared with the base
# by compare.sh, which must report a difference. One tree and one target dir
# serve all three in turn, so each rebuild is incremental.
# Usage: mutants.sh <branch tree (pristine)> <scratch dir> <base out dir> <repo>
set -uo pipefail
src=$1 scratch=$2 base=$3 repo=$4
here=$(cd "$(dirname "$0")" && pwd)
file=crates/aethelgard-eda/src/schematic_arms.rs
for m in M1 M2 M3; do
  rm -rf "$scratch/mut-src"; mkdir -p "$scratch/mut-src"
  cp -a "$src/." "$scratch/mut-src/"
  python3 -I - "$scratch/mut-src/$file" "$m" <<'PY'
import sys
p, m = sys.argv[1], sys.argv[2]
s = open(p).read()
cut = s.index("#[cfg(test)]")
code, tests = s[:cut], s[cut:]
if m == "M1":   # W25Q64JVxIQ's WP and HOLD rows swapped
    old = '        ("3", Role::Strap("WP")),\n        ("7", Role::Strap("HOLD")),\n'
    new = '        ("7", Role::Strap("HOLD")),\n        ("3", Role::Strap("WP")),\n'
elif m == "M2": # BMP581's second 100 nF decoupler left out
    old = '    passives: &[Passive::Decouple("100n"), Passive::Decouple("100n")],\n'
    new = '    passives: &[Passive::Decouple("100n")],\n'
else:           # DRV8833's CVCP and CVINT drawn in the other order
    a = code.index('        Passive::Required(Required {\n            name: "CVCP",\n            value: Some("10n")')
    b = code.index('        Passive::Required(Required {\n            name: "CVINT"')
    e = code.index('        }),\n', b) + len('        }),\n')
    old = code[a:e]
    new = code[b:e] + code[a:b]
assert code.count(old) == 1, m
open(p, "w").write(code.replace(old, new) + tests)
print("planted", m)
PY
  bash "$here/run.sh" "$scratch/mut-src" "$scratch/target-mut" "$scratch/m/$m" "$repo" > "$scratch/run-$m.log" 2>&1
  echo "$m run exit $?"
  if bash "$here/compare.sh" "$base" "$scratch/m/$m" "$repo" > "$scratch/compare-$m.txt" 2>&1; then
    echo "$m SURVIVED"
  else
    echo "$m KILLED"
  fi
  grep -E "identical|DIFFERS|differs" "$scratch/compare-$m.txt" | sed 's/^/  /' | head -20
done
