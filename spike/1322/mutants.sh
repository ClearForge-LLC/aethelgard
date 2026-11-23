#!/usr/bin/env bash
# AET-WO-1322: each guard this work order adds or changes can fail its test.
# One mutant per guard, planted in a copy of the branch's tree (code only,
# never the tests after `#[cfg(test)]`), run against the test named beside
# it; it must fail that test. 1319's restored M2 is judged by bytes
# separately (spike/1319/harness/mutants.sh ... M2).
# Usage: mutants.sh <branch tree (pristine)> <scratch dir> [ids...]
set -uo pipefail
src=$1 scratch=$2; shift 2
ids=${*:-B1 B2 B3 K1 K2 E1 E2 S1 R1 R2 C1 C2 D1 X1 L1 I1 P1 N1}
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
    # (2) prepare_read: a copy of the carrying chain per child.
    "B1": (E + "board.rs", "            stack.push((child, here.clone()));",
           "            stack.push((child, here.as_ref().map(|w| Rc::new(Within { label: render(Some(&**w)), footprint: w.footprint, parent: None }))));",
           "-p aethelgard-eda --test prepare_read_memory -- a_10_kb_reference_over_100_000_pads_stays_bounded"),
    # (3) board.rs names: the reference unbounded.
    "B2": (E + "board.rs", 'Some(format!("footprint {}", shown(reference)))',
           'Some(format!("footprint {reference}"))', lib + "strict::tests::a_long_name_reaches_no_refusal_whole"),
    # (3) board.rs: the net class unbounded.
    "B3": (E + "board.rs", "                    strict::shown(class)\n", "                    class\n",
           lib + "strict::tests::a_long_name_reaches_no_refusal_whole"),
    # (2) the keepout: any depth admitted.
    "K1": (E + "board.rs", "|| !within.is_some_and(|w| w.footprint && w.parent.is_none())",
           "|| !within.is_some_and(|w| w.footprint)", lib + "board::tests::a_footprints_keepout_is_judged_by_where_it_sits"),
    # (2) the keepout: any named carrier admitted.
    "K2": (E + "board.rs", "|| !within.is_some_and(|w| w.footprint && w.parent.is_none())",
           "|| !within.is_some_and(|w| w.parent.is_none())", lib + "board::tests::a_footprints_keepout_is_judged_by_where_it_sits"),
    # (3) the echo: writing past a cut piece.
    "E1": (E + "strict.rs", "            self.cut = end < s.len();", "            self.cut = false;",
           lib + "strict::tests::an_echo_is_cut_on_a_character_boundary_and_counted"),
    # (3) the echo: the whole, never the head.
    "E2": (E + "strict.rs", "    if c.bytes <= MAX_ECHO_BYTES {", "    if true {",
           lib + "strict::tests::an_echo_is_cut_on_a_character_boundary_and_counted"),
    # (3) shown: everything short as written.
    "S1": (E + "strict.rs", "text.chars().all(|c| c.escape_debug().len() == 1)", "true",
           lib + "strict::tests::a_short_plain_name_is_shown_as_written"),
    # (3) route::read: the reference unbounded.
    "R1": (E + "route/read.rs", "    let named = strict::shown(&reference).into_owned();", "    let named = reference.clone();",
           lib + "strict::tests::a_long_name_reaches_no_refusal_whole"),
    # (3) route::read: a layer echoed whole.
    "R2": (E + "route/read.rs", '        other => Err(format!("{what} on layer {}", strict::echo(&other))),',
           '        other => Err(format!("{what} on layer {other:?}")),', lib + "strict::tests::a_long_name_reaches_no_refusal_whole"),
    # (3) MAX_ITEMS: keepouts not counted.
    "C1": (E + "route/read.rs", "        .map(|p| p.courtyard.len() + p.keepouts.len())", "        .map(|p| p.courtyard.len())",
           lib + "route::tests::a_board_past_the_item_cap_is_refused"),
    # (3) MAX_ITEMS: courtyard edges not counted.
    "C2": (E + "route/read.rs", "        .map(|p| p.courtyard.len() + p.keepouts.len())", "        .map(|p| p.keepouts.len())",
           lib + "route::tests::a_board_past_the_item_cap_is_refused"),
    # (6) a doubled DECOUPLERS entry accepted.
    "D1": (E + "schematic_arms.rs", '                return Err("a part with two DECOUPLERS entries");', "                ()",
           lib + "schematic::arms::tests::a_part_with_two_decoupler_entries_is_refused"),
    # (1) the crossing count: only an empty bus refused.
    "X1": (E + "schematic_power.rs", "        [device] => Ok(device),", "        [device, ..] => Ok(device),",
           lib + "schematic::power::tests::a_crossing_is_refused_without_one_device_or_a_translator"),
    # (1) lines: any row a translator.
    "L1": (E + "schematic_power.rs", '        return Err(refuse(reference, format!("`{part}` is not a translator")));',
           "        return Ok(Vec::new());", lib + "schematic::power::tests::a_crossing_is_refused_without_one_device_or_a_translator"),
    # (4) the IPROPI text as before.
    "I1": (E + "schematic_arms.rs", "; `validate` admits that tie (`straps`: `{{\\\"tie\\\": \\\"GND\\\"}}`) only on the pin the row's trip senses (AET-WO-1217 F2)",
           ", and a pin device cannot declare that tie", lib + "schematic::arms::tests::an_undeclared_current_output_names_the_tie_validate_admits"),
    # (7) an exhausted run reported as its net.
    "P1": (E + "pcb.rs", "        (Some((reference, pads)), _) => EmitError::PadEscapeExhausted { reference, pads },",
           "        (Some(_), _) => EmitError::Unroutable { net: e.net, why: e.why },",
           lib + "pcb::tests::a_run_with_no_escape_keeps_its_shape"),
    # (5) eda's claim loop claiming nothing (names::references bypassed).
    "N1": (E + "schematic.rs", "    for r in aethelgard_ir::names::references(spec) {",
           "    for r in aethelgard_ir::names::references(spec).into_iter().take(0) {",
           "-p aethelgard-cli --test hot_plug_tier1 -- a_hot_plug_reference_the_emitter_refuses_is_refused_at_validate"),
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
