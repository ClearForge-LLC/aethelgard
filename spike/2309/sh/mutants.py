#!/usr/bin/env python3
"""AET-WO-2309: one mutant per guard the rule patch adds, each run against the aethelgard-chaos suite
(the suites the guards live in) in the repository copy given, which holds the patch. Each mutant is an
exact, once-only text replacement; the file is restored after. Prints KILLED or SURVIVED, and the
first failing test with its panic line, so a kill is seen to be by the guard's own named refusal.

Usage: mutants.py REPO MUTANTS... (ids below; none means all)
"""
import pathlib
import subprocess
import sys

CAMPAIGN = "crates/aethelgard-chaos/src/campaign.rs"
ARCHIVE = "crates/aethelgard-chaos/src/archive.rs"
MUTANTS = {
    # Rule (1): the writer's bound.
    "w-gone": (ARCHIVE, "        if bytes > MAX_TRACE_BYTES {\n            self.scratch = scratch;\n            return Err(ArchiveError::TraceFull {",
               "        if false {\n            self.scratch = scratch;\n            return Err(ArchiveError::TraceFull {"),
    "w-at-bound": (ARCHIVE, "        if bytes > MAX_TRACE_BYTES {", "        if bytes >= MAX_TRACE_BYTES {"),
    "w-after-fold": (ARCHIVE,
        "        let bytes = self.bytes + scratch.len() as u64;\n        if bytes > MAX_TRACE_BYTES {\n            self.scratch = scratch;\n            return Err(ArchiveError::TraceFull {\n                index,\n                bytes,\n                limit: MAX_TRACE_BYTES,\n            });\n        }\n        self.replayer.apply(event)?;\n",
        "        self.replayer.apply(event)?;\n        let bytes = self.bytes + scratch.len() as u64;\n        if bytes > MAX_TRACE_BYTES {\n            self.scratch = scratch;\n            return Err(ArchiveError::TraceFull {\n                index,\n                bytes,\n                limit: MAX_TRACE_BYTES,\n            });\n        }\n"),
    # Rule (2): exactly one kind 26 per kind 24 in a booted run.
    "m-gone": (CAMPAIGN, "    if booted && witnessed.len() != solved.len() {", "    if false && witnessed.len() != solved.len() {"),
    "m-unbooted-too": (CAMPAIGN, "    if booted && witnessed.len() != solved.len() {", "    if witnessed.len() != solved.len() {"),
}

repo = pathlib.Path(sys.argv[1])
ids = sys.argv[2:] or list(MUTANTS)
for mid in ids:
    rel, old, new = MUTANTS[mid]
    path = repo / rel
    text = path.read_text()
    assert text.count(old) == 1, (mid, "not found once")
    path.write_text(text.replace(old, new))
    try:
        p = subprocess.run(
            ["cargo", "test", "-q", "-p", "aethelgard-chaos", "--no-fail-fast"],
            cwd=repo, capture_output=True, text=True,
            env={**__import__("os").environ, "CARGO_TARGET_DIR": str(repo / "target")},
        )
        out = p.stdout + p.stderr
        failed = [l for l in out.splitlines() if l.endswith("--- FAILED") or "panicked at" in l or l.startswith("  left") or l.startswith(" right") or "assertion" in l]
        verdict = "SURVIVED" if p.returncode == 0 else "KILLED"
        print(f"{mid}: {verdict}")
        for l in failed[:6]:
            print(f"    {l}")
    finally:
        path.write_text(text)
