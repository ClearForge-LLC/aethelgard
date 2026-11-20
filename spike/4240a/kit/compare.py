"""AET-WO-4240a: baseline vs branch under the declared mask (PREDICTIONS.md): a solved topology's
top-level schema_version (12 -> 13) and specification_sha256; nothing else."""
import json, sys
B, A = sys.argv[1], sys.argv[2]
bad = 0
for f in ('lib-specs.jsonl', 'lib-topos.jsonl'):
    b = [json.loads(l) for l in open(f'{B}/{f}')]; a = [json.loads(l) for l in open(f'{A}/{f}')]
    assert len(a) == len(b)
    for x, y in zip(b, a):
        assert x['path'] == y['path']
        masked = []
        for d in (x, y):
            t = d.get('topology')
            if t is not None:
                t['schema_version'] = 'MASK'; t['specification_sha256'] = 'MASK'
        if x != y:
            bad += 1
            keys = [k for k in set(x) | set(y) if x.get(k) != y.get(k)]
            print('DIFF', x['path'], keys)
            for k in keys: print('  base', str(x.get(k))[:300]); print('  now ', str(y.get(k))[:300])
        else:
            print('same', x['path'])
print('DIFFS', bad)
