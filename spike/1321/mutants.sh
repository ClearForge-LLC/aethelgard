#!/usr/bin/env bash
# AET-WO-1321: the guards this work order adds can fail their tests.
# R1 deletes the reverse check, R2 accepts a pin row as an entry's row, and
# R3 skips the last entry; each must fail
# schematic::arms::tests::a_decoupler_entry_with_no_bus_row_is_refused.
# N1 has declared_bulk pass no note; it must fail
# pin_devices_drawn_tier1::d1_is_drawn_with_its_declared_bulk (its bytes are
# judged separately by spike/1319/harness/run.sh and compare.sh).
# Each is planted in a copy of the branch's tree; one copy and one target dir
# serve all in turn.
# Usage: mutants.sh <branch tree (pristine)> <scratch dir> [ids...]
set -uo pipefail
src=$1 scratch=$2; shift 2
ids=${*:-R1 R2 R3 N1}
bad=0
mkdir -p "$scratch"
for m in $ids; do
  rm -rf "$scratch/mut-src"; mkdir -p "$scratch/mut-src"
  rsync -a --exclude target --exclude .git "$src/" "$scratch/mut-src/"
  how=$(python3 -I - "$scratch/mut-src" "$m" <<'PY'
import sys
root, m = sys.argv[1], sys.argv[2]
arms = "crates/aethelgard-eda/src/schematic_arms.rs"
eda = "-p aethelgard-eda --lib -- schematic::arms::tests::a_decoupler_entry_with_no_bus_row_is_refused"
cli = "-p aethelgard-cli --test pin_devices_drawn_tier1 -- d1_is_drawn_with_its_declared_bulk"
M = {
    "R1": ('            return Err("a DECOUPLERS entry with no bus row");', "            ()", eda),
    "R2": ("if same(rows[r].part, part) && !matches!(rows[r].wiring, Wiring::Pins { .. }) {",
           "if same(rows[r].part, part) {", eda),
    "R3": ("    while e < counts.len() {", "    while e + 1 < counts.len() {", eda),
    "N1": ('''        Some(format!(
            "{name} declared, {document} states no value: {}",
            bulk.source
        )),
    )''', "        None,\n    )", cli),
}
old, new, test = M[m]
p = f"{root}/{arms}"
s = open(p).read()
cut = s.index("#[cfg(test)]")
code, tests = s[:cut], s[cut:]
assert code.count(old) == 1, (m, code.count(old))
open(p, "w").write(code.replace(old, new) + tests)
print(test)
PY
  ) || { echo "$m NOT PLANTED"; bad=1; continue; }
  log="$scratch/$m.log"
  # shellcheck disable=SC2086
  (cd "$scratch/mut-src" && cargo test --target-dir "$scratch/target-mut" -j 6 ${how%% -- *} -- ${how#* -- } > "$log" 2>&1)
  rc=$?
  grep -q "Compiling aethelgard-eda v.* ($scratch/mut-src/crates/aethelgard-eda)" "$log" || { echo "$m: eda not rebuilt from the copy"; bad=1; }
  name=${how##* }
  if [ "$rc" -ne 0 ] && grep -q "test .*$name ... FAILED" "$log"; then
    echo "$m KILLED by $name"
  else
    echo "$m SURVIVED (exit $rc)"; bad=1
  fi
done
exit "$bad"
