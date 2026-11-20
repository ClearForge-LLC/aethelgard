# AET-WO-1912 §1 steps 2-3: the new rows, from the five runs' samples and the throwaway's alone pass.
# Usage: python3 -I refit.py REPO REV RUNSDIR ALONE_TSV THROWAWAY_RUN RUN...
#   writes REPO/ci/test-weights.tsv and REPO/ci/colocate.tsv rows in place (values only; new rows in sorted place),
#   prints the decision table (markdown) on stdout.
# Rule (REFIT RULE 1-3, 5; WO §1): a row is changed only where it sits more than 25 % under its worst trustworthy sample.
#   test, not colocated: trustworthy = an alone time (the throwaway's alone pass, or a `solo` sample of the five runs);
#     raised to the worst of them, 3 decimals; a test with no row gets one at 0.5 s or more.
#   colocated block: its worst libtest `finished in` over the five runs and the throwaway's normal pass, rounded up.
import importlib.util, math, os, re, sys

repo, rev, d, alone_tsv, throwaways, runs = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5].split(','), sys.argv[6:]
throwaway = throwaways[-1]  # the run whose alone pass is ALONE_TSV
spec = importlib.util.spec_from_file_location('table', os.path.join(os.path.dirname(os.path.abspath(__file__)), 'table.py'))
table = importlib.util.module_from_spec(spec); spec.loader.exec_module(table)
coloc = table.rows(repo, rev, 'colocate.tsv', 1); weights = table.rows(repo, rev, 'test-weights.tsv', 2)

solo, cand, blocks = {}, {}, {}
for run in runs + throwaways:
    for f in os.listdir(f'{d}/{run}'):
        m = re.match(r'shard-(\d+)\.log$', f)
        if not m: continue
        _, _, bins, _ = table.shard(f'{d}/{run}/{f}')
        for b in bins:
            if (b['binary'],) in coloc: blocks.setdefault(b['binary'], []).append((b['fin'], run, int(m.group(1)))); continue
            if run in throwaways: continue  # the throwaway's normal pass: blocks only; its tests are weighed by the alone pass
            for n, s, how in b['tests']:
                (solo if how == 'solo' else cand).setdefault((b['binary'], n), []).append((s, run, int(m.group(1))))
alone = {}
for l in open(alone_tsv):
    k, b, n, s, cpu = l.rstrip('\n').split('\t')
    alone.setdefault((b, n), []).append((float(s), throwaway, int(k)))

def fmt(x): return f'{x:.3f}'
new_w, out = {}, []
for key in sorted(alone):  # the named list: every lagging or candidate test, each measured alone
    row = weights.get(key)
    trusted = alone.get(key, []) + solo.get(key, [])
    worst = max(trusted)
    c = max(cand.get(key, [(0, '', 0)]))[0]
    if row is None:
        if worst[0] < 0.5: continue
        verdict = 'new row'
    elif row < 0.75 * worst[0]: verdict = 'raised'
    else:
        if key in alone and c and row < 0.75 * c: out.append((key, row, row, worst, c, 'kept: the alone time holds; the in-shard candidate was a lock wait or a bound'))
        continue
    new_w[key] = worst[0]
    out.append((key, row, worst[0], worst, c, verdict))

new_c = {}
for b, ss in sorted(blocks.items()):
    w = max(ss); r = coloc[(b,)]
    if r < 0.75 * w[0]: new_c[b] = math.ceil(w[0]); out.append(((b, '(block)'), r, math.ceil(w[0]), w, 0, 'raised'))

print('| binary | test | row before | row after | from (s, run, shard) | worst lock/ub candidate | verdict |')
print('|---|---|---|---|---|---|---|')
for (b, n), r, a, w, c, v in out:
    print(f'| {b} | {n} | {r if r is not None else "-"} | {a if isinstance(a, int) else fmt(a)} | {w[0]:.2f} ({w[1]}, {w[2]}) | {f"{c:.2f}" if c else "-"} | {v} |')

# write in place
p = f'{repo}/ci/test-weights.tsv'
lines = open(p).read().splitlines()
done = set()
for i, l in enumerate(lines):
    if l.startswith('#') or not l.strip(): continue
    b, n, _ = l.split('\t')
    if (b, n) in new_w: lines[i] = f'{b}\t{n}\t{fmt(new_w[(b, n)])}'; done.add((b, n))
for key in sorted(k for k in new_w if k not in done):
    idx = [i for i, l in enumerate(lines) if not l.startswith('#') and l.strip() and tuple(l.split('\t')[:2]) < key]
    at = (idx[-1] + 1) if idx else len(lines)
    lines.insert(at, f'{key[0]}\t{key[1]}\t{fmt(new_w[key])}')
open(p, 'w').write('\n'.join(lines) + '\n')
p = f'{repo}/ci/colocate.tsv'
lines = open(p).read().splitlines()
for i, l in enumerate(lines):
    if l.startswith('#') or not l.strip(): continue
    b, _ = l.split('\t')
    if b in new_c: lines[i] = f'{b}\t{new_c[b]}'
open(p, 'w').write('\n'.join(lines) + '\n')
print(f'\n{len(new_w)} test rows written ({len(new_w) - len(done)} new), {len(new_c)} blocks raised', file=sys.stderr)
