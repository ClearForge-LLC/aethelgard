# AET-WO-1912 §5, the targeted adversarial pass, over the runs' logs (normal passes only).
# (a) A row raised from one outlier: every raised test row against the test's in-shard upper bound on each run (`ok`
#     less its binary's `Running`, true for any test, lock or not). A new row above every run's bound is an alone sample
#     the test never took in a shard: an outlier that would over-plan its shard every run.
# (b) A block weighed from a run where it did not run whole in one process: each block sample's passed and filtered counts.
# (c) The mirror of §1 (F3): rows at REV more than 1/0.75 of their worst upper bound, so above anything CI measured.
#     Blind to lock-serialised binaries, whose bound holds the queue.
# Usage: python3 -I adv.py REPO REV RUNSDIR DECISIONS_MD RUN...
import importlib.util, os, re, sys
spec = importlib.util.spec_from_file_location('table', os.path.join(os.path.dirname(os.path.abspath(__file__)), 'table.py'))
table = importlib.util.module_from_spec(spec); spec.loader.exec_module(table)
repo, rev, d, dec, runs = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5:]
ub = table.upper_bounds(d, runs)

raised = {}
for l in open(dec):
    p = [x.strip() for x in l.split('|')]
    if len(p) > 7 and p[7] in ('raised', 'new row') and p[2] != '(block)': raised[(p[1], p[2])] = float(p[4])
print('(a) raised rows above every in-shard upper bound:')
bad = 0
for k, new in sorted(raised.items()):
    u = ub.get(k, [])
    if not u: print(f'  {k}: no in-shard sample'); continue
    if new > max(u): bad += 1; print(f'  OUTLIER {k[0]} {k[1]}: new {new} > every upper bound {max(u):.2f} over {len(u)} runs')
print(f'  {bad} of {len(raised)} rows above every upper bound')

print('(b) block samples not run whole:')
coloc = table.rows(repo, rev, 'colocate.tsv', 1)
for run in runs:
    for f in sorted(os.listdir(f'{d}/{run}')):
        if not re.match(r'shard-\d+\.log$', f): continue
        cur = None
        for l in open(f'{d}/{run}/{f}', encoding='utf-8', errors='replace'):
            m = table.TS.match(l.rstrip('\n'))
            if not m: continue
            s = m.group(2)
            if s.startswith('ALONE ') or 'model name' in s: break
            mb = re.match(r'\s*Running (.+?) \(', s)
            if mb: cur = mb.group(1); continue
            mr = re.search(r'test result: ok\. (\d+) passed; \d+ failed; \d+ ignored; \d+ measured; (\d+) filtered', s)
            if mr and (cur,) in coloc and int(mr.group(1)):
                print(f'  {"PARTIAL " if int(mr.group(2)) else ""}{cur} {run} {f}: {mr.group(1)} passed, {mr.group(2)} filtered')

print('(c) rows above every in-shard upper bound (not lock-serialised binaries):')
w = table.rows(repo, rev, 'test-weights.tsv', 2)
tot = 0.0
for k, r in sorted(w.items(), key=lambda x: -(x[1] - max(ub.get(x[0], [x[1]])))):
    u = ub.get(k)
    if not u or k[0] in table.LOCKED or r <= max(u) / 0.75 or r - max(u) < 1: continue
    tot += r - max(u)
    print(f'  {k[0]} {k[1]}: row {r} > {max(u):.2f}, the worst upper bound over {len(u)} runs')
print(f'  {tot:.0f} s planned above the bounds')
