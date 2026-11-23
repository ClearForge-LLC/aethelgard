"""AET-WO-4300j Phase A: one mutant per guard this build adds or changes in `aethelgard-ir`, each run against the test
that should kill it. Each mutant is applied as exact text edits (each `old` must occur once), the named test target is
run with the filter, and every file is restored byte for byte, whatever happens. A mutant that does not compile is
reported as such, never as killed.

usage: python3 -I spike/4300j/mutants_a.py <repo root> > spike/4300j/mutants-a.md
(cargo runs under the caller's cap: run this script itself inside `systemd-run --user --scope -p MemoryMax=16G ...`)"""
import os, subprocess, sys

root = sys.argv[1]
OFF = "crates/aethelgard-ir/src/offboard.rs"
PP = "crates/aethelgard-ir/src/power_parts.rs"
M = "crates/aethelgard-ir/src/motor.rs"

# (id, guard, mutation, [(file, old, new)], test target, test filter)
MUTANTS = [
    ("A1", "bound floor side (`MotorPullOutsideBound`)", "`<` to `<=` at the floor",
     [(OFF, "if r.ohms < w.floor_ohms ||", "if r.ohms <= w.floor_ohms ||")],
     "offboard", "a_pull_outside_the_bound_is_refused_at_both_edges"),
    ("A2", "bound ceiling side (`MotorPullOutsideBound`)", "`>` to `>=` at the ceiling",
     [(OFF, "|| r.ohms > w.ceiling_ohms {", "|| r.ohms >= w.ceiling_ohms {")],
     "offboard", "a_pull_outside_the_bound_is_refused_at_both_edges"),
    ("A3", "floor rounded up", "floor rounded down",
     [(OFF, "let floor = (v_hi * rpu + den - 1) / den;", "let floor = v_hi * rpu / den;")],
     "offboard", "a_pull_outside_the_bound_is_refused_at_both_edges"),
    ("A4", "floor rounded up, exactly", "floor rounded up one step too far on a whole-ohm floor",
     [(OFF, "let floor = (v_hi * rpu + den - 1) / den;", "let floor = (v_hi * rpu + den) / den;")],
     "motor", "the_bound_moves_with_the_rail"),
    ("A5", "ceiling rounded down", "ceiling rounded up",
     [(OFF, "let ceiling = (low - margin) * n * (1000 - tol) / i_na;",
       "let ceiling = ((low - margin) * n * (1000 - tol) + i_na - 1) / i_na;")],
     "offboard", "a_pull_outside_the_bound_is_refused_at_both_edges"),
    ("A6", "cross-rail: the peer's VIH at VDD max (D1 (a))", "same-rail: VIH at VDD min",
     [(OFF, "let v_hi = (high + margin) * n * (1000 + tol);", "let v_hi = (high + margin) * n * (1000 - tol);")],
     "offboard", "a_pull_outside_the_bound_is_refused_at_both_edges"),
    ("A7", "the pull-up at VDD min", "the pull-up at nominal",
     [(OFF, "let vdd_lo = n * (1000 - tol) * 1000;", "let vdd_lo = n * 1000 * 1000;")],
     "offboard", "a_pull_outside_the_bound_is_refused_at_both_edges"),
    ("A8", "I = leakage + peer input current", "the peer's current dropped",
     [(OFF, "i128::from(b.leakage_max_nanoamps.value) + i128::from(b.peer_input_current_nanoamps.value);",
       "i128::from(b.leakage_max_nanoamps.value) + 0 * i128::from(b.peer_input_current_nanoamps.value);")],
     "offboard", "a_pull_outside_the_bound_is_refused_at_both_edges"),
    ("A9", "margin at the floor", "margin dropped at the floor",
     [(OFF, "let v_hi = (high + margin) * n * (1000 + tol);", "let v_hi = high * n * (1000 + tol);")],
     "offboard", "a_pull_outside_the_bound_is_refused_at_both_edges"),
    ("A10", "margin at the ceiling", "margin dropped at the ceiling",
     [(OFF, "let ceiling = (low - margin) * n * (1000 - tol) / i_na;", "let ceiling = low * n * (1000 - tol) / i_na;")],
     "offboard", "a_pull_outside_the_bound_is_refused_at_both_edges"),
    ("A11", "`MotorPullFloorUnreachable` (denominator <= 0)", "check removed",
     [(OFF, "if den <= 0 {", "if den <= 0 && false {")],
     "motor", "an_empty_or_unreachable_bound_admits_nothing"),
    ("A12", "`MotorPullBoundEmpty` (floor > ceiling)", "check removed",
     [(OFF, "Ok(floor_ohms) if floor_ohms <= ceiling_ohms =>", "Ok(floor_ohms) =>")],
     "motor", "an_empty_or_unreachable_bound_admits_nothing"),
    ("A13", "link part a jumper row (`PowerPartUnknown`)", "a resistor row admitted as the link",
     [(OFF, "            Some(power_parts::Kind::Jumper(_))\n        ) {",
       "            Some(power_parts::Kind::Jumper(_) | power_parts::Kind::Resistor(_))\n        ) {")],
     "offboard", "a_link_whose_part_is_not_a_jumper_row_is_refused"),
    ("A14", "link reference in `names::references`", "link references left out",
     [("crates/aethelgard-ir/src/names.rs", ".map(|m| m.pull.link.reference.as_str()),",
       ".map(|m| m.pull.link.reference.as_str())\n                .filter(|_| false),")],
     "offboard", "a_pull_reference_another_part_holds_is_refused"),
    ("A15", "`<label>_PD` in `names::own_nets`", "pull nets left out",
     [("crates/aethelgard-ir/src/names.rs", "out.extend(spec.motor_outputs.iter().map(|m| motor_pull_net(&m.label)));",
       "out.extend(spec.motor_outputs.iter().map(|m| motor_pull_net(&m.label)).filter(|_| false));")],
     "motor", "a_net_named_motor1_pd_is_refused_in_any_case"),
    ("A16", "link reference an identifier (step 2)", "check removed",
     [("crates/aethelgard-ir/src/lib.rs", "identifier(IdentifierKind::Reference, &m.pull.link.reference, || {",
       "identifier(IdentifierKind::Reference, &r.reference, || {")],
     "offboard", "a_blank_source_or_reference_is_refused_by_the_house_rules"),
    ("A17", "`fitment` required, no default (`INV-06`)", "`#[serde(default)]` on `fitment`",
     [(M, "    pub link: MotorPullLink,\n    pub fitment: PullFitment,",
       "    pub link: MotorPullLink,\n    #[serde(default)]\n    pub fitment: PullFitment,"),
      (M, "#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]\n#[serde(rename_all = \"lowercase\")]\npub enum PullFitment {\n    Fitted,",
       "#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]\n#[serde(rename_all = \"lowercase\")]\npub enum PullFitment {\n    #[default]\n    Fitted,")],
     "offboard", "a_missing_null_or_unknown_fitment_or_link_is_refused_at_parse"),
    ("A18", "`implied_pupd` pull-down when `open`", "`open` gives 0",
     [("crates/aethelgard-ir/src/silicon.rs", "crate::motor::PullFitment::Open => PUPDR_PULL_DOWN,",
       "crate::motor::PullFitment::Open => 0,")],
     "offboard", "a_motor_net_is_pulled_internally_only_when_open"),
    ("A19", "`implied_pupd` none when `fitted`", "`fitted` gives pull-down",
     [("crates/aethelgard-ir/src/silicon.rs", "crate::motor::PullFitment::Fitted => 0,",
       "crate::motor::PullFitment::Fitted => PUPDR_PULL_DOWN,")],
     "offboard", "a_motor_net_is_pulled_internally_only_when_open"),
    ("A20", "solve copies the fitment", "every output solved `fitted`",
     [("crates/aethelgard-ir/src/solve.rs", "fitment: p.fitment,", "fitment: crate::motor::PullFitment::Fitted,")],
     "offboard", "solve_copies_the_pull_with_its_bound"),
    ("A21", "solve copies the window", "floor one ohm low",
     [("crates/aethelgard-ir/src/solve.rs", "floor_ohms: w.floor_ohms,", "floor_ohms: w.floor_ohms - 1,")],
     "offboard", "solve_copies_the_pull_with_its_bound"),
    ("A22", "reader: margin < input low", "check dropped",
     [(PP, "low >= high || margin >= low ||", "low >= high ||")],
     "offboard", "the_mcu_row_holds_the_bound_inputs_and_refuses_them_malformed"),
    ("A23", "reader: input high + margin < 1000", "check dropped",
     [(PP, "|| high + margin >= 1000 {", "{")],
     "offboard", "the_mcu_row_holds_the_bound_inputs_and_refuses_them_malformed"),
    ("A24", "reader: input low < input high", "check dropped",
     [(PP, "if high >= 1000 || low >= high ||", "if high >= 1000 ||")],
     "offboard", "the_mcu_row_holds_the_bound_inputs_and_refuses_them_malformed"),
    ("A25", "reader: a ruled value above zero", "check dropped",
     [(PP, "        if c.value <= 0 {", "        if c.value < 0 {")],
     "offboard", "the_mcu_row_holds_the_bound_inputs_and_refuses_them_malformed"),
    ("A26", "reader: jumper figures above zero", "`positive` to `limit`",
     [(PP, "max_milliohms: r.positive(j.max_milliohms,", "max_milliohms: r.limit(j.max_milliohms,")],
     "offboard", "the_jumper_row_is_held_and_refuses_it_malformed"),
    ("A27", "reader: a ruled value's basis is text", "basis check dropped",
     [(PP, '        text(&c.basis, &format!("{field}.basis"), 256)?;\n', "")],
     "offboard", "the_mcu_row_holds_the_bound_inputs_and_refuses_them_malformed"),
    ("A28", "reader: jumper current above zero", "`positive` to `limit`",
     [(PP, "max_milliamps: r.positive(j.max_milliamps,", "max_milliamps: r.limit(j.max_milliamps,")],
     "offboard", "the_jumper_row_is_held_and_refuses_it_malformed"),
    ("A29", "link part an identifier (step 2)", "check removed",
     [("crates/aethelgard-ir/src/lib.rs", 'plain(&m.pull.link.part, || format!("{at}.link.part"))?;',
       'plain(&r.part, || format!("{at}.link.part"))?;')],
     "offboard", "a_blank_source_or_reference_is_refused_by_the_house_rules"),
]

