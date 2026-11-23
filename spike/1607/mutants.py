#!/usr/bin/env python3
"""AET-WO-1607 §1.2: one mutant per guard this work order adds, each against the test that pins it.

Runs in a `git archive` copy of the head (`tree`), with CARGO_TARGET_DIR outside it and HOME a
private directory holding `.cargo/config.toml` (CARGO_HOME and RUSTUP_HOME the real ones): the
acceptance condition of WO §2. Each mutant is one exact replacement (its text must occur once),
built and run, then the file is restored from the pristine copy before the next. A mutant is
KILLED only when the named test is among the failures; any other failure is reported as such.

Usage: mutants.py <tree> <target dir> <fake home> <log dir>
"""
import os
import re
import shutil
import subprocess
import sys

tree, target, home, logs = sys.argv[1:5]
CLI = "crates/aethelgard-cli"
MUTANTS = [
    # (id, file, from, to, test target, test name)
    ("M1", "src/main.rs",
     '    let version = kicad::pinned(version(invocation(\n        run::args(["version"]),\n        "status-version",\n    ))?)?;\n    let net',
     '    let version = version(invocation(run::args(["version"]), "status-version"))?;\n    let net',
     "cli_backlog_tier1", "check_support_runs_only_the_pinned_kicad"),
    ("M2", "tests/common/mod.rs",
     'let run = PathBuf::from(format!("/run/user/{}", euid.as_raw()));',
     'let run = PathBuf::from(std::env::var_os("HOME").unwrap()).join(".cache");',
     "cli_backlog_tier1", "the_private_scratch_is_outside_home"),
    ("M2b", "tests/common/mod.rs",
     '        assert!(\n            !base.starts_with(&home),',
     '        assert!(\n            home.as_os_str().is_empty() || true || !base.starts_with(&home),',
     None, None),  # applied together with M2: see PAIRS
    ("M3", "tests/fab_tier1.rs",
     '    let dir = root()\n        .join("target/aet-tests")',
     '    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))',
     "fab_tier1", "a_rail_or_bus_no_fault_kind_reaches_is_named"),
    ("M4", "tests/readme_status.rs",
     '        if about.is_empty() {\n            return Some(format!("does not state P{n}"));\n        }',
     '        if about.is_empty() {\n            continue;\n        }',
     "readme_status", "a_stale_status_line_or_an_unmarked_phase_is_caught"),
    ("M5", "tests/readme_status.rs",
     '        if !passed && done {',
     '        if !passed && done && false {',
     "readme_status", "a_stale_status_line_or_an_unmarked_phase_is_caught"),
    ("M5b", "tests/readme_status.rs",
     '        if let Some(n) = named.iter().find(|n| !phases.contains_key(n)) {',
     '        if let Some(n) = named.iter().find(|n| !phases.contains_key(n) && false) {',
     "readme_status", "a_stale_status_line_or_an_unmarked_phase_is_caught"),
    ("M6", "src/verify.rs",
     '|g: &Gate| !drawn && g.verdict == "not_applicable" && BOARD_GATES.contains(&g.gate);',
     '|g: &Gate| g.verdict == "not_applicable" && BOARD_GATES.contains(&g.gate);',
     "--bin", "a_board_gate_is_not_applicable_only_on_a_board_not_drawn"),
    ("M6b", "src/verify.rs",
     '|g: &Gate| !drawn && g.verdict == "not_applicable" && BOARD_GATES.contains(&g.gate);',
     '|g: &Gate| !drawn && g.verdict == "not_applicable";',
     "--bin", "a_board_gate_is_not_applicable_only_on_a_board_not_drawn"),
    ("M7", "src/verify.rs",
     '    if fab_mode.is_some()\n        && let Err(r) = crate::draws_a_board(&spec, command)',
     '    if let Err(r) = crate::draws_a_board(&spec, command)',
     "cli_backlog_tier1", "verify_of_a_board_not_drawn_runs_past_its_board_gates"),
    ("M8", "src/verify.rs",
     '    if want.drawn {\n        board_gates(report, p, want)?;',
     '    if want.drawn || true {\n        board_gates(report, p, want)?;',
     "cli_backlog_tier1", "verify_of_a_board_not_drawn_runs_past_its_board_gates"),
    ("M9", "src/verify.rs",
     '        verdict: "not_applicable",\n        reason: Some(NO_BOARD),',
     '        verdict: "pass",\n        reason: Some(NO_BOARD),',
     "--bin", "a_board_gate_is_not_applicable_only_on_a_board_not_drawn"),
    ("M10", "src/main.rs",
     '            lifecycle: spec.lifecycle_banner().unwrap_or_default(),\n            schematic,',
     '            lifecycle: Vec::new(),\n            schematic,',
     "cli_backlog_tier1", "the_emit_verdict_carries_the_banner_and_the_files_do_not"),
    ("M11", "src/main.rs",
     '            .map(|(spec, _)| spec.lifecycle_banner().unwrap_or_default())',
     '            .map(|(_, _)| Vec::new())',
     "cli_backlog_tier1", "the_drc_and_support_verdicts_carry_the_banner_of_their_spec"),
    ("M12", "src/main.rs",
     '    v.lifecycle = spec.lifecycle_banner().unwrap_or_default();\n    v.specification_sha256 = Some(sha256);',
     '    v.specification_sha256 = Some(sha256);',
     "cli_backlog_tier1", "the_drc_and_support_verdicts_carry_the_banner_of_their_spec"),
    ("M13", "src/main.rs",
     '    v.lifecycle = lifecycle.unwrap_or_default();\n    v.harness = Some(harness.to_owned());',
     '    v.harness = Some(harness.to_owned());',
     "cli_backlog_tier1", "the_prove_verdict_carries_the_banner_of_its_spec"),
    ("M14", "src/pinout.rs",
     '        lifecycle: topology.lifecycle_banner().unwrap_or_default(),\n        topology_sha256',
     '        lifecycle: Vec::new(),\n        topology_sha256',
     "cli_backlog_tier1", "the_pinout_verdict_carries_the_banner_of_its_topology"),
    ("M16", "tests/tools_tier1.rs",
     '        .env("TMPDIR", &tmp.0)\n        .env("PATH", path)',
     '        .env("PATH", path)',
     "tools_tier1", "prove_pass_reports_the_harness"),
    ("M17", "tests/generate_tier1.rs",
     '        .env("TMPDIR", &tmp.0)\n        .stdin(Stdio::null());',
     '        .stdin(Stdio::null());',
     "generate_tier1", "build_and_prove_refuse_what_cargo_would_run_before_any_tool"),
    ("M15", "src/doctor.rs",
     '        (&KICAD, &KICAD),',
     '        (&KICAD, &["check drc", "check erc", "verify"]),',
     "doctor_tier1", "a_tool_off_its_pin_reports_its_version_and_blocks_what_pins_it"),
]
# A mutant that needs another applied with it, so the first guard does not hide the second.
PAIRS = {"M2": ["M2b"]}
SKIP = {"M2b"}

