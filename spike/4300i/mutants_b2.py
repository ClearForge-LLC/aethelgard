"""AET-WO-4300i phase B2 (eda draws the motor pull): one mutant per guard B2 added, each run against the test
that should kill it. usage (from the repo root, work committed):
  python3 -I spike/4300i/mutants_b2.py > spike/4300i/out/mutants-b2.tsv
Each mutant is applied, `cargo test -p aethelgard-cli --test <t> <filter>` is run, and the file is restored with
`git checkout -- <file>`. A mutant that does not compile is reported as such, never as killed."""
import os, subprocess, sys
SC, SU = "crates/aethelgard-eda/src/schematic.rs", "crates/aethelgard-eda/src/support.rs"
M = [
 ("B1", SC, "    motor_pulls(&mut b)?;\n", "", "emit_tier1", "every_motor_net_carries_its_declared_pull_down"),
 ("B2", SC, ".map(|m| &m.pull.resistor.reference),", ".map(|m| &m.pull.resistor.reference).filter(|_| false),",
  "emit_tier1", "a_motor_pull_declared_below_the_emitters_numbers_keeps_its_reference"),
 ("B3", SU, "Some(_) => Some(text(&comp, \"footprint\")?.to_owned()),", "Some(_) => None,",
  "support_tier1", "the_motor_fixtures_pulls_pass_the_support_gate"),
 ("B4", SU, ".filter(|c| c.symbol.as_deref() == Some(row.symbol.as_str()) && !c.unpopulated)",
  ".filter(|c| !c.unpopulated)", "support_tier1", "a_motor_pull_absent_unpopulated_or_not_a_resistor_is_missing"),
 ("B5", SU, ".filter(|c| c.symbol.as_deref() == Some(row.symbol.as_str()) && !c.unpopulated)",
  ".filter(|c| c.symbol.as_deref() == Some(row.symbol.as_str()))", "support_tier1",
  "a_motor_pull_absent_unpopulated_or_not_a_resistor_is_missing"),
 ("B6", SU, "        if !joins(reference) {", "        if false && !joins(reference) {", "support_tier1",
  "a_motor_pull_off_its_nets_fails_by_name"),
 ("B7", SU, "pins.iter().map(|(p, _)| *p).eq([\"1\", \"2\"]) && {", "pins.len() == 2 && {", "support_tier1",
  "a_motor_pull_off_its_nets_fails_by_name"),
 ("B8", SU, "                have == want\n", "                have.len() == want.len()\n", "support_tier1",
  "a_motor_pull_off_its_nets_fails_by_name"),
 ("B9", SU, "ohms(token) == Some(u64::from(r.ohms)) && part == r.part", "part == r.part", "support_tier1",
  "a_motor_pull_of_another_value_or_part_fails_by_name"),
 ("B10", SU, "ohms(token) == Some(u64::from(r.ohms)) && part == r.part", "ohms(token) == Some(u64::from(r.ohms))",
  "support_tier1", "a_motor_pull_of_another_value_or_part_fails_by_name"),
 ("B11", SU, "        if c.footprint.as_deref() != Some(row.footprint.as_str()) {",
  "        if c.footprint.is_none() {", "support_tier1", "a_motor_pull_on_another_footprint_or_none_fails_by_name"),
 ("B12", SU, "        if c.footprint.as_deref() != Some(row.footprint.as_str()) {",
  "        if c.footprint.is_some() && c.footprint.as_deref() != Some(row.footprint.as_str()) {", "support_tier1",
  "a_motor_pull_on_another_footprint_or_none_fails_by_name"),
 ("B13", SU, "        if !parallel.is_empty() {", "        if false && !parallel.is_empty() {", "support_tier1",
  "a_second_resistor_beside_a_motor_pull_fails_by_name"),
 ("B14", SU, ".filter(|q| q.as_str() != reference && joins(q))", ".filter(|q| joins(q))", "support_tier1",
  "the_motor_fixtures_pulls_pass_the_support_gate"),
 ("B16", SU, ".filter(|q| q.as_str() != reference && joins(q))",
  ".filter(|q| q.as_str() != reference && joins(q) && netlist.components[*q].symbol.as_deref() == Some(row.symbol.as_str()))",
  "support_tier1", "a_second_resistor_beside_a_motor_pull_fails_by_name"),
 ("B17", SU, "        if !extra.is_empty() {", "        if false && !extra.is_empty() {", "support_tier1",
  "another_resistor_on_a_motor_line_fails_by_name"),
 ("B18", SU, "                q.as_str() != reference\n                    && is_resistor(x)", "                is_resistor(x)",
  "support_tier1", "the_motor_fixtures_pulls_pass_the_support_gate"),
 ("B19", SU, "                    && !parallel.contains(q)\n", "", "support_tier1",
  "a_second_resistor_beside_a_motor_pull_fails_by_name"),
 ("B20", SU, "&& netlist.pins_of(q).any(|(_, n)| n == line)", "&& netlist.pins_of(q).next().is_some()",
  "support_tier1", "the_motor_fixtures_pulls_pass_the_support_gate"),
 ("B21", SU, "        s.strip_prefix(\"Device:R\")\n            .is_some_and(|rest| rest.is_empty() || rest.starts_with('_'))\n",
  "        false\n", "support_tier1", "another_resistor_on_a_motor_line_fails_by_name"),
 ("B22", SU, "        .is_some_and(|f| f.starts_with(\"Resistor_\"))", "        .is_some_and(|_| false)", "support_tier1",
  "another_resistor_on_a_motor_line_fails_by_name"),
 ("B15", SU, "    motor_pulls(spec, netlist, &mut fail)?;\n", "", "support_tier1", "a_motor_pull_"),
]
env = dict(os.environ, CARGO_TARGET_DIR=os.path.expanduser("~/.cache/aet-4300i/target-b2"),
           PATH=os.path.expanduser("~/.cargo/bin") + ":" + os.environ["PATH"])
only = set(sys.argv[1:])
print("id\tfile\tmutant\ttest\tresult")
for mid, f, old, new, tbin, flt in M:
    if only and mid not in only:
        continue
    t = open(f).read()
    assert t.count(old) == 1, (mid, old)
    assert subprocess.run(["git", "diff", "--quiet", "--", f]).returncode == 0, f"{f} is dirty"
    open(f, "w").write(t.replace(old, new))
    try:
        r = subprocess.run(["nice", "-n", "10", "cargo", "test", "-q", "-p", "aethelgard-cli", "--test", tbin, flt],
                           env=env, capture_output=True, text=True)
        out = r.stdout + r.stderr
        if "could not compile" in out:
            res = "NO-COMPILE"
        elif r.returncode != 0 and "FAILED" in out:
            res = "killed"
        elif r.returncode == 0 and " 0 passed" in out:
            res = "NO-TEST-RAN"
        else:
            res = "SURVIVED" if r.returncode == 0 else f"rc{r.returncode}"
    finally:
        subprocess.run(["git", "checkout", "--", f], check=True)
    oneline = lambda s: s.strip().replace("\n", " ").replace("\t", " ")
    print(f"{mid}\t{f.replace('crates/aethelgard-eda/', '')}\t`{oneline(old)}` -> `{oneline(new)}`\t{tbin}::{flt}\t{res}", flush=True)
