#!/usr/bin/env python3
"""AET-WO-1916 D2: one mutant per guard a share or a rewritten assertion stands on, each against the test that pins it.

Runs in a `git archive` copy of the commit (`tree`). Each mutant is one exact replacement (its text must occur once),
run, then restored from the pristine bytes before the next. `host` mutants run `cargo test` on the host (tier 1);
`image` mutants run in the pinned image through ~/.cache/aet-1916/img.sh and t2.sh (tier 2, `--features with-tools`).
A mutant is KILLED only when its named test fails and the failure names the expected assertion message; any other
outcome is reported as it is. (The exit status is not the judge: t2.sh exits 0 whatever its tests do.) `PRISTINE` runs
each binary unmutated first: it must pass.

Usage: mutants.py <tree> <log dir> [ID...]      run
       mutants.py --judge <log dir>             judge the logs a run left, without running
"""
import os
import re
import subprocess
import sys

if sys.argv[1] != "--judge":
    tree, logs = sys.argv[1:3]
    only = set(sys.argv[3:])
KIT = os.path.expanduser("~/.cache/aet-1916")
CLI = "crates/aethelgard-cli"

V2K2 = ("image", "verify_2k2_tier2", None)
MUTANTS = [
    # (id, file, from, to, where, binary, test, the assertion message its failure must carry)
    ("MB1", f"{CLI}/src/verify.rs",
     '        report.against = Some("identical");',
     '        report.against = Some("same");',
     *V2K2[:2], "verify_binds_the_reference_and_two_runs_print_the_same_bytes",
     "--against finds the committed tree identical"),
    ("MB2", "crates/aethelgard-eda/src/fab.rs",
     '"Every hole is a plated via, drilled {} mm, with a {} mm pad{class}. ",',
     '"Every hole is a plated via drilled {} mm, with a {} mm pad{class}. ",',
     *V2K2[:2], "verify_fab_out_writes_the_package_its_verdict_names_at_any_path",
     "FAB-NOTES.md states plated via, drilled 0.3 mm"),
    ("MB3", f"{CLI}/src/verify.rs",
     '    report.campaign = Some(out);',
     '    report.campaign = Some(out.into_iter().rev().collect());',
     *V2K2[:2], "verify_binds_the_reference_and_two_runs_print_the_same_bytes",
     "the campaign ran every seeded scenario, in order, each as it declares"),
    ("MB4", f"{CLI}/src/verify.rs",
     '            state_sequence_hash: text("state_sequence_hash"),',
     '            state_sequence_hash: text("run_id"),',
     *V2K2[:2], "verify_binds_the_reference_and_two_runs_print_the_same_bytes",
     "verify's state hash is the one campaign gives the ELF directly"),
    # MB4 is caught by verify's own attribute binding (BindingMismatch) before the test's direct comparison: the
    # wrong reason. MB4b moves only the reported state hash of a run no attribute gate re-reads.
    ("MB4b", f"{CLI}/src/verify.rs",
     '    report.campaign = Some(out);',
     '    report.campaign = Some(out.into_iter().map(|mut r| { if r.scenario == "pullup-3k3" '
     '{ r.state_sequence_hash = r.run_id.clone(); } r }).collect());',
     *V2K2[:2], "verify_binds_the_reference_and_two_runs_print_the_same_bytes",
     "verify's state hash is the one campaign gives the ELF directly"),
    ("MB5", f"{CLI}/src/verify.rs",
     '                g.uid_boundary = None;\n                g.verdict = if attributed.code == 0',
     '                g.uid_boundary = Some(false);\n                g.verdict = if attributed.code == 0',
     *V2K2[:2], "verify_binds_the_reference_and_two_runs_print_the_same_bytes",
     "attribute reports no uid boundary"),
    ("MB6", f"{CLI}/src/verify.rs",
     '                report.gates.push(g);\n                detection(m, r, a).err()',
     '                report.gates.insert(0, g);\n                detection(m, r, a).err()',
     *V2K2[:2], "verify_binds_the_reference_and_two_runs_print_the_same_bytes",
     "every gate runs, in order, and passes"),
    # LP1 (AET-WO-1909): `--against` alone says `same` when no fab flag is given.
    ("ML1", f"{CLI}/src/verify.rs",
     '        report.against = Some("identical");',
     '        report.against = Some(if fab_mode.is_none() { "same" } else { "identical" });',
     "image", "with_tools", "a_board_not_drawn_runs_its_gates_and_is_never_verified",
     "--against with no fab flag finds generate's tree identical"),
    # 1607 F-L: each reader of the shared emit still catches its own defect.
    ("MF1", f"{CLI}/src/main.rs",
     '            specification_sha256: topology.specification_sha256.clone(),\n'
     '            lifecycle: spec.lifecycle_banner().unwrap_or_default(),',
     '            specification_sha256: topology.specification_sha256.clone(),\n'
     '            lifecycle: Vec::new(),',
     "host", "cli_backlog_tier1", "the_emit_verdict_carries_the_banner_and_the_files_do_not",
     "the_emit_verdict_carries_the_banner_and_the_files_do_not"),
    ("MF2", f"{CLI}/src/main.rs",
     '    v.lifecycle = spec.lifecycle_banner().unwrap_or_default();\n    v.specification_sha256 = Some(sha256);',
     '    v.lifecycle = Vec::new();\n    v.specification_sha256 = Some(sha256);',
     "host", "cli_backlog_tier1", "the_drc_and_support_verdicts_carry_the_banner_of_their_spec",
     "the_drc_and_support_verdicts_carry_the_banner_of_their_spec"),
    ("MF3", f"{CLI}/src/main.rs",
     '    // Pinned as `check drc` and `check erc` are (`AET-WO-1607` (1), `1311` F4).\n'
     '    let version = kicad::pinned(version(invocation(\n        run::args(["version"]),\n        "status-version",\n    ))?)?;',
     '    let version = version(invocation(run::args(["version"]), "status-version"))?;',
     "host", "cli_backlog_tier1", "check_support_runs_only_the_pinned_kicad",
     "check_support_runs_only_the_pinned_kicad"),
]


