"""AET-WO-4300j Phase B1: one mutant per guard or branch this build adds or changes in svd-hal, hwdef, pincheck's
tests' rule and the CLI's ledger, each run against the test that should kill it. Each mutant is applied as exact text
edits (each `old` must occur once), the named test target is run with the filter, and every file is restored byte for
byte, whatever happens. A mutant that does not compile is reported as such, never as killed.

Run in a copy of the tree (B2 edits the same worktree at the same time), with its own target directory:
  rsync -a --exclude target --exclude .git <repo>/ <copy>/
  CARGO_TARGET_DIR=<copy-target> python3 -I spike/4300j/mutants_b1.py <copy> > spike/4300j/mutants-b1.md
(cargo runs under the caller's cap: run this script itself inside `systemd-run --user --scope -p MemoryMax=16G ...`)"""
import os, subprocess, sys

root = sys.argv[1]
B = "crates/aethelgard-svd-hal/src/board.rs"
H = "crates/aethelgard-hwdef/src/lib.rs"
L = "crates/aethelgard-cli/src/ledger.rs"
S = "crates/aethelgard-ir/src/silicon.rs"
SVD_PIN = ("aethelgard-svd-hal", "motor_outputs",
           "each_motor_pin_s_pupdr_follows_its_fitment_in_the_generator_and_the_oracle")
HW_PULL = ("aethelgard-hwdef", "generate", "each_motor_line_s_pull_follows_its_fitment")
HW_COMMENT = ("aethelgard-hwdef", "generate", "a_comment_line_above_each_motor_line_names_its_fitment_and_bound")
LEDGER = ("aethelgard-cli", "ledger_tier1", "a_motor_output_s_pin_pull_follows_its_fitment_by_the_independent_rule")
PIN_OPEN = ("aethelgard-pincheck", "check", "an_open_motor_pin_with_no_internal_pull_is_refused_as_wrong_pull")

