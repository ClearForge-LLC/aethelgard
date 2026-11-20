# AET-WO-1913 §1 step 3: set every twin_tier2 and board_tier2 row to its alone time, and raise the two blocks to their worst.
# Usage: python3 -I refit.py REPO REV ALONE_TSV RUNSDIR THROWAWAY_RUN RUN... [--floor BINARY=SECONDS ...]
#   ALONE_TSV is alone_times.py's output for THROWAWAY_RUN (shard, binary, name, seconds, cpu).
#   Writes REPO/ci/test-weights.tsv and REPO/ci/colocate.tsv values in place; prints the decision table (markdown).
# Rules (WO §1 step 3; REFIT RULE 1, 3, 5):
#   a twin_tier2 or board_tier2 test: the worse of its alone samples (each name ran alone on two shards), 3 decimals,
#     lowered or raised; never from a shared-process run (both take their lock per call).
#   four_layer_fab_tier2, verify_47k_tier2: their worst `finished in` over the runs, the throwaway's normal pass, its
#     whole-binary alone pass and any --floor (a sample measured earlier and cited by the WO), rounded up; never lowered.
import importlib.util, math, os, re, sys
args = [a for a in sys.argv[1:] if not a.startswith('--floor')]
floors = {}
for i, a in enumerate(sys.argv):
    if a == '--floor': b, s = sys.argv[i + 1].split('='); floors[b] = float(s)
args = [a for a in args if '=' not in a]
repo, rev, alone_tsv, d, throwaway, runs = args[0], args[1], args[2], args[3], args[4], args[5:]
spec = importlib.util.spec_from_file_location('table', os.path.join(os.path.dirname(os.path.abspath(__file__)), 'table.py'))
table = importlib.util.module_from_spec(spec); spec.loader.exec_module(table)
BINS = ('tests/twin_tier2.rs', 'tests/board_tier2.rs')
BLOCKS = ('tests/four_layer_fab_tier2.rs', 'tests/verify_47k_tier2.rs')
weights = table.rows(repo, rev, 'test-weights.tsv', 2); coloc = table.rows(repo, rev, 'colocate.tsv', 1)

def jobs_of(run):
    j = {}
    for l in open(f'{d}/{run}/jobs.tsv'):
        p = l.split('\t'); m = re.match(r'tools-shard \((\d+)\)', p[1])
        if m: j[m.group(1)] = p[0]
    return j
TJ = jobs_of(throwaway)
alone = {}
for l in open(alone_tsv):
    k, b, n, s, cpu = l.rstrip('\n').split('\t')
    if n.startswith('@'): alone.setdefault((n[1:],), []).append((float(s), f'{throwaway} shard {k} job {TJ[k]} alone', cpu))
    elif b in BINS: alone.setdefault((b, n), []).append((float(s), f'{throwaway} shard {k} job {TJ[k]}', cpu))
for run in runs + [throwaway]:
    for f in os.listdir(f'{d}/{run}'):
        m = re.match(r'shard-(\d+)\.log$', f)
        if not m: continue
        for b in table.shard(f'{d}/{run}/{f}')[2]:
            if b['binary'] in BLOCKS: alone.setdefault((b['binary'],), []).append((b['fin'], f'{run} shard {m.group(1)} job {jobs_of(run)[m.group(1)]}', ''))

new_w, new_c, out, missing = {}, {}, [], []
for key, row in sorted(weights.items()):
    if key[0] not in BINS: continue
    ss = alone.get(key, [])
    if len(ss) < 2: missing.append(f'{key[1]}: {len(ss)} alone sample(s)')
    if not ss: continue
    worst = max(ss)
    new = round(worst[0], 3)
    new_w[key] = new
    how = 'lowered' if new < row else 'raised' if new > row else 'same'
    out.append((key, row, ss, new, how))
# The architect's widening (bus, 2026-11-20 18:09Z): any other row more than 25 % off its alone time may be set to it.
# Its alone time here is a `solo` sample (the only test of its binary in a shard: the binary's `finished in`) over the five
# runs and the throwaway's normal pass; set to the worst of them, lowered or raised. A row with no solo sample is not moved.
solo = {}
for run in runs + [throwaway]:
    jobs = jobs_of(run)
    for f in os.listdir(f'{d}/{run}'):
        m = re.match(r'shard-(\d+)\.log$', f)
        if not m: continue
        for b in table.shard(f'{d}/{run}/{f}')[2]:
            for n, s, how in b['tests']:
                if how == 'solo': solo.setdefault((b['binary'], n), []).append((s, f'{run} shard {m.group(1)} job {jobs[m.group(1)]}', 'solo'))
for key, row in sorted(weights.items()):
    if key[0] in BINS or key not in solo: continue
    worst = max(solo[key])
    if abs(row - worst[0]) <= 0.25 * worst[0]: continue
    new = round(worst[0], 3); new_w[key] = new
    out.append((key, row, solo[key], new, 'lowered' if new < row else 'raised'))
for b in BLOCKS:
    ss = alone.get((b,), [])
    if b in floors: ss.append((floors[b], 'cited by the WO (AET-WO-1912)', ''))
    new = max(math.ceil(max(ss)[0]), int(coloc[(b,)]))
    new_c[b] = new
    out.append(((b, '(block)'), coloc[(b,)], ss, new, 'raised' if new > coloc[(b,)] else 'kept'))

def fmt(x): return f'{x:.3f}'.rstrip('0').rstrip('.') if '.' in f'{x:.3f}' else str(x)
for f, upd, cols in (('test-weights.tsv', new_w, 2), ('colocate.tsv', new_c, 1)):
    p = f'{repo}/ci/{f}'; lines = open(p).read().split('\n')
    for i, l in enumerate(lines):
        if l.startswith('#') or not l.strip(): continue
        parts = l.split('\t'); key = tuple(parts[:cols]) if cols == 2 else parts[0]
        if key in upd: lines[i] = '\t'.join(parts[:cols] + [fmt(upd[key]) if cols == 2 else str(upd[key])])
    open(p, 'w').write('\n'.join(lines))

print('| binary | test | row before | alone samples (s, run/shard, CPU) | row after | change |')
print('|---|---|---|---|---|---|')
for key, row, ss, new, how in out:
    sm = '; '.join(f'{s:.2f} ({w}{", " + c if c else ""})' for s, w, c in sorted(ss, reverse=True))
    print(f'| {key[0].split("/")[-1]} | {key[1]} | {fmt(row)} | {sm} | {fmt(new) if isinstance(new, float) else new} | {how} |')
tw = [(r, n) for k, r, _, n, _ in out if k[0] in BINS]
for b in BINS:
    xs = [(r, n) for k, r, _, n, _ in out if k[0] == b]
    print(f'\n{b}: {len(xs)} rows, {sum(r for r, _ in xs):.1f} s before, {sum(n for _, n in xs):.1f} s after; '
          f'{sum(1 for k, r, _, n, h in out if k[0] == b and h == "lowered")} lowered, {sum(1 for k, r, _, n, h in out if k[0] == b and h == "raised")} raised')
for m in missing: print(f'MISSING {m}', file=sys.stderr)
