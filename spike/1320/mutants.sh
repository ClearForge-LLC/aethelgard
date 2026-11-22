#!/usr/bin/env bash
# AET-WO-1320 §1.2: each of the five new check() lines can fail its test.
# K1–K5 delete one check each (its `return Err("...")` becomes `()`); W1–W5
# are the reviewer's weakening mutants that survived the first tests (§5).
# Each is planted in a copy of the branch's tree, and eda's arm tests run
# there; the check's own test must fail, named. One copy and one target dir
# serve all in turn.
# Usage: mutants.sh <branch tree (pristine)> <scratch dir> [ids...]
set -uo pipefail
src=$1 scratch=$2; shift 2
ids=${*:-K1 K2 K3 K4 K5 W1 W2 W3 W4 W5}
file=crates/aethelgard-eda/src/schematic_arms.rs
bad=0
mkdir -p "$scratch"
for m in $ids; do
  rm -rf "$scratch/mut-src"; mkdir -p "$scratch/mut-src"
  rsync -a --exclude target --exclude .git "$src/" "$scratch/mut-src/"
  test=$(python3 -I - "$scratch/mut-src/$file" "$m" <<'PY'
import sys
p, m = sys.argv[1], sys.argv[2]
def k(msg, test):
    return (f'return Err("{msg}");', "()", test)
M = {
    "K1": k("two unstated passives on one pin", "two_unstated_passives_on_one_pin_are_refused"),
    "K2": k("an I2C row with no address strap", "an_i2c_row_with_no_address_strap_is_refused"),
    "K3": k("a bus row whose decoupler count is not its part's", "a_decoupler_dropped_or_added_is_refused"),
    "K4": k("one own-net name on two pins of a row", "one_own_net_name_on_two_pins_is_refused"),
    "K5": k("a pin listed twice in one row", "a_pin_listed_twice_is_refused"),
    # share_a_pin without the term for a's second end against b's first
    "W1": ("on(a.one, b.one) || on(a.one, b.two) || on(a.two, b.one) || on(a.two, b.two)",
           "on(a.one, b.one) || on(a.one, b.two) || on(a.two, b.two)",
           "two_unstated_passives_on_one_pin_are_refused"),
    # ... and without the term for both second ends
    "W2": ("on(a.one, b.one) || on(a.one, b.two) || on(a.two, b.one) || on(a.two, b.two)",
           "on(a.one, b.one) || on(a.one, b.two) || on(a.two, b.one)",
           "two_unstated_passives_on_one_pin_are_refused"),
    # the pin-twice and own-name loop compares adjacent pins only
    "W3": ("                j += 1;\n            }\n            i += 1;",
           "                j += 1;\n                break;\n            }\n            i += 1;",
           "a_pin_listed_twice_is_refused one_own_net_name_on_two_pins_is_refused"),
    # an address strap's own net is not a name
    "W4": ("Role::Strap(name) | Own(name) | Addr(Some(name), _) => Some(name),",
           "Role::Strap(name) | Own(name) => Some(name),",
           "one_own_net_name_on_two_pins_is_refused"),
    # an unstated passive beside a stated one on its pin
    "W5": ("                            && b.value.is_none()\n", "",
           "two_unstated_passives_on_one_pin_are_refused"),
}
old, new, test = M[m]
s = open(p).read()
cut = s.index("#[cfg(test)]")
code, tests = s[:cut], s[cut:]
assert code.count(old) == 1, (m, code.count(old))
open(p, "w").write(code.replace(old, new) + tests)
print(test)
PY
  ) || { echo "$m NOT PLANTED"; bad=1; continue; }
  log="$scratch/$m.log"
  (cd "$scratch/mut-src" && cargo test -p aethelgard-eda --lib --target-dir "$scratch/target-mut" -j 6 -- schematic::arms:: > "$log" 2>&1)
  grep -q "Compiling aethelgard-eda v.* ($scratch/mut-src/crates/aethelgard-eda)" "$log" || { echo "$m: eda not rebuilt from the copy"; bad=1; }
  failed=$(sed -n 's/^    schematic::arms::tests::\([a-z0-9_]*\)$/\1/p' "$log" | sort -u | tr '\n' ' ')
  for t in $test; do
    if grep -q "^test schematic::arms::tests::$t \.\.\. FAILED" "$log"; then
      echo "$m KILLED by $t (failed: $failed)"
    else
      echo "$m SURVIVED $t (failed: ${failed:-none}; compile error: $(grep -c '^error' "$log"))"; bad=1
    fi
  done
done
rm -rf "$scratch/mut-src"
exit "$bad"
