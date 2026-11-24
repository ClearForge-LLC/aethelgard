#!/usr/bin/env python3 -I
"""Usage: show.py PATH [variant] [results file] -- the diff of the rows for one key; numeric changes collapsed per field."""
import json, sys, re, collections
f = sys.argv[3] if len(sys.argv) > 3 else '/home/forge/.cache/aet-4300k/sweep/results.json'
def cut(x, n=330): return x if len(x) < n else x[:n] + '...'
for r in json.load(open(f)):
    if sys.argv[1] != r['path']: continue
    if len(sys.argv) > 2 and sys.argv[2] != '-' and r.get('variant') != sys.argv[2]: continue
    print('=====', r['path'], r.get('carrier'), r.get('variant'), r.get('exits'))
    num = collections.OrderedDict()
    for k, p, v in r.get('diff', []):
        if k == 'chg' and all(re.fullmatch(r'-?[\d.]+', x) for x in v):
            key = re.sub(r'\[[^\]]*\]', '[]', p); num.setdefault(key, []).append((v[0], v[-1])); continue
        print(' ', k, p, ' || '.join(cut(x) for x in v[:4]))
    for k, l in num.items(): print('  chg-num', k, f'x{len(l)}', 'e.g.', l[0][0], '->', l[0][1])
