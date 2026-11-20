#!/usr/bin/env python3
"""AET-WO-4241 §1.4 re-judge compare (~/.cache/aet-4241/rj). Per group and scenario, main against the branch:
the verdict and `matched`; the findings with `device_unmodelled` (main) and `identity_not_witnessable` (branch)
set aside, which must otherwise be equal (the ruling's D3: any other kind appearing or vanishing is a stop);
replay and attribute, but for run ids and hashes. Per group, the boot (simulate): verdict, findings likewise,
and every modelled step's status-write instant (STEP hook counts) equal; the boot word's shift and the
not-witnessable block's total. Usage: python3 -I cmp.py <rj-dir>"""
import json, os, re, sys
D = sys.argv[1]
SKIP = {'run', 'run_id', 'state_sequence_hash', 'trace_sha256', 'archive', 'firmware_sha256', 'sha256'}
NEW = {'identity_not_witnessable', 'device_unmodelled'}
def jl(p):
    try: return [json.loads(l) for l in open(p) if l.strip()]
    except OSError: return []
def jf(p):
    try: return json.load(open(p))
    except Exception: return None
def kind(f):
    return f if isinstance(f, str) else f[0] if isinstance(f, list) else f.get('kind')
def kinds(fs):
    return sorted((kind(f), json.dumps(f) if not isinstance(f, str) and kind(f) not in NEW and isinstance(f, list) else None)
                  for f in fs or [] if kind(f) not in NEW)
def new(fs):
    return sum(1 for f in fs or [] if kind(f) in NEW)
def steps(p):
    out = {}
    try:
        for l in open(p):
            m = re.search(r'\] \[WARNING\] sysbus: AETHELGARD-STEP (\d+) ([0-9a-f]+) (\d+)$', l.strip())
            if m: out.setdefault(int(m.group(1)), []).append((m.group(2), int(m.group(3))))
            m = re.search(r'AETHELGARD-BOOTWORD ([0-9a-f]+) [0-9a-f]+ (\d+)$', l.strip())
            if m and m.group(1) == '2100b007': out.setdefault('bw', []).append(int(m.group(2)))
    except OSError: pass
    return out
lines, stops = [], 0
for g in sorted(os.listdir(f'{D}/sim/main')):
    a, b = jf(f'{D}/sim/main/{g}/simulate.json'), jf(f'{D}/sim/branch/{g}/simulate.json')
    if not a or not b:
        lines.append(f'STOP? sim {g}: missing verdict main={bool(a)} branch={bool(b)}'); stops += 1; continue
    sa, sb = steps(f'{D}/sim/main/{g}/boot.log'), steps(f'{D}/sim/branch/{g}/boot.log')
    n_checks = len((a.get('boot') or {}).get('i2c', [])) + len((a.get('boot') or {}).get('spi', []))
    moved = [k for k in sa if k != 'bw' and sa.get(k) != sb.get(k)]
    va, vb = a.get('verdict'), b.get('verdict')
    ka, kb = kinds(a.get('findings')), kinds(b.get('findings'))
    nw = (b.get('boot') or {}).get('not_witnessable')
    bw = (sa.get('bw', [None])[-1], sb.get('bw', [None])[-1])
    shift = (bw[1] - bw[0]) * 10 if None not in bw else None
    ok = va == vb and ka == kb and not moved
    stops += not ok
    lines.append(f'{"=" if ok else "STOP"} sim {g}: verdict {va}->{vb}; other kinds equal={ka == kb}; new rows {new(a.get("findings"))}->{new(b.get("findings"))}; '
                 f'modelled STEP instants equal={not moved} ({len([k for k in sa if k != "bw"])} hooked); boot word {bw[0]}->{bw[1]} (+{shift} ns); '
                 f'not_witnessable count={nw and nw.get("count")} total_ns={nw and nw.get("total_ns")}')
    if not ok: lines.append(f'    main kinds {ka}\n    branch kinds {kb}\n    moved {moved}: {[(k, sa.get(k), sb.get(k)) for k in moved]}')
for g in sorted(os.listdir(f'{D}/camp/main')):
    lm, lb = jl(f'{D}/camp/main/{g}/campaign.out'), jl(f'{D}/camp/branch/{g}/campaign.out')
    bm = {l.get('scenario'): l for l in lm}; bb = {l.get('scenario'): l for l in lb}
    for sc, x in bm.items():
        y = bb.get(sc)
        if y is None: lines.append(f'STOP camp {g}/{sc}: missing on branch'); stops += 1; continue
        if 'run_id' not in x:
            same = {k: v for k, v in x.items() if k not in SKIP} == {k: v for k, v in y.items() if k not in SKIP}
            lines.append(f'{"=" if same else "MOVED"} camp {g} summary'); stops += not same; continue
        ka, kb = kinds(x.get('findings')), kinds(y.get('findings'))
        flip = (x.get('verdict'), x.get('matched')) != (y.get('verdict'), y.get('matched'))
        other = [k for k in sorted(set(x) | set(y)) if k not in SKIP | {'findings'} and x.get(k) != y.get(k)]
        ra, rb = jf(f'{D}/camp/main/{g}/replay-{x["run_id"]}.out') or {}, jf(f'{D}/camp/branch/{g}/replay-{y["run_id"]}.out') or {}
        rd = [k for k in sorted(set(ra) | set(rb)) if k not in SKIP and ra.get(k) != rb.get(k)]
        aa, ab = jf(f'{D}/camp/main/{g}/attribute-{x["run_id"]}.out') or {}, jf(f'{D}/camp/branch/{g}/attribute-{y["run_id"]}.out') or {}
        ad = [k for k in sorted(set(aa) | set(ab)) if k not in SKIP and aa.get(k) != ab.get(k)]
        ok = not flip and ka == kb
        stops += not ok
        lines.append(f'{"=" if ok else "STOP"} camp {g}/{sc}: {x.get("verdict")}/{x.get("matched")} -> {y.get("verdict")}/{y.get("matched")}; other kinds equal={ka == kb}; '
                     f'new rows {new(x.get("findings"))}->{new(y.get("findings"))}; line moved {other}; replay moved {rd}; attribute moved {ad} ({aa.get("domain")}/{aa.get("rule")} -> {ab.get("domain")}/{ab.get("rule")})')
        if not ok: lines.append(f'    main {ka}\n    branch {kb}')
lines.append(f'stops: {stops}')
print('\n'.join(lines))
