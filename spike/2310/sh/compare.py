#!/usr/bin/env python3
"""AET-WO-2310 §1.1 sweep B: campaign lines against their archives' replays (main's CLI), on the verdict fields.
Usage: compare.py RUNS REPLAY_TXT REPO   (run with python3 -I)."""
import json, os, sys, glob, subprocess
runs, replay_txt, repo = sys.argv[1:4]
lines = {}
windows = {}
files = set()
for out in sorted(glob.glob(os.path.join(runs, 'camp', '*', 'campaign.out'))):
    g = out.split(os.sep)[-2]
    for raw in open(out):
        raw = raw.strip()
        if not raw:
            continue
        d = json.loads(raw)
        if 'run_id' in d:
            lines[d['run_id']] = (g, d)
            windows[d.get('window')] = windows.get(d.get('window'), 0) + 1
            files.add(d.get('file'))
reps = {}
for raw in open(replay_txt):
    path, rc, js = raw.split(' ', 2)
    reps[path.rsplit('/', 1)[1]] = (path, int(rc[3:]), json.loads(js))
FIELDS = ['verdict', 'expect', 'matched', 'findings', 'expect_findings']
bad = 0
for rid, (g, d) in sorted(lines.items(), key=lambda x: (x[1][0], x[1][1]['scenario'])):
    p = reps.get(rid)
    if p is None:
        print(f'{g} {d["scenario"]} {rid} NO-ARCHIVE'); bad += 1; continue
    path, rc, r = p
    diff = [f for f in FIELDS if r.get(f) != d.get(f)]
    same = 'same' if rc == 0 and not diff else 'DIFFERENT'
    if same != 'same':
        bad += 1
    print(f'{g} {d["scenario"]} {rid} campaign_matched={d["matched"]} verdict={d["verdict"]} replay_rc={rc} {same} {diff if diff else ""}')
print(f'lines={len(lines)} archives={len(reps)} different={bad} windows={windows}')
committed = subprocess.run(['git', '-C', repo, 'ls-files', 'scenarios/*.json'], capture_output=True, text=True).stdout.split()
names = {os.path.basename(f) for f in files if f}
missing = [c for c in committed if not c.endswith('cases.json') and not any(f and (f == c or f.endswith(c) or c.endswith(f)) for f in files)]
print(f'committed scenario files={len(committed)} not in any campaign line={missing}')
