# AET-WO-1912 §5, the targeted adversarial pass, over the five runs' logs.
# (a) A row raised from one outlier: for every raised test row, its in-shard upper bound on each run (`ok` less its
#     binary's `Running`, true for any test, lock or not). A new row above every run's upper bound is an alone sample
#     the test never took in a shard: an outlier that would over-plan its shard every run.
# (b) A block weighed from a run where it did not run whole in one process: each block sample's passed count against
#     the block's test count on every run.
# Usage: python3 -I adv.py RUNSDIR DECISIONS_MD RUN...
import re, sys, os
from datetime import datetime
TS = re.compile(r'^[^\t]*\t[^\t]*\t\ufeff?(\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\.\d+)Z ?(.*)$')
d, dec, runs = sys.argv[1], sys.argv[2], sys.argv[3:]
raised = {}
for l in open(dec):
    p = [x.strip() for x in l.split('|')]
    if len(p) > 7 and p[7] in ('raised', 'new row') and p[2] != '(block)': raised[(p[1], p[2])] = float(p[4])
ub, block_n = {}, {}
for run in runs:
    for f in os.listdir(f'{d}/{run}'):
        if not re.match(r'shard-\d+\.log$', f): continue
        cur = None
        for l in open(f'{d}/{run}/{f}', encoding='utf-8', errors='replace'):
            m = TS.match(l.rstrip('\n'))
            if not m: continue
            t, s = datetime.fromisoformat(m.group(1)[:26]), m.group(2)
            if s.startswith('ALONE ') or 'model name' in s: break
            mb = re.match(r'\s*Running (.+?) \(', s)
            if mb: cur, start = mb.group(1), t; continue
            mt = re.match(r'test (\S+) \.\.\. ok', s)
            if mt and cur: ub.setdefault((cur, mt.group(1)), []).append((t - start).total_seconds())
            mr = re.search(r'test result: ok\. (\d+) passed; \d+ failed; \d+ ignored; \d+ measured; (\d+) filtered', s)
            if mr and cur in ('tests/supply_tier1.rs', 'tests/attribute_seeded.rs', 'tests/campaign_tier2.rs') and int(mr.group(1)):
                block_n.setdefault(cur, []).append((run, f, int(mr.group(1)), int(mr.group(2))))
print('(a) raised rows above every in-shard upper bound:')
bad = 0
for k, new in sorted(raised.items()):
    u = ub.get(k, [])
    if not u: print(f'  {k}: no in-shard sample'); continue
    if new > max(u): bad += 1; print(f'  OUTLIER {k[0]} {k[1]}: new {new} > every upper bound {max(u):.2f} over {len(u)} runs')
print(f'  {bad} of {len(raised)} rows above every upper bound')
print('(b) block samples not run whole:')
for b, ss in sorted(block_n.items()):
    part = [s for s in ss if s[3] != 0]
    print(f'  {b}: {len(ss)} samples, passed {sorted(set(s[2] for s in ss))}, filtered {sorted(set(s[3] for s in ss))}{"  PARTIAL " + str(part) if part else ""}')
