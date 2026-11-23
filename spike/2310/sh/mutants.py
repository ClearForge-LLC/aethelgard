#!/usr/bin/env python3
"""AET-WO-2310: one mutant per guard this work order adds or changes, each an exact, once-only text
replacement in the repository copy given, run against the suite the guard lives in; the file is restored after.
Prints KILLED or SURVIVED with the first failing test and its panic line, so a kill is seen to be by the
guard's own named refusal. (2309's harness, re-pointed.)

Usage: mutants.py REPO [MUTANT...]   (run with python3 -I; none means all)
"""
import pathlib, subprocess, sys
C = "crates/aethelgard-chaos/src/campaign.rs"
E = "crates/aethelgard-chaos/src/event.rs"
R = "crates/aethelgard-chaos/src/record.rs"
B = "crates/aethelgard-chaos/src/battery.rs"
CC = "crates/aethelgard-cli/src/campaign.rs"
CA = "crates/aethelgard-cli/src/analog.rs"
CR = "crates/aethelgard-cli/src/renode.rs"
CHAOS = ["-p", "aethelgard-chaos"]
# verify.rs's own droop tests are red until 1607 merges (held, FEEDBACK §9): skipped, so a kill is never theirs.
CLI_UNIT = ["-p", "aethelgard-cli", "--bin", "aethelgard", "--", "--skip", "verify::"]
CLI_T1 = ["-p", "aethelgard-cli", "--test", "campaign_tier1"]
M = {
    # Scenario 6 (chaos parse): the one offset rule, and its two callers (after the reduction pass).
    "offset-grid": (R, "    (-MAX_PLANT_OFFSET_NS..=MAX_PLANT_OFFSET_NS).contains(&v) && v % 10 == 0", "    (-MAX_PLANT_OFFSET_NS..=MAX_PLANT_OFFSET_NS).contains(&v)", CHAOS),
    "offset-range": (R, "    (-MAX_PLANT_OFFSET_NS..=MAX_PLANT_OFFSET_NS).contains(&v) && v % 10 == 0", "    v % 10 == 0", CHAOS),
    "parse-offset": (C, "    if !record::plant_offset_ok(v) {", "    if false {", CHAOS),
    "sag-offset-shape": (B, "        && crate::record::plant_offset_ok(offset)\n", "", CHAOS),
    "version-6": (C, "if (battery_sag.is_some() || droop.is_some()) && version < PLANTED_VERSION {", "if (battery_sag.is_some() || droop.is_some()) && version < 2 {", CHAOS),
    # Replay: the reference and the placed instants.
    "ref-none": (C, "        (true, [Some(word)]) => Ok(Some(*word)),", "        (true, [Some(word)]) => Ok(Some(*word)),\n        (true, [None]) => Ok(Some(0)),", CHAOS),
    "ref-unplanted": (C, "        (false, []) => Ok(None),", "        (false, _) => Ok(None),", CHAOS),
    "droop-instant": (C, "    if record::resolve_instant(word, droop.from_boot_word_ns) != Some(rail.start_ns) {", "    if false {", CHAOS),
    "droop-level": (C, "    if level != Some(rail.bor_level) {", "    if false {", CHAOS),
    "sag-instant": (C, "    if record::resolve_instant(word, sag.from_boot_word_ns) != Some(*start_ns) {", "    if false {", CHAOS),
    "bor-bound": (R, "                } else if droop.bor_level > crate::power::MAX_BOR_LEVEL {", "                } else if false {", CHAOS),
    # Replay: item (4) and item (7).
    "halted": (C, "        if any_severity(HALTED_FINDING) != want || count(HALTED_FINDING) != want {", "        if false {", CHAOS),
    "unwitnessed": (C, "            if count(kind) < unwitnessed {", "            if false {", CHAOS),
    # The header (item 5).
    "hash-rule": (C, "    if hashed != want {", "    if false {", CHAOS),
    "toolchain-named": (C, "    tools.push(RUST_TOOLCHAIN);\n", "", CHAOS),
    "hash-names-a-tool": (E, '            return Err("a tool hash names no tool of the header");', '            return Ok(());', CHAOS),
    "changes-list": (E, "    &TRACE_CHANGES[TRACE_CHANGES.partition_point(|c| c.version <= found)..]", "    &TRACE_CHANGES[..0]", CHAOS),
    # The CLI.
    "library-canonical": (CA, "    if !p.is_absolute() || fs::canonicalize(p).ok().as_deref() != Some(p) {", "    if !p.is_absolute() {", CLI_UNIT),
    "tree-link-bytes": (CC, "                    file(&mut hash, &p, resolved.1)?;\n", "", CLI_UNIT),
    "booted-sleep-stop": (CR, "    let halted = observed.instructions < expected;", "    let halted = observed.instructions < expected;\n    if booted && halted {\n        return Err(untrusted(\"the boot word is set but the core executed fewer\".to_owned()));\n    }", CLI_UNIT),
    "place-past-run": (CC, "        Some(at) if at <= aethelgard_twin::boot::MAX_VIRTUAL_NS => Ok(at),", "        Some(at) => Ok(at),", CLI_T1),
    "library-folded": (CC, "        (Some(hex), true) => hashes.push((rules::NGSPICE.to_owned(), unhex(&hex)?)),", "        (Some(_), true) => {}", CLI_T1),
    "toolchain-absent": (CC, '        Err(s) if s.kind == "ToolMissing" => return absent(),', '        Err(s) if s.kind == "ToolMissingX" => return absent(),', CLI_T1),
}
repo = pathlib.Path(sys.argv[1])
ids = sys.argv[2:] or list(M)
for mid in ids:
    rel, old, new, suite = M[mid]
    path = repo / rel
    text = path.read_text()
    assert text.count(old) == 1, (mid, text.count(old))
    path.write_text(text.replace(old, new))
    try:
        r = subprocess.run(["cargo", "test", "-q", "--locked", "--no-fail-fast", *suite], cwd=repo,
                           capture_output=True, text=True)
        out = r.stdout + r.stderr
        if r.returncode == 0:
            print(f"{mid}: SURVIVED", flush=True)
        else:
            failed = [l for l in out.splitlines() if l.startswith("    ") and "::" not in l and l.strip() and " " not in l.strip()][:3]
            panic = next((l for l in out.splitlines() if "panicked at" in l or "assertion" in l or "error[" in l), "")
            reason = next((l.strip()[:200] for l in out.splitlines() if l.strip().startswith(("left:", "right:", "called `Result"))), "")
            print(f"{mid}: KILLED {failed} {panic[:160]} {reason}", flush=True)
    finally:
        path.write_text(text)
