# AET-WO-1912 §1 step 2: spread the candidate names (binary<TAB>name<TAB>estimate s, from table.py --candidates)
# over N shards, longest first onto the least loaded, for the throwaway commit's alone pass.
# Usage: python3 -I alone.py N CANDIDATES > spike/1912/alone.tsv   (shard<TAB>name<TAB>estimate)
import sys
n, f = int(sys.argv[1]), sys.argv[2]
est = {}
for l in open(f):
    b, name, s = l.rstrip('\n').split('\t')
    est[name] = max(est.get(name, 0.0), float(s))
load = [0.0] * n
out = []
for name, s in sorted(est.items(), key=lambda x: (-x[1], x[0])):
    k = min(range(n), key=lambda i: (load[i], i))
    load[k] += s + 2.0  # about 2 s for one `cargo test --workspace` invocation to start every binary
    out.append((k, name, s))
print('# shard\tname\testimate s (AET-WO-1912 throwaway: each name run alone, after the shard\'s own tests)')
for k, name, s in sorted(out): print(f'{k}\t{name}\t{s:.2f}')
print(f'# loads: {", ".join(f"{x:.0f}" for x in load)}', file=sys.stderr)
