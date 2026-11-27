#!/usr/bin/env python3
"""AET-WO-4310 mutant kit: each mutant replaces one guard (exactly once), runs
the suite named for it, and must fail it. The source is restored from a copy
and touched, so cargo never reuses a mutant's binary. Usage: run.py LIST.tsv"""
import os, shutil, subprocess, sys, time
root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../../.."))
rows = [l.rstrip("\n").split("\t") for l in open(sys.argv[1]) if l.strip() and not l.startswith("#")]
env = dict(os.environ, CARGO_BUILD_JOBS="6", RUST_TEST_THREADS="6",
           PATH=os.path.expanduser("~/.cargo/bin") + ":" + os.environ["PATH"])
for mid, path, old, new, pkg, test in rows:
    old, new = old.encode().decode("unicode_escape"), new.encode().decode("unicode_escape")
    f = os.path.join(root, path); src = open(f).read()
    if src.count(old) != 1:
        print(f"{mid}\tBAD\t{old!r} occurs {src.count(old)} times", flush=True); continue
    keep = f + ".orig4310"; shutil.copy2(f, keep)
    try:
        open(f, "w").write(src.replace(old, new))
        t0 = time.time()
        r = subprocess.run(["nice", "-n", "10", "cargo", "test", "-q", "-p", pkg, "--test", test],
                           cwd=root, env=env, capture_output=True, text=True)
        out = r.stdout + r.stderr
        compiled = "error[E" not in out
        verdict = "KILLED" if (r.returncode != 0 and compiled) else ("NOCOMPILE" if not compiled else "SURVIVED")
        print(f"{mid}\t{verdict}\t{time.time()-t0:.0f}s", flush=True)
    finally:
        shutil.copy(keep, f); os.remove(keep); os.utime(f, None)