env = dict(os.environ)
print("# AET-WO-4300j Phase A: mutants of the guards this build adds or changes in `aethelgard-ir`\n")
print("Run by `spike/4300j/mutants_a.py`: each mutant applied as exact text, its killing test run alone, every file "
      "restored byte for byte.\n")
print("| id | guard | mutation | killing test | killed |")
print("|---|---|---|---|---|")
ONLY = set(sys.argv[2:])
for mid, guard, mutation, edits, target, test in [m for m in MUTANTS if not ONLY or m[0] in ONLY]:
    saved = {}
    try:
        for f, old, new in edits:
            p = os.path.join(root, f)
            if p not in saved:
                saved[p] = open(p).read()
            t = open(p).read()
            assert t.count(old) == 1, (mid, f, t.count(old))
            open(p, "w").write(t.replace(old, new))
        r = subprocess.run(["cargo", "test", "-q", "-p", "aethelgard-ir", "--test", target, test],
                           cwd=root, env=env, capture_output=True, text=True)
        out = r.stdout + r.stderr
        if "could not compile" in out:
            verdict = "no: does not compile"
        elif r.returncode != 0 and "FAILED" in out:
            verdict = "yes"
        elif r.returncode == 0 and " 1 passed" in out:
            verdict = "**no: survived**"
        else:
            verdict = f"unclear (rc {r.returncode})"
    finally:
        for p, t in saved.items():
            open(p, "w").write(t)
    print(f"| {mid} | {guard} | {mutation} | `{target}::{test}` | {verdict} |", flush=True)