# (id, guard, mutation, [(file, old, new)], (package, test target, test filter))
MUTANTS = [
    ("B1", "svd-hal generator: `open` writes `10` (`motor_pull`)", "`open` gives `00`",
     [(B, "        motor::PullFitment::Open => 0b10,", "        motor::PullFitment::Open => 0b00,")], SVD_PIN),
    ("B2", "svd-hal generator: `fitted` writes `00` (`motor_pull`)", "`fitted` gives `10`",
     [(B, "        motor::PullFitment::Fitted => 0b00,", "        motor::PullFitment::Fitted => 0b10,")], SVD_PIN),
    ("B3", "svd-hal generator: a motor pin's pull comes from its fitment", "the timer role's pull instead (4300i)",
     [(B, "(role, None, motor_pull(m.pull.fitment))", "(role, None, role.policy().2)")], SVD_PIN),
    ("B4", "svd-hal oracle: a motor pin's expected field from `implied_pupd`", "the oracle holds every motor pin `00`",
     [(B, ".map(|(pin, p)| (pin.clone(), silicon::implied_pupd(solved, &p.net)))",
       ".map(|(pin, p)| (pin.clone(), if solved.motor_outputs.contains_key(&p.net) { 0 } else { silicon::implied_pupd(solved, &p.net) }))")],
     SVD_PIN),
    ("B5", "the oracle's rule: `implied_pupd` `open` gives `10` (Phase A's, read by the oracle)", "`open` gives `00`",
     [(S, "crate::motor::PullFitment::Open => PUPDR_PULL_DOWN,", "crate::motor::PullFitment::Open => 0,")], SVD_PIN),
    ("B6", "pincheck `WrongPull` on an `open` motor pin (by `implied_pupd`)", "as B5",
     [(S, "crate::motor::PullFitment::Open => PUPDR_PULL_DOWN,", "crate::motor::PullFitment::Open => 0,")], PIN_OPEN),
    ("B7", "hwdef pull token: `open` writes `PULLDOWN`", "`open` writes `FLOATING`",
     [(H, '                (\n                    "PULLDOWN",', '                (\n                    "FLOATING",')], HW_PULL),
    ("B8", "hwdef pull token: `fitted` writes `FLOATING`", "`fitted` writes `PULLDOWN`",
     [(H, '                (\n                    "FLOATING",', '                (\n                    "PULLDOWN",')], HW_PULL),
    ("B9", "hwdef comment line: `fitted` names the link", "the link left out",
     [(H, '"{net} pull: fitted, {} {} ohm to GND via link {}; bound {}..{} ohm, checked for BLHeli passthrough idle with it fitted, the ESC threshold a stand-in (AET-WO-4300j R1)",\n                    p.resistor, p.ohms, p.link, p.floor_ohms, p.ceiling_ohms',
       '"{net} pull: fitted, {} {} ohm to GND via link {}; bound {}..{} ohm, checked for BLHeli passthrough idle with it fitted, the ESC threshold a stand-in (AET-WO-4300j R1)",\n                    p.resistor, p.ohms, p.resistor, p.floor_ohms, p.ceiling_ohms')],
     HW_COMMENT),
    ("B10", "hwdef comment line: `open` names the bound", "ceiling written as the floor",
     [(H, '"{net} pull: open (link {} not fitted), PULLDOWN in firmware; {} {} ohm, bound {}..{} ohm (AET-WO-4300j)",\n                    p.link, p.resistor, p.ohms, p.floor_ohms, p.ceiling_ohms',
       '"{net} pull: open (link {} not fitted), PULLDOWN in firmware; {} {} ohm, bound {}..{} ohm (AET-WO-4300j)",\n                    p.link, p.resistor, p.ohms, p.floor_ohms, p.floor_ohms')],
     HW_COMMENT),
    ("B11", "hwdef comment line: one above each motor line", "no comment line",
     [(H, "        self.0.push((format!(\"# {}\", text.as_ref()), String::new()));",
       "        let _ = text;")], HW_COMMENT),
    ("B12", "hwdef render: a comment line sets no width", "comment lines widen every line",
     [(H, "            .filter(|(_, source)| !source.is_empty())\n", "")], HW_COMMENT),
    ("B13", "ledger rule: `MOTOR_PUPD_RULE` names `open`, `10`", "the `open` clause dropped",
     [(L, "; open, `10`, the firmware's internal pull-down, the link DNP;", ";")], LEDGER),
    ("B14", "ledger note: an `open` row names its fitment and link `DNP`", "an `open` row written as `fitted`",
     [(L, "            PullFitment::Open => format!(\n                \"open: the firmware's internal pull-down, `10`, from the first GPIO init, link {} DNP, so the declared resistor {} holds nothing (AET-WO-4300j D6)\",\n                m.pull.link, m.pull.resistor\n            ),",
       "            PullFitment::Open => format!(\n                \"fitted: no internal pull, `00`, from the first GPIO init, its pull-down the declared external resistor {} through link {} (motor_outputs[].pull, AET-WO-4300i D0) from power-up\",\n                m.pull.resistor, m.pull.link\n            ),")],
     LEDGER),
    ("B15", "ledger rule value: an `open` pin's rule is `10` (by `implied_pupd`)", "as B5",
     [(S, "crate::motor::PullFitment::Open => PUPDR_PULL_DOWN,", "crate::motor::PullFitment::Open => 0,")], LEDGER),
]

env = dict(os.environ)
print("# AET-WO-4300j Phase B1: mutants of the guards this build adds or changes in svd-hal, hwdef and the ledger\n")
print("Run by `spike/4300j/mutants_b1.py` in a copy of the tree (B2 edits the worktree at the same time): each mutant "
      "applied as exact text, its killing test run alone, every file restored byte for byte. B5, B6 and B15 mutate "
      "Phase A's `silicon::implied_pupd` (not edited by B1) to show that the oracle, `check pinout` and the ledger "
      "each read it.\n")
print("| id | guard | mutation | killing test | killed |")
print("|---|---|---|---|---|")
ONLY = set(sys.argv[2:])
for mid, guard, mutation, edits, (pkg, target, test) in [m for m in MUTANTS if not ONLY or m[0] in ONLY]:
    saved = {}
    try:
        for f, old, new in edits:
            p = os.path.join(root, f)
            if p not in saved:
                saved[p] = open(p).read()
            t = open(p).read()
            assert t.count(old) == 1, (mid, f, t.count(old))
            open(p, "w").write(t.replace(old, new))
        r = subprocess.run(["cargo", "test", "-q", "-p", pkg, "--test", target, test, "--", "--exact"],
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
    print(f"| {mid} | {guard} | {mutation} | `{pkg} {target}::{test}` | {verdict} |", flush=True)
