"""AET-WO-1609: one mutant per guard this work order adds, each against the suite it lives in,
in a source copy (~/.cache/aet-1609/mut) with its own target. A mutant that does not compile is
reported as such, never as a kill."""
import os, subprocess, sys
ROOT = os.path.expanduser("~/.cache/aet-1609/mut")
ENV = dict(os.environ, CARGO_TARGET_DIR=os.path.expanduser("~/.cache/aet-1609/mut-target"),
           PATH=os.path.expanduser("~/.cargo/bin") + ":" + os.environ["PATH"])
CLI = "crates/aethelgard-cli/src/"
M = [
 ("M1 supply: an empty drop_limits counts as on", CLI+"extract.rs", ".is_some_and(|l| !l.is_empty());", ".is_some();", ["--bin", "aethelgard", "the_supply_part"]),
 ("M2 oscillator: any mcu.hse is on", CLI+"extract.rs", "let crystal = matches!(spec.mcu.hse, Some(aethelgard_ir::clock::Hse::Crystal(_)));", "let crystal = spec.mcu.hse.is_some();", ["--bin", "aethelgard", "the_oscillator_part"]),
 ("M3 pairs: always on", CLI+"extract.rs", "            !spec.diff_pairs.is_empty(),\n            conductors as u64,", "            true,\n            conductors as u64,", ["--bin", "aethelgard", "the_pair_part"]),
 ("M4 lines: counted as 1", CLI+"extract.rs", 'Ran::judged("extract.lines", lines as u64)', 'Ran::judged("extract.lines", 1)', ["--bin", "aethelgard", "every_part_has_a_row"]),
 ("M5 drc.power: always on", CLI+"main.rs", '            spec.power.is_some(),\n            loops.unwrap_or(0) as u64,', '            true,\n            loops.unwrap_or(0) as u64,', ["--bin", "aethelgard", "drc_parts_are"]),
 ("M6 drc: no --spec reads diff_pairs", CLI+"main.rs", '        return vec![\n            Ran::off("drc.pairs", "--spec"),', '        return vec![\n            Ran::off("drc.pairs", "diff_pairs"),', ["--bin", "aethelgard", "drc_parts_are"]),
 ("M7 hse_clock: judged before the boot word", CLI+"main.rs", '(true, false) => Ran::not_reached(', '(true, false) => Ran::judged("simulate.hse_clock", 1), (true, true) if false => Ran::not_reached(', ["--bin", "aethelgard", "the_hse_clock_row"]),
 ("M8 table: an unknown gate passes", CLI+"verify.rs", ".find(|g| !GATES.iter().any(|(n, _)| *n == g.gate))", ".find(|_| false)", ["--bin", "aethelgard", "a_planted_gate"]),
 ("M9 table: a Never gate absent is off", CLI+"verify.rs", '                Absent::Never => {\n                    return Err(format!("gate `{name}` was not pushed and has no row"));\n                }', '                Absent::Never => Row::off(name, "never"),', ["--bin", "aethelgard", "a_planted_gate"]),
 ("M10 table: not_applicable is judged", CLI+"verify.rs", '("not_applicable", _) => Row::off(name, NOT_APPLICABLE_KEY),', '("not_applicable", _) => Row::judged(name, 0),', ["--bin", "aethelgard", "off_and_not_reached"]),
 ("M11 table: after a failed build, gpio is off", CLI+"verify.rs", '                    Some(s) if s.verdict == "not_run" => {', '                    Some(s) if s.verdict == "never" => {', ["--bin", "aethelgard", "off_and_not_reached"]),
 ("M12 table: not_run without a reason is given one", CLI+"verify.rs", '.ok_or_else(|| format!("gate `{name}` is not_run with no reason"))?,', '.unwrap_or(NO_ELF),', ["--bin", "aethelgard", "off_and_not_reached"]),
 ("M13 table: parts dropped", CLI+"verify.rs", "            out.extend(parts(g)?);", "            let _ = parts(g)?;", ["--bin", "aethelgard", "the_table_lists_every_gate"]),
 ("M14 parts: an extra field passes", CLI+"verify.rs", "if part.is_empty() || fields.len() != 2 {", "if part.is_empty() {", ["--bin", "aethelgard", "a_malformed_part_row"]),
 ("M15 parts: another gate's prefix passes", CLI+"verify.rs", "gate.strip_prefix(g.gate).and_then(|p| p.strip_prefix('.')),", "gate.split_once('.').map(|(_, p)| p),", ["--bin", "aethelgard", "a_malformed_part_row"]),
 ("M16 conclude: no table", CLI+"verify.rs", "    report.ran = ran_table(&report.gates).map_err(|e| {", "    let _ = ran_table(&report.gates).map_err(|e| {", ["--bin", "aethelgard", "the_exit_code_is_the_verdicts"]),
 ("M17 not_reached: no reason", CLI+"verify.rs", "        reason: Some(why),\n        ..not_run(name)", "        reason: { let _ = why; None },\n        ..not_run(name)", ["--bin", "aethelgard", "off_and_not_reached"]),
 ("M18 elf: Thumb bit kept", CLI+"elf.rs", "let address = u64::from(u32_at(bytes, at + 4) & !1);", "let address = u64::from(u32_at(bytes, at + 4));", ["--bin", "aethelgard", "a_function_symbol"]),
 ("M19 elf: any symbol type", CLI+"elf.rs", "if info & 0xF != STT_FUNC || start >= strings.len() {", "if start >= strings.len() {", ["--bin", "aethelgard", "a_function_symbol"]),
 ("M20 elf: two addresses, the first", CLI+"elf.rs", "        [one] => *one,\n        many => {", "        [one, ..] => *one,\n        many => {", ["--bin", "aethelgard", "a_function_symbol"]),
 ("M21 elf: outside executable code passes", CLI+"elf.rs", "    if !runs {\n        return Err(malformed(format!(", "    if !runs && false {\n        return Err(malformed(format!(", ["--bin", "aethelgard", "a_function_symbol"]),
 ("M22 foreign: any address is an entry", CLI+"renode/foreign.rs", "(u64::from_str_radix(pc.trim(), 16).ok()? == address)", "(u64::from_str_radix(pc.trim(), 16).ok().is_some() || address == 0)", ["--bin", "aethelgard", "a_panic_entry"]),
 ("M23 foreign: any source is the CPU", CLI+"renode/foreign.rs", '        .strip_prefix("[WARNING] cpu: ")?\n        .strip_prefix(PANIC_MARK)?;', '        .strip_prefix("[WARNING] ")?\n        .split_once(": ")?.1\n        .strip_prefix(PANIC_MARK)?;', ["--bin", "aethelgard", "a_panic_entry"]),
 ("M24 foreign: no hook", CLI+"renode/foreign.rs", "    c.push(panic_hook(panic));", "    let _ = panic_hook(panic);", ["--bin", "aethelgard", "a_foreign_boot_hooks_the_panic"]),
 ("M25 extract: supply counted as 0", CLI+"extract.rs", "        out.len(),\n        supplies.len(),\n", "        out.len(),\n        0,\n", ["--test", "supply_tier1", "the_four_layer_board_bounds"]),
 ("M26 extract: conductors counted as 0", CLI+"extract.rs", '.map(|p| p["conductors"].as_array().map_or(0, Vec::len))', '.map(|p| p["conductors"].as_array().map_or(0, |_| 0))', ["--test", "supply_tier1", "a_declared_pair"]),
 ("M27 against: no table", CLI+"verify.rs", "            report.ran = none_ran(AGAINST_DIFFERS);\n", "", ["--test", "generate_tier1", ""]),
 ("M28 parts: a parted gate stating none passes", CLI+"verify.rs", "            Value::Null if ran && PARTED.contains(&g.gate) => {", "            Value::Null if false && ran && PARTED.contains(&g.gate) => {", ["--bin", "aethelgard", "a_malformed_part_row"]),
 ("M29 panic: read before the begin marker only", CLI+"renode/foreign.rs", "    let entered: Vec<Option<u64>> = lines\n", "    let entered: Vec<Option<u64>> = run_log\n", ["--test", "boot_tier1", "a_foreign_boot_that_enters_the_panic"]),
 ("M30 panic: not kept past the budget", CLI+"renode/foreign.rs", "        MODEL_LOAD_ERROR,\n        PANIC_MARK,\n", "        MODEL_LOAD_ERROR,\n", ["--bin", "aethelgard", "a_panic_entry"]),
 ("M31 panic: entries never stated", CLI+"renode/foreign.rs", "    report.panic.entered_ns = Some(entered);", "    let _ = entered;", ["--test", "boot_tier1", "a_foreign_boot_that_enters_the_panic"]),
 ("M32 hse_clock: a bounded run blamed on the boot word", CLI+"main.rs", "        (true, None) => Ran::not_reached(\n            \"simulate.hse_clock\",\n            \"the run ended at a bound, so its end was never read\",", "        (true, None) => Ran::not_reached(\n            \"simulate.hse_clock\",\n            \"the firmware never wrote its boot word\",", ["--bin", "aethelgard", "the_hse_clock_row"]),
]
only = sys.argv[1:]
for name, f, old, new, test in M:
    if only and name.split()[0] not in only:
        continue
    path = os.path.join(ROOT, f)
    src = open(path).read()
    assert src.count(old) == 1, f"{name}: {src.count(old)} sites"
    open(path, "w").write(src.replace(old, new))
    try:
        r = subprocess.run(["cargo", "test", "-q", "-p", "aethelgard-cli", *test[:2], "--", test[2]],
                           cwd=ROOT, env=ENV, capture_output=True, text=True)
        out = r.stdout + r.stderr
        if "error[E" in out or "could not compile" in out:
            verdict = "DOES NOT COMPILE"
        elif r.returncode != 0 and "test result: FAILED" in out:
            verdict = "killed"
        elif r.returncode == 0 and "running 0 tests" not in out.split("test result")[0][-200:]:
            verdict = "SURVIVED"
        else:
            verdict = f"? rc={r.returncode}"
        print(f"{name}: {verdict}", flush=True)
    finally:
        open(path, "w").write(src)
