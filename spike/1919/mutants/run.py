#!/usr/bin/env python3
"""AET-WO-1919: one mutant per guard this work order adds, each against the test that should kill it.
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
     '''    let reported = invocation(with_config(&["--version"]), "status-version").run_bounded(
        run::Bounds {
            wall: bounds.wall,
            ..renode::LOAD_BOUNDS
        },
    )?;''', '''    let reported = invocation(with_config(&["--version"]), "status-version").run()?;''',
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
    ("load-guard-unrecorded", CLI + "main.rs", "    if plan.is_none() && bounds.wall != renode::LOAD_BOUNDS.wall {", "    if false {",
     ["--test", "tools_tier1", "--", "a_load_s_test_guard_is_on_its_verdict", "--exact"]),
    ("flag-firmware-only", CLI + "main.rs",
     '#[arg(long, hide = true, value_name = "MS", value_parser = clap::value_parser!(u64).range(1..=renode::BOOT_BOUNDS.wall.as_secs() * 1000))]',
     '#[arg(long, hide = true, value_name = "MS", requires = "firmware", value_parser = clap::value_parser!(u64).range(1..=renode::BOOT_BOUNDS.wall.as_secs() * 1000))]',
     ["--test", "tools_tier1", "--", "a_load_waiting_at_its_prompt_is_refused_at_the_hang_guard", "--exact"]),
    ("empty-reader", CLI + "renode/foreign.rs", "    }\n    frames_present(&f)?;\n    let port = ports.get(&f.port)", "    }\n    let port = ports.get(&f.port)",
     ["--test", "boot_tier1", "--", "an_input_with_no_frame_is_refused_by_its_field_and_in_replay", "--exact"]),
    ("empty-replay", CLI + "renode/foreign.rs", "    frames_present(&f)?;\n    if f.port != label {", "    if f.port != label {",
     ["--test", "boot_tier1", "--", "an_input_with_no_frame_is_refused_by_its_field_and_in_replay", "--exact"]),
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
