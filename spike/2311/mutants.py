"""AET-WO-2311 mutants: one per guard this WO adds, each run against the suites the guard lives in.
A mutant is killed only by a failing test (named); a compile error is reported as such, not a kill.
python3 mutants.py [NAME...]   (run from the worktree; reverts each file after)"""
import subprocess, sys, os, re

W = "/home/forge/aethelgard-2311"
TWIN = "crates/aethelgard-twin/src/serial.rs"
FOR = "crates/aethelgard-cli/src/renode/foreign.rs"
T_TWIN = ["-p", "aethelgard-twin", "--lib", "serial::tests"]
T_FOR = ["-p", "aethelgard-cli", "--bin", "aethelgard", "renode::"]
T_B1 = ["-p", "aethelgard-cli", "--test", "boot_tier1", "foreign"]

M = [
    # (name, file, old, new, suites)
    ("swap-unjudged", TWIN, '_ => [Expect::Clear, line, line]', '_ => [Expect::Unjudged, line, line]', [T_TWIN, T_FOR, T_B1]),
    ("txinv-rxinv-masks-exchanged", TWIN, '("TXINV", txinv, t)', '("TXINV", rxinv, t)', [T_FOR]),
    ("txinv-rxinv-kinds-exchanged", FOR, '("TXINV", _) => (TXINV_WRONG,', '("TXINV", _) => (RXINV_WRONG,', [T_FOR]),
    ("first-ue-clear-judged", TWIN, '            in_force = value;\n            j.judged = Some((value, i));',
     '            in_force = value;\n            j.judged = j.judged.or(Some((value, i)));', [T_TWIN]),
    ("ue-ignored", TWIN, '        if !ue {\n            in_force = value;', '        if true {\n            in_force = value;', [T_TWIN]),
    ("ignored-dropped", TWIN, '        if changed != 0 {\n            let fields', '        if false {\n            let fields', [T_TWIN]),
    ("ignored-mask-all-bits", TWIN, '.filter(|b| b.2 != Expect::Unjudged)\n        .fold(0, |m, b| m | b.1);',
     '.fold(0, |m, b| m | b.1);', [T_TWIN]),
    ("narrow-accepted", TWIN, 'if a.bits != 32 || (a.offset', 'if (a.offset', [T_TWIN]),
    ("misplaced-accepted", TWIN, '|| (a.offset != o.cr1 && a.offset != o.cr2)', '|| false', [T_TWIN]),
    ("rc-rxinv-judged", TWIN, '[Expect::Clear, Expect::Clear, Expect::Unjudged]', '[Expect::Clear, Expect::Clear, line]', [T_TWIN, T_FOR]),
    ("rc-txinv-follows-line", TWIN, '[Expect::Clear, Expect::Clear, Expect::Unjudged]', '[Expect::Clear, line, Expect::Unjudged]', [T_TWIN, T_FOR]),
    ("unwritten-silent", FOR, '            out.push(Finding::plain(\n                OPTIONS_UNWRITTEN,', '            out.push(Finding::info(\n                OPTIONS_UNWRITTEN,', [T_FOR]),
    ("not-whole-judged", FOR, '        if !whole {\n            out.push(Finding::plain(\n                OPTIONS_UNJUDGEABLE,',
     '        if false {\n            out.push(Finding::plain(\n                OPTIONS_UNJUDGEABLE,', [T_FOR]),
    ("pull-fails", FOR, '            out.push(Finding::info(\n                PULL,', '            out.push(Finding::plain(\n                PULL,', [T_FOR]),
    ("usarts-not-logged", FOR, '    c.extend(usart_logs(serial));\n', '', [T_FOR]),
    ("pulls-not-read", FOR, '            ForeignRead::Pull(i),\n        ));', '            ForeignRead::Pull(i),\n        ));\n        r.pop();', [T_FOR, T_B1]),
]

env = dict(os.environ, PATH=os.path.expanduser("~/.cargo/bin:") + os.environ["PATH"],
           CARGO_TARGET_DIR=W + "/target/mut", RUST_TEST_THREADS="6")
want = set(sys.argv[1:])
for name, f, old, new, suites in M:
    if want and name not in want:
        continue
    path = os.path.join(W, f)
    src = open(path).read()
    if src.count(old) != 1:
        print(f"MUTANT {name}: SITE NOT UNIQUE ({src.count(old)})", flush=True)
        continue
    open(path, "w").write(src.replace(old, new))
    try:
        verdict, killers = "SURVIVED", []
        for s in suites:
            r = subprocess.run(["nice", "-n", "10", "cargo", "test", "-j", "6", *s[:-1], "--", s[-1]],
                               cwd=W, env=env, capture_output=True, text=True)
            out = r.stdout + r.stderr
            if "error[E" in out or "could not compile" in out and "test failed" not in out:
                verdict = "COMPILE-ERROR"
                break
            failed = re.findall(r"^test (\S+) \.\.\. FAILED", out, re.M)
            if failed:
                verdict, killers = "KILLED", failed
                break
        print(f"MUTANT {name}: {verdict} {killers}", flush=True)
    finally:
        open(path, "w").write(src)
