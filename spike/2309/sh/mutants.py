#!/usr/bin/env python3
"""AET-WO-2309: one mutant per guard the rules add, each run against the aethelgard-chaos suite
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

# Rule (3): all read or all unread, and what only a read boot holds (ruled D3).
MUTANTS.update({
    "r-gone": (CAMPAIGN, "    check_read_with_the_boot(records)?;\n", ""),
    "r-11": (CAMPAIGN, "Record::RegisterObserved { reading, .. } => reading.observed.is_some() == read,",
             "Record::RegisterObserved { reading, .. } => true,"),
    "r-12-21-value": (CAMPAIGN, "} => observed.is_some() == read && status.is_some() == read,", "} => status.is_some() == read,"),
    "r-12-21-status": (CAMPAIGN, "} => observed.is_some() == read && status.is_some() == read,", "} => observed.is_some() == read,"),
    "r-13-lines": (CAMPAIGN, "} => read || (observed.is_none() && absent.is_empty()),", "} => read || absent.is_empty(),"),
    "r-13-absent": (CAMPAIGN, "} => read || (observed.is_none() && absent.is_empty()),", "} => read || observed.is_none(),"),
    "r-22-lines": (CAMPAIGN, "} => read || (observed.is_none() && unrouted.is_empty()),", "} => read || unrouted.is_empty(),"),
    "r-22-unrouted": (CAMPAIGN, "} => read || (observed.is_none() && unrouted.is_empty()),", "} => read || observed.is_none(),"),
    "r-renode-gone": (CAMPAIGN, "    if !read && renode_read {", "    if false && renode_read {"),
})
_BLOCK = [
    "            Record::MotorWitness { .. }",
    "                | Record::SerialWitness { .. }",
    "                | Record::JudgeStop { .. }",
    "                | Record::PinGpioWitness { .. }",
    "                | Record::PinAdcWitness { .. }",
    "                | Record::MotorLevelWitness { .. }",
]
for i, kind in enumerate([26, 29, 30, 34, 36, 37]):
    kept = _BLOCK[:i] + _BLOCK[i + 1:]
    if i == 0:
        kept[0] = kept[0].replace("                | ", "            ")
    MUTANTS[f"r-k{kind}"] = (CAMPAIGN, "\n".join(_BLOCK), "\n".join(kept))

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
