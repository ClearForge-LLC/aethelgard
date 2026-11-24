#!/usr/bin/env python3 -I
"""AET-WO-4300k key-deletion sweep.  Usage: python3 -I sweep.py [--only SUBSTR] [--no-speeds]
Reads keys.json (from enumerate.py), builds carriers, applies delete/null/empty/rename per key,
runs validate, solve, emit, extract, check speeds on host, diffs against the carrier baseline.
Writes results.json next to keys.json under /home/forge/.cache/aet-4300k/sweep/."""
import json, os, re, subprocess, sys, copy, shutil, hashlib
from concurrent.futures import ThreadPoolExecutor
ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '../../..'))
CACHE = '/home/forge/.cache/aet-4300k/sweep'
A = os.environ.get('AET_BIN') or os.path.join(ROOT, 'target/debug/aethelgard')
ONLY = None; SPEEDS = True; PATHS = None; ALLC = False; SYNTH = False; OUT = 'results'
args = sys.argv[1:]
if '--only' in args: ONLY = args[args.index('--only') + 1]
if '--no-speeds' in args: SPEEDS = False
if '--paths' in args: PATHS = args[args.index('--paths') + 1].split(',')
if '--all-carriers' in args: ALLC = True
if '--synth' in args: SYNTH = True
if '--out' in args: OUT = args[args.index('--out') + 1]
S03 = 'crates/aethelgard-cli/tests/fixtures/sentinel-03.json'

def load(p): return json.load(open(os.path.join(ROOT, p)))
universe = [l.rstrip('\n').split('\t') for l in open(CACHE + '/universe.tsv')]

# ---------- path handling ----------
def toks(path):
    out = []
    for t in path.split('  ')[0].split('.'):
        m = re.match(r'(\w+)(\[\]|\{\})?$', t)
        out.append((m.group(1), m.group(2)))
    return out

def containers(spec, path):
    """Yield (container_dict, key) for each place the path's final key may sit."""
    ts = toks(path)
    cur = [spec]
    for name, kind in ts[:-1]:
        nxt = []
        for c in cur:
            if isinstance(c, dict) and name in c:
                v = c[name]
                if kind == '[]' and isinstance(v, list): nxt += [x for x in v if isinstance(x, dict)]
                elif kind == '{}' and isinstance(v, dict): nxt += [x for x in v.values() if isinstance(x, dict)]
                elif kind is None and isinstance(v, dict): nxt.append(v)
        cur = nxt
    return [(c, ts[-1][0]) for c in cur if isinstance(c, dict) and ts[-1][0] in c]

def declared(spec, path):
    for c, k in containers(spec, path):
        v = c[k]
        if v is None or v == [] or v == {}: continue
        return True
    return False

def apply(spec, path, variant):
    s = copy.deepcopy(spec); n = 0
    for c, k in containers(s, path):
        v = c[k]
        if variant == 'delete': del c[k]
        elif variant == 'null': c[k] = None
        elif variant == 'empty':
            if isinstance(v, list): c[k] = []
            elif isinstance(v, dict): c[k] = {}
            else: return None
        elif variant == 'rename': c[k + '_x'] = c.pop(k)
        n += 1
    return s if n else None

def gated(v, limit=165):
    v = copy.deepcopy(v); st = v['board']['stackup']
    st['outer_copper_min_um'] = {"value": 30, "source": "test: 1 oz foil plus plating, at least"}
    st['plane_copper_min_um'] = {"value": 15, "source": "test: 1/2 oz foil, at least"}
    st['plating_min_um'] = {"value": 18, "source": "test: the fabricator's average hole plating"}
    st['copper_temperature_max_c'] = {"value": 125, "source": "test: the load's junction maximum"}
    v['power']['drop_limits'] = [{"domain": "VDD_3V3", "max_millivolts": limit, "source": "test: the limit"}]
    return v

# ---------- running ----------
def run(cmd, timeout=1800):
    try:
        p = subprocess.run(cmd, stdin=subprocess.DEVNULL, capture_output=True, text=True, timeout=timeout, cwd=ROOT)
        return p.returncode, p.stdout, p.stderr
    except subprocess.TimeoutExpired:
        return 124, '', 'TIMEOUT'