def run(where, binary, test, log):
    with open(log, "w") as f:
        if where == "host":
            env = dict(os.environ, CARGO_TARGET_DIR=f"{KIT}/target-host-mut",
                       PATH=os.path.expanduser("~/.cargo/bin") + ":" + os.environ["PATH"])
            r = subprocess.run(["nice", "-n", "10", "cargo", "test", "-p", "aethelgard-cli", "--test", binary, "--",
                                *([test, "--exact"] if test else [])],
                               cwd=tree, env=env, stdout=f, stderr=subprocess.STDOUT)
        else:
            env = dict(os.environ, TARGET="/cache/target-mut", LABEL=os.path.basename(log)[:-4])
            r = subprocess.run([f"{KIT}/img.sh", "16G", tree, "bash", "/cache/t2.sh", "test", binary,
                                *([test, "--exact"] if test else [])],
                               env=env, stdout=f, stderr=subprocess.STDOUT)
    text = open(log, errors="replace").read()
    inner = f"{KIT}/t2/{os.path.basename(log)[:-4]}.log"
    if where == "image" and os.path.exists(inner):
        text += open(inner, errors="replace").read()
    return r.returncode, text


def main():
    os.makedirs(logs, exist_ok=True)
    results = []
    ids = [m[0] for m in MUTANTS if not only or m[0] in only]
    pristine = {(m[4], m[5], m[6]) for m in MUTANTS if m[0] in ids}
    for where, binary, test in sorted(pristine, key=str):
        code, text = run(where, binary, test, f"{logs}/pristine-{binary}-{test}.log")
        ok = code == 0 and re.search(r"test result: ok\. [1-9]", text)
        results.append(f"PRISTINE {binary} {test}: {'pass' if ok else 'FAIL (exit %d)' % code}")
        print(results[-1], flush=True)
    for mid, path, old, new, where, binary, test, message in MUTANTS:
        if mid not in ids:
            continue
        full = os.path.join(tree, path)
        src = open(full).read()
        n = src.count(old)
        if n != 1:
            results.append(f"{mid}: NOT APPLIED ({n} matches in {path})")
            print(results[-1], flush=True)
            continue
        open(full, "w").write(src.replace(old, new))
        try:
            code, text = run(where, binary, test, f"{logs}/{mid}.log")
        finally:
            open(full, "w").write(src)
        results.append(judge(mid, test, message, text))
        print(results[-1], flush=True)
    open(f"{logs}/result.txt", "w").write("\n".join(results) + "\n")


def judge(mid, test, message, text):
    failed = sorted(set(re.findall(r"^test (\S+) \.\.\. FAILED$", text, re.M)))
    killed = test in failed and message in text
    return (f"{mid}: {'KILLED' if killed else 'SURVIVED or wrong reason'} by {test} "
            f"(failed {failed}; message {'found' if message in text else 'MISSING'})")


def rejudge(logs):
    out = []
    for mid, _, _, _, where, binary, test, message in MUTANTS:
        log = f"{logs}/{mid}.log"
        if not os.path.exists(log):
            continue
        text = open(log, errors="replace").read()
        inner = f"{KIT}/t2/{mid}.log"
        if where == "image" and os.path.exists(inner):
            text += open(inner, errors="replace").read()
        out.append(judge(mid, test, message, text))
    print("\n".join(out))


if __name__ == "__main__":
    if sys.argv[1] == "--judge":
        rejudge(sys.argv[2])
    else:
        main()
