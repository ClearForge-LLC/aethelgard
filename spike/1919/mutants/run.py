#!/usr/bin/env python3
"""AET-WO-1919: one mutant per guard (Rust; the probe's two, in ci/test-ceiling.sh, are recorded in out.txt) this work order adds, each against the test that should kill it.
Each mutant is applied to a copy of its file, the test run, the file restored and touched (cargo keys
on mtime: a restored file keeps an old one). Run from the repository root, under a capped scope.
usage: run.py [NAME...]"""
import os, shutil, subprocess, sys, time

CLI = "crates/aethelgard-cli/src/"
M = [
    ("load-unbounded", CLI + "main.rs", "        (None, _) => run.run_bounded(bounds)?,", "        (None, _) => run.run()?,",
     ["--test", "tools_tier1", "--", "a_load_waiting_at_its_prompt_is_refused_at_the_hang_guard", "--exact"]),
    ("load-kill-unnamed", CLI + "main.rs", "        (None, run::Ended::Bounded(_)) => return Err(renode::hung(\"its load\", bounds.wall)),\n", "",
     ["--test", "tools_tier1", "--", "a_load_waiting_at_its_prompt_is_refused_at_the_hang_guard", "--exact"]),
    ("version-unbounded", CLI + "main.rs",
     '''    let reported =
        invocation(with_config(&["--version"]), "status-version").run_bounded(run::Bounds {
            wall: bounds.wall,
            ..renode::LOAD_BOUNDS
        })?;''', '''    let reported = invocation(with_config(&["--version"]), "status-version").run()?;''',
     ["--test", "tools_tier1", "--", "a_renode_version_that_never_returns_is_refused_at_the_hang_guard", "--exact"]),
    ("version-kill-unnamed", CLI + "main.rs",
     '''    if matches!(reported.ended, run::Ended::Bounded(_)) {
        return Err(renode::hung("its `--version`", bounds.wall));
    }
''', "",
     ["--test", "tools_tier1", "--", "a_renode_version_that_never_returns_is_refused_at_the_hang_guard", "--exact"]),
    ("guard-30s", CLI + "renode.rs", "pub const LOAD_BOUNDS: Bounds = Bounds {\n    wall: BOOT_BOUNDS.wall,",
     "pub const LOAD_BOUNDS: Bounds = Bounds {\n    wall: std::time::Duration::from_secs(30),",
     ["--bin", "aethelgard", "--", "a_load_takes_the_boot_s_hang_guard_and_no_memory_cap"]),
    ("guard-memory-cap", CLI + "renode.rs", "    rss_kib: u64::MAX,\n};", "    rss_kib: BOOT_BOUNDS.rss_kib,\n};",
     ["--bin", "aethelgard", "--", "a_load_takes_the_boot_s_hang_guard_and_no_memory_cap"]),
    ("load-guard-unrecorded", CLI + "main.rs", "    v.wall_bound_ms = (bounds.wall != renode::BOOT_BOUNDS.wall)\n        .then(", "    v.wall_bound_ms = (plan.is_some() && bounds.wall != renode::BOOT_BOUNDS.wall)\n        .then(",
     ["--test", "tools_tier1", "--", "a_load_s_test_guard_is_on_its_verdict", "--exact"]),
    ("flag-firmware-only", CLI + "main.rs",
     '#[arg(long, hide = true, value_name = "MS", value_parser = clap::value_parser!(u64).range(1..=renode::BOOT_BOUNDS.wall.as_secs() * 1000))]',
     '#[arg(long, hide = true, value_name = "MS", requires = "firmware", value_parser = clap::value_parser!(u64).range(1..=renode::BOOT_BOUNDS.wall.as_secs() * 1000))]',
     ["--test", "tools_tier1", "--", "a_load_waiting_at_its_prompt_is_refused_at_the_hang_guard", "--exact"]),
    ("empty-reader", CLI + "renode/foreign.rs", "    }\n    frames_present(&f)?;\n    let port = ports.get(&f.port)", "    }\n    let port = ports.get(&f.port)",
     ["--test", "boot_tier1", "--", "an_input_with_no_frame_is_refused_by_its_field_and_in_replay", "--exact"]),
    ("empty-replay", CLI + "renode/foreign.rs", "    frames_present(&f)?;\n    if f.port != label {", "    if f.port != label {",
     ["--test", "boot_tier1", "--", "an_input_with_no_frame_is_refused_by_its_field_and_in_replay", "--exact"]),
    ("g-whole-tree", CLI + "main.rs", "aethelgard_cli::generate::board_tree(&spec, &topology, &table)", "aethelgard_cli::generate::tree(&spec, &topology, &table)",
     ["--test", "cli_backlog_tier1", "--", "prove_spec_judges_the_crate_of_a_specification_whose_board_does_not_route", "--exact"]),
    ("g-board-tree-drops-the-platform", CLI + "generate.rs", '    for (path, text) in board_files(project, topology, table)? {\n        files.insert(', '    for (path, text) in board_files(project, topology, table)? {\n        if path.ends_with(".repl") {\n            continue;\n        }\n        files.insert(',
     ["--lib", "--", "generate::tests::board_tree_is_tree_s_board", "--exact"]),
    ("x-lane-reports-its-refusal", CLI + "verify.rs",
     "            let out = run_gate(&args);\n",
     "            let out = run_gate(&args);\n            if let Ok(o) = &out {\n                if !matches!(o.status.code(), Some(0 | 1)) {\n                    use std::io::Write;\n                    let _ = std::io::stdout().write_all(&o.stdout);\n                }\n            }\n",
     ["--test", "cli_backlog_tier1", "--", "of_two_refusals_in_two_lanes_the_earlier_in_today_s_order_is_reported", "--exact"]),
    ("x-no-lanes", CLI + "verify.rs", "    start_lanes(p, want, expected);\n", "",
     ["--test", "cli_backlog_tier1", "--", "the_prove_lane_and_build_s_lane_run_beside_each_other", "--exact"]),
    ("doctor-unbounded", CLI + "doctor.rs", "        ran_renode(invocation(", "        ran(invocation(",
     ["--test", "doctor_tier1", "--", "a_renode_that_never_says_its_version_is_killed_at_the_hang_guard", "--exact"]),
    ("x-gate-outlives-verify", CLI + "verify.rs", "        .env(run::PARENT_PID_VAR, std::process::id().to_string())\n        .stdin(Stdio::null())\n        .output()", "        .stdin(Stdio::null())\n        .output()",
     ["--test", "cli_backlog_tier1", "--", "a_verify_killed_outright_leaves_no_lane_gate", "--exact"]),
    ("x-lanes-not-stopped", CLI + "verify.rs", "    stop_lanes();\n    if let Err(code) = gated.and_then(|()| recheck_all()) {", "    if let Err(code) = gated.and_then(|()| recheck_all()) {",
     ["--test", "cli_backlog_tier1", "--", "a_verify_that_stops_early_leaves_no_lane_gate_and_no_late_file", "--exact"]),
    ("x-gate-guardians-killed", CLI + "verify.rs", "            let into = if guardians.contains(&c) {", "            let into = if false && guardians.contains(&c) {",
     ["--test", "cli_backlog_tier1", "--", "a_verify_that_stops_early_leaves_no_lane_gate_and_no_late_file", "--exact"]),
    ("x-no-turn-recheck", CLI + "verify.rs", "    if ahead {\n        recheck(name, &args)?;", "    if false {\n        recheck(name, &args)?;",
     ["--bin", "aethelgard", "--", "verify::tests::a_gate_run_ahead_is_refused_at_its_turn_if_its_file_changed", "--exact"]),
    ("x-no-end-recheck", CLI + "verify.rs", "    if let Err(code) = gated.and_then(|()| recheck_all()) {", "    if let Err(code) = gated {",
     ["--test", "board_tier2", "--features", "with-tools", "--", "verify_refuses_a_board_swapped_after_extract_ran_ahead", "--exact"]),
    ("x-end-recheck-blind", CLI + "verify.rs", "        (have.as_deref() != Some(want.as_str())).then(", "        (false && have.as_deref() != Some(want.as_str())).then(",
     ["--bin", "aethelgard", "--", "verify::tests::a_file_changed_after_every_turn_is_refused_at_the_end", "--exact"]),
]
only = set(sys.argv[1:])
bad = 0
for name, path, old, new, test in M:
    if only and name not in only:
        continue
    orig = open(path).read()
    assert orig.count(old) == 1, (name, "anchor not found once")
    keep = path + ".mutant-orig"
    shutil.copy2(path, keep)
    try:
        open(path, "w").write(orig.replace(old, new))
        t0 = time.time()
        r = subprocess.run(["cargo", "test", "-j", "4", "--locked", "-q", "-p", "aethelgard-cli"] + test,
                           capture_output=True, text=True, timeout=900)
        out = r.stdout + r.stderr
        # A kill is a test that ran and failed; a mutant that does not compile kills nothing.
        compiled = "error[E" not in out and "could not compile" not in out
        killed = compiled and r.returncode != 0 and "test result: FAILED" in out
        state = "killed" if killed else ("DID NOT COMPILE" if not compiled else "SURVIVED")
        print(f"MUTANT {name}: {state} (exit {r.returncode}, {time.time() - t0:.0f} s)", flush=True)
        bad += not killed
    finally:
        shutil.move(keep, path)
        os.utime(path)
print(f"mutants: {len([m for m in M if not only or m[0] in only]) - bad} killed, {bad} survived")
sys.exit(1 if bad else 0)
