"""AET-WO-4300i phase B1: one mutant per guard B1 adds or changes (no internal motor pull), each run against
the test that should kill it, by name.
usage (from the repo root, work committed): python3 -I spike/4300i/mutants-b1.py > spike/4300i/out/mutants-b1.tsv
Each mutant is applied, `cargo test -p <crate> --test <t> <filter>` is run with AETHELGARD_BLESS unset (so no
golden is rewritten), and the file is restored with `git checkout -- <file>`. A mutant counts as killed only
when the named test is among the failures; one that does not compile is reported as such."""
import os, subprocess, sys
BOARD = "crates/aethelgard-svd-hal/src/board.rs"
HWDEF = "crates/aethelgard-hwdef/src/lib.rs"
LEDGER = "crates/aethelgard-cli/src/ledger.rs"
PINCHECK = "crates/aethelgard-pincheck/src/lib.rs"
M = [
 ("B1", BOARD, "Self::TimCh1 | Self::TimCh2 | Self::TimCh3 | Self::TimCh4 => (MODE_AF, 0, 0b00),",
  "Self::TimCh1 | Self::TimCh2 | Self::TimCh3 | Self::TimCh4 => (MODE_AF, 0, 0b10),",
  "aethelgard-svd-hal", "motor_outputs", "motor_pins_carry_no_internal_pull_from_the_first_port_init"),
 ("B2", BOARD, ".map(|(pin, p)| (pin.clone(), silicon::implied_pupd(solved, &p.net)))",
  ".map(|(pin, p)| (pin.clone(), if solved.motor_outputs.contains_key(&p.net) { 0b10 } else { silicon::implied_pupd(solved, &p.net) }))",
  "aethelgard-svd-hal", "motor_outputs", "motor_pins_carry_no_internal_pull_from_the_first_port_init"),
 ("B3", HWDEF, "\"{} {} {} PWM({}) {} PUSHPULL FLOATING\",", "\"{} {} {} PWM({}) {} PUSHPULL PULLDOWN\",",
  "aethelgard-hwdef", "generate", "every_motor_line_is_floating"),
 ("B4", LEDGER, "        if row.value != Some(rule) {\n            return Err(LedgerError::RuleDisagrees {",
  "        if false && row.value != Some(rule) {\n            return Err(LedgerError::RuleDisagrees {",
  "aethelgard-cli", "ledger_tier1", "a_motor_output_s_pin_carries_no_internal_pull_by_the_independent_rule"),
 ("B5", LEDGER, "        let rule = silicon::implied_pupd(topology, net);\n        let pin = topology",
  "        let rule = 0b10;\n        let pin = topology",
  "aethelgard-cli", "ledger_tier1", "a_motor_output_s_pin_carries_no_internal_pull_by_the_independent_rule"),
 ("B6", PINCHECK, "let want_pupd = aethelgard_ir::silicon::implied_pupd(topology, net);",
  "let want_pupd = if topology.motor_outputs.contains_key(net) { 0b10 } else { aethelgard_ir::silicon::implied_pupd(topology, net) };",
  "aethelgard-pincheck", "check", "a_motor_pin_with_an_internal_pull_is_refused_as_wrong_pull"),
]
env = dict(os.environ, CARGO_TARGET_DIR=os.path.expanduser("~/.cache/aet-4300i/target-b1"),
           PATH=os.path.expanduser("~/.cargo/bin") + ":" + os.environ["PATH"])
env.pop("AETHELGARD_BLESS", None)
only = set(sys.argv[1:])
print("id\tfile\tmutant\ttest\tresult")
for mid, f, old, new, pkg, tbin, flt in M:
    if only and mid not in only:
        continue
    t = open(f).read()
    assert t.count(old) == 1, (mid, old)
    assert subprocess.run(["git", "diff", "--quiet", "--", f]).returncode == 0, f"{f} is dirty"
    open(f, "w").write(t.replace(old, new))
    try:
        r = subprocess.run(["nice", "-n", "10", "cargo", "test", "-q", "-p", pkg, "--test", tbin, flt],
                           env=env, capture_output=True, text=True)
        out = r.stdout + r.stderr
        if "could not compile" in out:
            res = "NO-COMPILE"
        elif r.returncode != 0 and any(l.startswith("test ") and flt in l and "FAILED" in l
                                       or l.strip() == flt for l in out.splitlines()):
            res = "killed"
        elif r.returncode == 0 and " 0 passed" in out:
            res = "NO-TEST-RAN"
        else:
            res = "SURVIVED" if r.returncode == 0 else f"rc{r.returncode}"
    finally:
        subprocess.run(["git", "checkout", "--", f], check=True)
    oneline = lambda s: s.strip().replace("\n", " ").replace("\t", " ")
    print(f"{mid}\t{f}\t`{oneline(old)}` -> `{oneline(new)}`\t{pkg} {tbin}::{flt}\t{res}", flush=True)
