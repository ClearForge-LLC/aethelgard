#!/usr/bin/env bash
# AET-WO-1323 §2: each guard this work order adds can fail its test. One
# mutant per guard, planted in a copy of the branch's tree (code only, never
# the tests after `#[cfg(test)]\nmod tests`), run against the test named
# beside it; it must fail that test. The WO's five: the bound's check
# removed (W1), the bound off by one (W2), U1 reverted to bans alone (U1),
# the keepout's containment removed (K1), the stub fix reverted (S1); and
# four more: the field's branch (K2), the early pair judge (S2), an
# escape's end not kept by the prune (S3, the adversarial pass's A1), the
# threads' order (T1).
# Usage: mutants.sh <branch tree (pristine)> <scratch dir> [ids...]
set -uo pipefail
src=$1 scratch=$2; shift 2
ids=${*:-W1 W2 U1 K1 K2 S1 S2 S3 T1}
bad=0
mkdir -p "$scratch"
for m in $ids; do
  rm -rf "$scratch/mut-src"; mkdir -p "$scratch/mut-src"
  rsync -a --exclude target --exclude .git "$src/" "$scratch/mut-src/"
  how=$(python3 -I - "$scratch/mut-src" "$m" <<'PY'
import sys
root, m = sys.argv[1], sys.argv[2]
E = "crates/aethelgard-eda/src/"
lib = "-p aethelgard-eda --lib -- "
M = {
    # D1: the bound never passed.
    "W1": (E + "route.rs", "        self.work > self.limit\n", "        false\n",
           lib + "route::tests::a_routing_one_unit_over_its_bound_is_refused_by_name"),
    # D1: the bound off by one (a routing of exactly the bound refused).
    "W2": (E + "route.rs", "        self.work > self.limit\n", "        self.work >= self.limit\n",
           lib + "route::tests::a_routing_one_unit_over_its_bound_is_refused_by_name"),
    # D2: U1 reverted to bans alone.
    "U1": (E + "route.rs", "                if soft {\n                    self.hard_open(l, n, net)",
           "                if false {\n                    self.hard_open(l, n, net)",
           "-p aethelgard-cli --test stackups_tier1 -- a_shifted_d3_routes_with_its_soft_hops_judged_on_the_hard_maps"),
    # D3: the keepout's containment removed.
    "K1": (E + "route/read.rs", "        if !field(&part).is_some_and(|f| f.contains(k.rect)) && !held(&part.courtyard, k.rect) {",
           "        if false {", lib + "route::tests::a_keepout_outside_its_own_footprint_is_refused_at_read"),
    # D3: the pad field's branch removed (courtyard alone).
    "K2": (E + "route/read.rs", "        if !field(&part).is_some_and(|f| f.contains(k.rect)) && !held(&part.courtyard, k.rect) {",
           "        if !held(&part.courtyard, k.rect) {", lib + "route::tests::a_keepout_outside_its_own_footprint_is_refused_at_read"),
    # D4: the stub fix reverted.
    "S1": (E + "route.rs", "            let cut_p = self.prune(p);\n            let cut_n = self.prune(n);",
           "            let cut_p = false;\n            let cut_n = false;",
           lib + "route::tests::a_pairs_breakout_leaves_no_stub"),
    # D4: the pair judge not run before the other nets.
    "S2": (E + "route.rs", "    if !board.stack.pairs.is_empty() {\n        let laid", "    if false {\n        let laid",
           lib + "route::tests::a_pair_the_judge_refuses_is_refused_by_its_own_name_first"),
    # D4, adversarial A1: an escaped pad's escape end pruned like any end.
    "S3": (E + "route.rs", "                            && !escaped.contains(&n)\n", "",
           lib + "route::tests::a_pair_on_escaped_pads_keeps_its_breakouts"),
    # D5: a thread's pins kept at the wrong index.
    "T1": (E + "supply.rs", "                out[i] = Some(r);", "                out[sinks.len() - 1 - i] = Some(r);",
           lib + "supply::tests::the_pins_bounded_on_threads_are_the_serial_pins_byte_for_byte"),
}
f, old, new, test = M[m]
p = f"{root}/{f}"
s = open(p).read()
cut = s.find("#[cfg(test)]\nmod tests")
code, tests = (s[:cut], s[cut:]) if cut >= 0 else (s, "")
assert code.count(old) == 1, (m, code.count(old))
open(p, "w").write(code.replace(old, new) + tests)
print(test)
PY
  ) || { echo "$m NOT PLANTED"; bad=1; continue; }
  log="$scratch/$m.log"
  # shellcheck disable=SC2086
  (cd "$scratch/mut-src" && nice -n 10 cargo test --target-dir "$scratch/target-mut" -j 6 ${how%% -- *} -- ${how#* -- } > "$log" 2>&1)
  rc=$?
  grep -q "Compiling aethelgard-eda v.* ($scratch/mut-src/crates/aethelgard-eda)" "$log" || { echo "$m: eda not rebuilt from the copy"; bad=1; }
  name=${how##* }
  if [ "$rc" -ne 0 ] && grep -q "test .*$name ... FAILED" "$log"; then
    echo "$m KILLED by $name"
  elif [ "$rc" -ne 0 ] && grep -q "^error\[" "$log"; then
    echo "$m DID NOT BUILD (exit $rc)"; bad=1
  else
    echo "$m SURVIVED (exit $rc)"; bad=1
  fi
done
exit "$bad"
