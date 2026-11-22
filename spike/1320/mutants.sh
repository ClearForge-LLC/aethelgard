#!/usr/bin/env bash
# AET-WO-1320 §1.2: each of the five new check() lines can fail its test.
# Each mutant deletes one check (its `return Err("...")` becomes `()`) in a
# copy of the branch's tree, and eda's arm tests run there; the check's own
# test must fail, named. One copy and one target dir serve all five in turn.
# Usage: mutants.sh <branch tree (pristine)> <scratch dir>
set -uo pipefail
src=$1 scratch=$2
file=crates/aethelgard-eda/src/schematic_arms.rs
declare -A kills=(
  [K1]="two unstated passives on one pin|two_unstated_passives_on_one_pin_are_refused"
  [K2]="an I2C row with no address strap|an_i2c_row_with_no_address_strap_is_refused"
  [K3]="a bus row whose decoupler count is not its part's|a_decoupler_dropped_or_added_is_refused"
  [K4]="one own-net name on two pins of a row|one_own_net_name_on_two_pins_is_refused"
  [K5]="a pin listed twice in one row|a_pin_listed_twice_is_refused"
)
bad=0
mkdir -p "$scratch"
for m in K1 K2 K3 K4 K5; do
  msg=${kills[$m]%%|*} test=${kills[$m]##*|}
  rm -rf "$scratch/mut-src"; mkdir -p "$scratch/mut-src"
  rsync -a --exclude target --exclude .git "$src/" "$scratch/mut-src/"
  python3 -I - "$scratch/mut-src/$file" "$msg" <<'PY' || { echo "$m NOT PLANTED"; bad=1; continue; }
import sys
p, msg = sys.argv[1], sys.argv[2]
s = open(p).read()
code, tests = s[:s.index("#[cfg(test)]")], s[s.index("#[cfg(test)]"):]
old = f'return Err("{msg}");'
assert code.count(old) == 1, (msg, code.count(old))
open(p, "w").write(code.replace(old, "()") + tests)
PY
  log="$scratch/$m.log"
  (cd "$scratch/mut-src" && cargo test -p aethelgard-eda --lib --target-dir "$scratch/target-mut" -j 6 -- schematic::arms:: > "$log" 2>&1)
  grep -q "Compiling aethelgard-eda v.* ($scratch/mut-src/crates/aethelgard-eda)" "$log" || { echo "$m: eda not rebuilt from the copy"; bad=1; }
  failed=$(sed -n 's/^    schematic::arms::tests::\([a-z0-9_]*\)$/\1/p' "$log" | sort -u | tr '\n' ' ')
  if grep -q "^test schematic::arms::tests::$test \.\.\. FAILED" "$log"; then
    echo "$m KILLED by $test (failed: $failed)"
  else
    echo "$m SURVIVED ($msg; failed: ${failed:-none})"; bad=1
  fi
done
exit "$bad"
