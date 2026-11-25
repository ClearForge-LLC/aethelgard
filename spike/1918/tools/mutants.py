#!/usr/bin/env python3
"""AET-WO-1918: one mutant per guard this WO adds or changes, each against the suite it lives in (CLAUDE.md:
mutation testing sized to the diff). Each mutant is applied to the worktree, its command run, and the file
restored and touched (a restored file keeps an old mtime, and cargo would reuse the mutant's build).
A mutant is killed when its command fails. Prints one line per mutant; exits 1 if any survives.

usage: python3 -I spike/1918/tools/mutants.py [NAME-PART...]   (from the worktree root, in a capped scope)
"""
import os
import subprocess
import sys

CLI = "cargo test -q -p aethelgard-cli"
M = [
    ("split at the bound, not past it", "crates/aethelgard-svd-hal/src/board.rs",
     "if units.len() <= FINAL_STATEMENTS_MAX {", "if units.len() < FINAL_STATEMENTS_MAX {",
     "cargo test -q -p aethelgard-svd-hal --lib final"),
    ("a binding not glued to its check", "crates/aethelgard-svd-hal/src/board.rs",
     'if ends && cur.contains("kani::assert(") {', "if ends {",
     "cargo test -q -p aethelgard-svd-hal --lib final"),
    ("a wall kill a finding again", "crates/aethelgard-cli/src/renode.rs",
     "        Bound::Wall => Err(Stop::tool_error(", "        Bound::Wall if false => Err(Stop::tool_error(",
     None),
    ("replay accepts the hang guard's finding", "crates/aethelgard-chaos/src/campaign.rs",
     "if any_severity(HANG_GUARD_FINDING) != 0 {", "if any_severity(HANG_GUARD_FINDING) > 1 {",
     "cargo test -q -p aethelgard-chaos --test campaign"),
    ("a hidden argument offered over MCP", "crates/aethelgard-cli/src/mcp.rs",
     "if arg.is_hide_set()\n", "if false\n",
     f"{CLI} --bin aethelgard simulate_keeps_its_name"),
    ("the test's guard not on the verdict", "crates/aethelgard-cli/src/main.rs",
     "v.wall_bound_ms = (bounds.wall != renode::BOOT_BOUNDS.wall)", "v.wall_bound_ms = (bounds.wall == renode::BOOT_BOUNDS.wall)",
     f"{CLI} --test boot_tier1 hang_guard"),
    ("the test's guard ignored", "crates/aethelgard-cli/src/main.rs",
     "wall: wall_bound_ms\n", "wall: None::<u64>\n",
     f"{CLI} --test boot_tier1 hang_guard"),
    ("the undisturbed boot's wall kill not refused", "crates/aethelgard-cli/src/renode.rs",
     "crate::run::Ended::Bounded(bound) => return bounded(out, bound, wall).map(|_| None),",
     "crate::run::Ended::Bounded(_) => return Ok(None),",
     f"{CLI} --bin aethelgard undisturbed_boot_killed"),
    ("a peak at the ceiling refused", "ci/ceiling.sh",
     "awk -F'\\t' -v c=\"$ceiling\" '$1 > c { print $2 }'", "awk -F'\\t' -v c=\"$ceiling\" '$1 >= c { print $2 }'",
     "bash ci/test-ceiling.sh"),
    ("a peak over the ceiling passed", "ci/ceiling.sh",
     "awk -F'\\t' -v c=\"$ceiling\" '$1 > c { print $2 }'", "awk -F'\\t' -v c=\"$ceiling\" '$1 > c + 1 { print $2 }'",
     "bash ci/test-ceiling.sh"),
    ("the probe drops what lives at its stop", "ci/probe.py",
     "for pid in [p for p in live if stopping or p not in seen]:", "for pid in [p for p in live if p not in seen]:",
     "bash ci/test-ceiling.sh"),
    ("a bench job not requiring a peak", ".github/workflows/ci.yml",
     "run: bash ci/probe-step.sh judge --require-peak\n", "run: bash ci/probe-step.sh judge\n",
     "bash ci/test-ceiling.sh"),
    ("against_fab_tier2 not kept in one shard", "ci/colocate.tsv",
     "tests/against_fab_tier2.rs\t", "tests/against_fab_tier2_x.rs\t",
     f"{CLI} --test fab_tier1 against_fab_tier2_writes"),
]
# The wall-kill mutant's command: the hang tests in boot_tier1 and the renode unit test.
WALL = f"{CLI} --test boot_tier1 never_returns && {CLI} --bin aethelgard undisturbed_boot_killed"

survived = 0
only = sys.argv[1:]
for name, path, old, new, cmd in M:
    if only and not any(o in name for o in only):
        continue
    cmd = cmd or WALL
    text = open(path).read()
    n = text.count(old)
    if n < 1:
        print(f"NOT APPLIED\t{name}\t({path}: the text is gone)")
        survived += 1
        continue
    # A mutant changes the first occurrence (the ci.yml one: one bench job).
    open(path, "w").write(text.replace(old, new, 1))
    try:
        r = subprocess.run(cmd, shell=True, capture_output=True, text=True)
    finally:
        open(path, "w").write(text)
        os.utime(path)
    killed = r.returncode != 0
    survived += not killed
    print(f"{'killed' if killed else 'SURVIVED'}\t{name}\t({cmd})")
sys.exit(1 if survived else 0)