def parse(out):
    ls = []
    for l in out.splitlines():
        if not l.strip(): continue
        try: ls.append(json.loads(l))
        except Exception: ls.append({'_raw': l})
    return ls

def pipeline(spec, label):
    d = f'{CACHE}/w/{label}'; shutil.rmtree(d, ignore_errors=True); os.makedirs(d)
    sp = d + '/spec.json'; open(sp, 'w').write(json.dumps(spec))
    res = {}
    def go(name, cmd):
        rc, o, e = run(cmd); res[name] = dict(exit=rc, out=parse(o), err=e.splitlines()); return rc
    if go('validate', [A, 'validate', '--file', sp]) == 0:
        go('solve', [A, 'solve', '--file', sp])
        if go('emit', [A, 'emit', '--file', sp, '--out', d + '/e']) == 0:
            pcb = [f for f in os.listdir(d + '/e') if f.endswith('.kicad_pcb')]
            if pcb: go('extract', [A, 'extract', '--board', d + '/e/' + pcb[0], '--spec', sp])
        if SPEEDS: go('speeds', [A, 'check', 'speeds', '--spec', sp])
    shutil.rmtree(d, ignore_errors=True)
    return res

# ---------- facts ----------
IDK = ('net', 'board_net', 'domain', 'name', 'id', 'bus', 'key', 'ref', 'part', 'signal', 'rule', 'check', 'pin', 'label', 'device')
SKIP = re.compile(r'(^|_)(sha256|path)$|^path$|elapsed|duration')
def flat(v, path, out):
    if isinstance(v, dict):
        for k, x in v.items():
            if SKIP.search(k): continue
            flat(x, path + '.' + k, out)
    elif isinstance(v, list):
        for i, x in enumerate(v):
            if isinstance(x, dict):
                ident = next((str(x[k]) for k in IDK if k in x and isinstance(x[k], (str, int))), None)
                flat(x, f'{path}[{ident if ident is not None else i}]', out)
            elif isinstance(x, list):
                flat(x, f'{path}[{i}]', out)
            else:
                out.setdefault(path + '[*]', set()).add(json.dumps(x))
        if not v: out.setdefault(path + '[]', set()).add('empty')
    else:
        out.setdefault(path, set()).add(json.dumps(v))

def facts(res):
    out = {}
    for cmd, r in res.items():
        out.setdefault(f'{cmd}:exit', set()).add(str(r['exit']))
        for i, l in enumerate(r['out']):
            flat(l, f'{cmd}:{i}', out)
        for l in r['err']: out.setdefault(f'{cmd}:stderr', set()).add(json.dumps(l))
    return out

def diff(base, var):
    d = []
    for p in sorted(set(base) | set(var)):
        b, v = base.get(p), var.get(p)
        if b == v: continue
        if v is None: d.append(('gone', p, sorted(b)))
        elif b is None: d.append(('new', p, sorted(v)))
        else:
            rem, add = sorted(b - v), sorted(v - b)
            if len(b) == 1 and len(v) == 1: d.append(('chg', p, rem + add))
            else:
                if rem: d.append(('rem', p, rem))
                if add: d.append(('add', p, add))
    return d

# ---------- main ----------
keys = json.load(open(CACHE + '/keys.json'))
if ONLY: keys = [k for k in keys if ONLY in k['path']]
if PATHS: keys = [k for k in keys if k['path'] in PATHS]
s03 = load(S03)
carriers = {'sentinel-03': s03, 'sentinel-03+gated': gated(s03)}
uni = {}
for n, p in universe:
    uni[n] = p
