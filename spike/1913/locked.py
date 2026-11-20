# AET-WO-1913: per run and shard, each lock-serialised binary's rows against its libtest `finished in` (the back-test).
# Usage: python3 -I locked.py WEIGHTS_TSV RUNSDIR RUN...
#   WEIGHTS_TSV is a ci/test-weights.tsv (any revision: `git show REV:ci/test-weights.tsv > f`).
# For twin_tier2 and board_tier2 in every (run, shard) that ran them: the tests run, the sum of their rows, the measured
# `finished in`, and rows less measured (positive: the plan over-weighs the shard; negative: it under-weighs it, the
# WO-1913 §5 risk). Then per binary the totals, the worst under-weighing and the mean absolute error.
import importlib.util, os, sys
spec = importlib.util.spec_from_file_location('table', os.path.join(os.path.dirname(os.path.abspath(__file__)), 'table.py'))
table = importlib.util.module_from_spec(spec); spec.loader.exec_module(table)
wf, d, runs = sys.argv[1], sys.argv[2], sys.argv[3:]
w = {}
for l in open(wf):
    if l.startswith('#') or not l.strip(): continue
    b, n, s = l.rstrip('\n').split('\t'); w[(b, n)] = float(s)
BINS = ('tests/twin_tier2.rs', 'tests/board_tier2.rs')
print('| run | shard | binary | tests | rows s | finished in s | rows - measured s |')
print('|---|---|---|---|---|---|---|')
agg = {b: [] for b in BINS}
for run in runs:
    for k in range(64):
        f = f'{d}/{run}/shard-{k}.log'
        if not os.path.exists(f): continue
        _, _, bins = table.shard(f)
        for b in bins:
            if b['binary'] not in BINS: continue
            r = sum(w.get((b['binary'], n), table.DEFAULT) for n, _, _ in b['tests'])
            agg[b['binary']].append((r, b['fin'], run, k))
            print(f"| {run} | {k} | {b['binary'].split('/')[-1]} | {len(b['tests'])} | {r:.0f} | {b['fin']:.0f} | {r - b['fin']:+.0f} |")
print()
for b, xs in agg.items():
    if not xs: continue
    worst = min(xs, key=lambda x: x[0] - x[1])
    print(f"{b}: {len(xs)} samples; rows {sum(x[0] for x in xs):.0f} s against {sum(x[1] for x in xs):.0f} s measured; "
          f"mean |error| {sum(abs(x[0] - x[1]) for x in xs) / len(xs):.0f} s; most under-weighed {worst[0] - worst[1]:+.0f} s ({worst[2]} shard {worst[3]})")
