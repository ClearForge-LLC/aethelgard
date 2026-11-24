#!/usr/bin/env python3 -I
"""bomdiff.py -- for the keys whose absence removes a declared thing, does emit's BOM lose parts?  Prints removed/added BOM lines."""
import json, os, subprocess, sys, copy
sys.argv = ['x', '--paths', 'none']
exec(open(os.path.join(os.path.dirname(os.path.abspath(__file__)), 'sweep.py')).read().split('# ---------- main ----------')[0])
def bom(spec, tag):
    d = f'{CACHE}/w/bom-{tag}'; shutil.rmtree(d, ignore_errors=True); os.makedirs(d)
    sp = d + '/spec.json'; open(sp, 'w').write(json.dumps(spec))
    rc, o, e = run([A, 'emit', '--file', sp, '--out', d + '/e'])
    if rc: return None
    f = [x for x in os.listdir(d + '/e') if x.endswith('.bom.csv')][0]
    L = set(open(d + '/e/' + f).read().splitlines()); shutil.rmtree(d, ignore_errors=True); return L
s03 = load(S03)
cases = [('sentinel-03', 'power.sense[]', 'empty'), ('sentinel-03', 'test_points[]', 'delete'), ('hse-crystal-16', 'mcu.hse', 'null'),
         ('sentinel-03', 'mcu.reset_switch', 'delete'), ('sentinel-03', 'power.loads[]', 'empty'), ('pair-standin', 'diff_pairs[]', 'empty'),
         ('sentinel-03', 'power.conditions.dc_bias_derating[]', 'empty'), ('sentinel-03', 'board.stackup.fab_tolerances', 'delete')]
for c, p, v in cases:
    base = load(dict(universe)[c]) if c != 'sentinel-03' else s03
    b = bom(base, 'b'); x = bom(apply(base, p, v), 'x')
    print('==', c, p, v)
    if b is None or x is None: print('   emit refused'); continue
    print('   BOM removed:', sorted(b - x)); print('   BOM added:', sorted(x - b))
