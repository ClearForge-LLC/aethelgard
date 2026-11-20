"""AET-WO-2305 §1.1: one table row per scenario of a run directory (sh/run.sh's), from its campaign
lines, its archives (kinds 13, 18, 19) and the scratch CLI's Renode logs. Run with python3 -I."""
import json, os, re, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from trace import records

STAMP = re.compile(r'^\[(\d\d):(\d\d):(\d\d)\.(\d{9})\] (.*)$')
def stamp(line):
    m = STAMP.match(line)
    if not m: return None, None
    h, mi, s, ns, body = m.groups()
    return ((int(h) * 60 + int(mi)) * 60 + int(s)) * 10**9 + int(ns), body

def row(run, scenario, log_path):
    rec = records(os.path.join(run, 'archive', scenario['run_id'], 'trace.cbor'))
    releases = [r[1] for r in rec if r[0] == 18 and r[3] == 1]
    asserts = [r[1] for r in rec if r[0] == 18 and r[3] == 0]
    probes = [r for r in rec if r[0] == 19]
    bound = [r for r in rec if r[0] == 32]
    w13 = [r for r in rec if r[0] in (13, 22)]
    expected = w13[0][6] if w13 else []
    device = w13[0][4] if w13 else None
    lines = open(log_path, errors='replace').read().splitlines()
    rel_idx = [i for i, l in enumerate(lines) if l.startswith('AETHELGARD-RELEASE-')]
    pb_idx = [i for i, l in enumerate(lines) if l.startswith('AETHELGARD-PROBE-BEGIN-')]
    begin = lines.index('AETHELGARD-BOOT-BEGIN')
    start_line = rel_idx[-1] + 1 if rel_idx else 0
    start_ns = releases[-1] + 10 if releases else 0
    bw = None; bw_line = None
    for i in range(start_line, begin):
        t, body = stamp(lines[i])
        if body and 'AETHELGARD-BOOTWORD ' in body:
            v, before, count = body.split('AETHELGARD-BOOTWORD ')[1].split()[:3]
            if int(v, 16) == 0x2100B007 and int(before, 16) != 0x2100B007:
                bw = start_ns + int(count) * 10; bw_line = i; break
    def dev_lines(a, b):
        out = []
        for i in range(a, b):
            t, body = stamp(lines[i])
            if body and device and re.match(r'^\[(NOISY|DEBUG|INFO|WARNING|ERROR)\] ' + re.escape(device) + ': ', body):
                out.append((t, body))
        return out
    first_id = None
    for t, body in dev_lines(start_line, begin):
        if expected and body == expected[0]: first_id = t; break
    at_probe = dev_lines(start_line, pb_idx[-1]) if pb_idx else None
    at_bw = dev_lines(start_line, bw_line) if bw_line is not None else None
    return {
        'scenario': scenario['scenario'], 'verdict': scenario['verdict'], 'matched': scenario['matched'],
        'findings': sorted(set(scenario.get('findings', []))),
        'asserted_ns': asserts, 'released_ns': releases, 'boot_start_ns': start_ns,
        'boot_word_ns': bw, 'boot_word_since_start_ns': None if bw is None else bw - start_ns,
        'first_identity_line_stamp_ns': first_id,
        'first_identity_line_since_start_ns': None if first_id is None else first_id - start_ns,
        'probe_at_ns': [p[1] for p in probes], 'probe_boot_word': [hex(p[3][3][0]) if p[3][3] else None for p in probes],
        'witness_at_probe': [len(p[6][0]) if p[6] else None for p in probes],
        'device_lines_to_probe': None if at_probe is None else len(at_probe),
        'device_lines_to_boot_word': None if at_bw is None else len(at_bw),
        'identity_lines_expected': len(expected), 'device': device,
        'recovery_bound': [r[2:] for r in bound],
    }

def main(run):
    scen = [json.loads(l) for l in open(os.path.join(run, 'campaign.jsonl')) if '"scenario"' in l]
    logs = sorted(f for f in os.listdir(os.path.join(run, 'log')) if f.startswith('renode.'))
    assert len(logs) == len(scen), (len(logs), len(scen))
    for s, l in zip(scen, logs):
        print(json.dumps(row(run, s, os.path.join(run, 'log', l))))
if __name__ == '__main__':
    main(sys.argv[1])
