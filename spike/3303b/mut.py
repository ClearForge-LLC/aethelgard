#!/usr/bin/env python3
"""AET-WO-3303b mutants: one per guard this WO adds or changes, each against svd-hal's board.rs unit
tests, on a copy of the tree at a named commit (never the worktree). Prints one row per mutant:
id, killed|SURVIVED, the failing tests."""
import os, subprocess, sys, shutil, re
SHA = sys.argv[1]
HOME = os.path.expanduser('~')
base = f'{HOME}/.cache/aet-3303b/mut'
tree = f'{base}/tree'
F = 'crates/aethelgard-svd-hal/src/board.rs'
MUTANTS = [
 ('M1 check_parts: no UnscheduledAction loop',
  '        if !scheduled.iter().any(|acts| acts.contains(&action)) {', '        if false && !scheduled.iter().any(|acts| acts.contains(&action)) {'),
 ('M2 check_parts: disjoint skipped', '    disjoint(&owners)\n}', '    let _ = disjoint(&owners);\n    Ok(())\n}'),
 ('M3 emit_proofs: split keyed on timers again', '    if parts.len() < 2 {', '    if plan.timers.is_empty() {'),
 ('M4 emit_proofs: < 1', '    if parts.len() < 2 {', '    if parts.len() < 1 {'),
 ('M5 emit_proofs: < 3', '    if parts.len() < 2 {', '    if parts.len() < 3 {'),
 ('M6 emit_proofs: no check_parts call', '        check_parts(plan, facts, &parts)?;\n', ''),
]
env = dict(os.environ, PATH=f"{HOME}/.cargo/bin:" + os.environ['PATH'], CARGO_TARGET_DIR=f'{base}/target')
for mid, old, new in MUTANTS:
    shutil.rmtree(tree, ignore_errors=True); os.makedirs(tree)
    subprocess.run(f'git -C {HOME}/aethelgard-3303b archive {SHA} | tar -x -C {tree}', shell=True, check=True)
    p = f'{tree}/{F}'; t = open(p).read()
    assert t.count(old) == 1, (mid, t.count(old))
    open(p, 'w').write(t.replace(old, new))
    r = subprocess.run(['nice', '-n', '10', 'cargo', 'test', '-q', '-p', 'aethelgard-svd-hal', '--lib', 'board::tests'], cwd=tree, env=env, capture_output=True, text=True)
    failed = sorted(set(re.findall(r'^    (board::tests::\S+)$', r.stdout, re.M)))
    state = 'killed' if r.returncode != 0 else 'SURVIVED'
    print(f'{mid}\t{state}\t{" ".join(f.split("::")[-1] for f in failed) or r.stderr[-300:].replace(chr(10)," ") if r.returncode else ""}', flush=True)