SYN = {}   # synthetic carriers: name -> (spec, [paths])
def synth():
    a = copy.deepcopy(s03); a['net_constraints'][0]['impedance_ohms'] = 50
    SYN['syn:s03+impedance_ohms=50'] = (a, ['net_constraints[].impedance_ohms'])
    b = copy.deepcopy(s03)
    for dl in b['board']['stackup']['dielectrics']: dl['max_thickness_um'] = dl['min_thickness_um'] + 40
    SYN['syn:s03+dielectric max_thickness_um=min+40'] = (b, ['board.stackup.dielectrics[].max_thickness_um'])
    SYN['syn:gated+dielectric max_thickness_um=min+40'] = (gated(b), ['board.stackup.dielectrics[].max_thickness_um'])
    c = copy.deepcopy(s03)
    for sp in c['serial_ports']:
        sp['peripheral'] = {'CONSOLE': 'USART1', 'CRSF': 'USART2', 'MAVLINK': 'USART3'}[sp['label']]
        if sp['role'] == 'rc_crsf': sp['negotiated'] = {'source': 'test: CRSF frame 0x70 proposal'}; sp['framing']['baud'] = 921600
    SYN['syn:s03+serial peripheral,negotiated'] = (c, ['serial_ports[].peripheral', 'serial_ports[].negotiated'])
    d = load('crates/aethelgard-ir/tests/fixtures/d1-drv8876.json')
    d['pin_devices'][0]['part_limits']['input_pwm_max_hz'] = {'value': 100000, 'source': 'test: a declared limit'}
    SYN['syn:d1-drv8876+input_pwm_max_hz'] = (d, ['pin_devices[].part_limits.input_pwm_max_hz'])
def carrier_for(path):
    out = []
    if declared(s03, path): out.append('sentinel-03')
    if not out or ALLC:
        for n, p in universe:
            if n == 'sentinel-03': continue
            sp = load(p)
            if declared(sp, path):
                carriers[n] = sp; out.append(n)
                if not ALLC or len(out) >= 4: break
    for n, (sp, ps) in SYN.items():
        if path in ps: carriers[n] = sp; out.append(n)
    return out
synth()
GATED = {'board.stackup.fab_tolerances', 'board.stackup.fab_tolerances.hole_position_um', 'board.stackup.fab_tolerances.hole_size_plus_um', 'board.stackup.fab_tolerances.hole_size_minus_um', 'board.stackup.fab_tolerances.track_width_permille', 'board.stackup.fab_tolerances.via_ring_um', 'power.drop_limits[]', 'board.stackup.outer_copper_min_um', 'board.stackup.plane_copper_min_um',
         'board.stackup.plating_min_um', 'board.stackup.copper_temperature_max_c'}
jobs = []
for k in keys:
    cs = carrier_for(k['path'])
    if k['path'] in GATED: cs = cs + ['sentinel-03+gated']
    if not cs: jobs.append((k, None, None)); continue
    for c in cs:
        for v in ('delete', 'null', 'empty', 'rename'):
            jobs.append((k, c, v))
bases = {}
def base(c):
    if c not in bases:
        bases[c] = pipeline(carriers[c], 'base-' + re.sub(r'\W', '_', c))
    return bases[c]
needed = sorted({c for _, c, _ in jobs if c})
with ThreadPoolExecutor(3) as ex: list(ex.map(base, needed))
print('baselines:', {c: {k: v['exit'] for k, v in bases[c].items()} for c in needed}, flush=True)

def work(j):
    k, c, v = j
    if c is None: return dict(path=k['path'], carrier=None)
    spec = apply(carriers[c], k['path'], v)
    r = dict(path=k['path'], carrier=c, variant=v, file=k['file'], line=k['line'], kinds=k['kinds'])
    if spec is None: r['na'] = True; return r
    lab = hashlib.sha1((k['path'] + c + v).encode()).hexdigest()[:12]
    res = pipeline(spec, lab)
    r['exits'] = {n: x['exit'] for n, x in res.items()}
    val = res['validate']
    if val['exit'] != 0:
        e = val['out'][0].get('error', {}) if val['out'] else {}
        r['refused'] = dict(kind=e.get('kind'), message=(e.get('message') or '')[:240])
    else:
        r['diff'] = diff(facts(base(c)), facts(res))
    return r
with ThreadPoolExecutor(6) as ex: results = list(ex.map(work, jobs))
json.dump(results, open(CACHE + '/' + OUT + '.json', 'w'), indent=1)
print('done', len(results))
