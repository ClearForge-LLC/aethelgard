# AET-WO-1913 §1 step 2: the throwaway's alone list. Every twin_tier2 and board_tier2 test twice, on two different
# shards (two runners: the row is the worse of two alone samples, not one runner's), and the two blocks
# (four_layer_fab_tier2, verify_47k_tier2) once each as a whole binary in one process (`@binary`).
# Longest first onto the least loaded shard; a name's second copy never lands on its first's shard.
# Usage: python3 -I alone.py N REPO REV RUNSDIR RUN... > spike/1913/alone.tsv   (shard<TAB>name<TAB>estimate s)
# The estimate is only for spreading: the test's row, capped at its worst in-shard `ok` less `Running`.
import importlib.util, os, sys
spec = importlib.util.spec_from_file_location('table', os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '1912', 'table.py'))
table = importlib.util.module_from_spec(spec); spec.loader.exec_module(table)
n, repo, rev, d, runs = int(sys.argv[1]), sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5:]
BINS = ('tests/twin_tier2.rs', 'tests/board_tier2.rs')
BLOCKS = ('tests/four_layer_fab_tier2.rs', 'tests/verify_47k_tier2.rs')
weights = table.rows(repo, rev, 'test-weights.tsv', 2); coloc = table.rows(repo, rev, 'colocate.tsv', 1)
ub = table.upper_bounds(d, runs)
items = []
for (b, name), r in weights.items():
    if b not in BINS: continue
    est = min(r, max(ub.get((b, name), [r])))
    items += [(est, name, 0), (est, name, 1)]
for b in BLOCKS: items.append((coloc[(b,)], '@' + b, 0))
load, where, out = [0.0] * n, {}, []
for est, name, copy in sorted(items, key=lambda x: (-x[0], x[1], x[2])):
    k = min((i for i in range(n) if where.get(name) != i), key=lambda i: (load[i], i))
    load[k] += est + 3.0  # one `cargo test --workspace` invocation starting every binary
    where[name] = k
    out.append((k, name, est))
print("# shard\tname\testimate s (AET-WO-1913 throwaway: each name run alone after the shard's own tests; @binary = that binary whole)")
for k, name, est in sorted(out): print(f'{k}\t{name}\t{est:.2f}')
print(f'# {len(out)} invocations; loads: {", ".join(f"{x:.0f}" for x in load)}', file=sys.stderr)