env = dict(os.environ)
env.update({
    "CARGO_TARGET_DIR": target,
    "HOME": home,
    "CARGO_HOME": os.path.expanduser("~/.cargo"),
    "RUSTUP_HOME": os.path.expanduser("~/.rustup"),
    "PATH": os.path.expanduser("~/.cargo/bin") + ":" + os.environ["PATH"],
})
assert os.path.isfile(os.path.join(home, ".cargo/config.toml")), "the fake HOME holds no config"
assert not os.path.realpath(target).startswith(os.path.realpath(tree) + "/")
os.makedirs(logs, exist_ok=True)
by_id = {m[0]: m for m in MUTANTS}


def path(rel):
    return os.path.join(tree, CLI, rel)


def apply(m):
    p = path(m[1])
    text = open(p).read()
    assert text.count(m[2]) == 1, f"{m[0]}: its text occurs {text.count(m[2])} times in {m[1]}"
    open(p, "w").write(text.replace(m[2], m[3]))


def run(args, log):
    with open(log, "w") as f:
        r = subprocess.run(["nice", "-n", "10", "cargo", "test", "-p", "aethelgard-cli", *args],
                           cwd=tree, env=env, stdout=f, stderr=subprocess.STDOUT)
    return r.returncode


for m in MUTANTS:
    n = open(path(m[1])).read().count(m[2])
    assert n == 1, f"{m[0]}: its text occurs {n} times in {m[1]}"
if os.environ.get("CHECK_ONLY"):
    sys.exit(0)

results = []
for m in MUTANTS:
    if m[0] in SKIP:
        continue
    mid, rel, _, _, target_test, name = m
    touched = [rel] + [by_id[p][1] for p in PAIRS.get(mid, [])]
    saved = {r: open(path(r)).read() for r in set(touched)}
    try:
        apply(m)
        for p in PAIRS.get(mid, []):
            apply(by_id[p])
        sel = ["--bin", "aethelgard"] if target_test == "--bin" else ["--test", target_test]
        log = os.path.join(logs, f"{mid}.log")
        code = run([*sel, "--", "--test-threads", "4"], log)
        out = open(log).read()
        failed = re.findall(r"^test (\S+) \.\.\. FAILED$", out, re.M)
        built = "error[E" not in out and "could not compile" not in out
        if code == 0:
            verdict = "SURVIVED"
        elif not built:
            verdict = "NOT BUILT"
        elif any(f.endswith(name) for f in failed):
            verdict = "KILLED"
        else:
            verdict = "FAILED ELSEWHERE"
        results.append((mid, verdict, name, failed))
        print(f"{mid} {verdict} by {name}; failures: {failed}", flush=True)
    finally:
        for r, t in saved.items():
            open(path(r), "w").write(t)

# The pristine tree passes the same selections, so every kill above is the mutant's.
for target_test in sorted({m[4] for m in MUTANTS if m[4]}):
    sel = ["--bin", "aethelgard"] if target_test == "--bin" else ["--test", target_test]
    code = run([*sel, "--", "--test-threads", "4"], os.path.join(logs, f"pristine-{target_test.strip('-')}.log"))
    print(f"pristine {target_test}: exit {code}", flush=True)
killed = sum(1 for r in results if r[1] == "KILLED")
print(f"{killed} of {len(results)} killed")
