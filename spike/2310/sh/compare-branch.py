#!/usr/bin/env python3
"""AET-WO-2310 acceptance: the branch's campaign lines against main's (spike/2310/out/campaign/*.jsonl), by group
and scenario. Verdict fields must be equal; the fields the bump removes or adds, and those that move by
construction (run_id, run, trace_bytes, event_count, state_sequence_hash), are reported, not compared.
Usage: compare-branch.py MAIN_DIR BRANCH_RUNS  (run with python3 -I)."""
import json, os, sys, glob
main_dir, runs = sys.argv[1:3]
VERDICT = ['verdict', 'expect', 'matched', 'findings', 'expect_findings', 'lifecycle', 'conditions',
           'uid_boundary', 'uid_boundary_unmet', 'observation_witness', 'virtual_time_ns']
MOVES = {'run_id', 'run', 'trace_bytes', 'event_count', 'state_sequence_hash'}
def load(path):
    return [json.loads(l) for l in open(path) if l.strip() and 'run_id' in json.loads(l)]
moved, same, keys = 0, 0, {}
for mfile in sorted(glob.glob(os.path.join(main_dir, '*.jsonl'))):
    g = os.path.basename(mfile)[:-6]
    bfile = os.path.join(runs, 'camp', g, 'campaign.out')
    m = {l['scenario']: l for l in load(mfile)}
    b = {l['scenario']: l for l in load(bfile)}
    assert set(m) == set(b), (g, set(m) ^ set(b))
    for sc in sorted(m):
        x, y = m[sc], b[sc]
        diff = [f for f in VERDICT if x.get(f) != y.get(f)]
        # battery_sag: every field but the instant's own
        xs, ys = x.get('battery_sag'), y.get('battery_sag')
        if (xs is None) != (ys is None):
            diff.append('battery_sag')
        elif xs:
            xs2 = {k: v for k, v in xs.items() if k != 'start_ns'}
            ys2 = {k: v for k, v in ys.items() if k not in ('start_ns', 'from_boot_word_ns')}
            if xs2 != ys2: diff.append('battery_sag')
            if xs['start_ns'] != ys['start_ns']: diff.append('battery_sag.start_ns %s -> %s' % (xs['start_ns'], ys['start_ns']))
        removed = sorted(set(x) - set(y)); added = sorted(set(y) - set(x))
        other = [f for f in set(x) & set(y) if x[f] != y[f] and f not in VERDICT and f not in MOVES and f != 'battery_sag']
        for k in removed: keys['removed:' + k] = keys.get('removed:' + k, 0) + 1
        for k in added: keys['added:' + k] = keys.get('added:' + k, 0) + 1
        if diff or other:
            moved += 1
            print(f'{g} {sc} MOVED {diff} {other}')
        else:
            same += 1
print(f'same={same} moved={moved} fields={keys}')
