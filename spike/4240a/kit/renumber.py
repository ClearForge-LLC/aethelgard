"""AET-WO-4240a: renumber the specification (FROM -> TO) and the solved topology (SFROM -> STO).
Version lines only: each committed specification's "schema_version", each stored topology's, the
solved JSON embedded in Rust sources, and the tests' version pins. Usage:
  python3 -I renumber.py REPO FROM TO SFROM STO     (4240a: 19 20 12 13; after 1210: 20 21 13 13)"""
import glob, json, os, re, sys
repo, F, T, SF, ST = sys.argv[1], *map(int, sys.argv[2:6])
def skip(rel): return rel.startswith(('spike/', 'target/', 'fab/', 'docs/')) or '/target/' in rel
n = {'spec': 0, 'topology': 0}
for p in sorted(glob.glob(os.path.join(repo, '**/*.json'), recursive=True)):
    rel = os.path.relpath(p, repo)
    if skip(rel): continue
    try: d = json.load(open(p))
    except Exception: continue
    if not isinstance(d, dict): continue
    if isinstance(d.get('mcu'), dict) and d.get('schema_version') == F: kind, a, b = 'spec', F, T
    elif 'assignments' in d and 'specification_sha256' in d and d.get('schema_version') == SF and SF != ST: kind, a, b = 'topology', SF, ST
    else: continue
    s = open(p).read(); s2, k = re.subn(r'("schema_version": *)%d,' % a, r'\g<1>%d,' % b, s, count=1); assert k == 1, rel
    open(p, 'w').write(s2); n[kind] += 1
print(n)
for p in sorted(glob.glob(os.path.join(repo, 'crates/**/*.rs'), recursive=True)):
    s = open(p).read(); s2 = s
    if SF != ST:
        s2 = s2.replace('edited("\\"schema_version\\":%d", "\\"schema_version\\":%d")' % (SF, ST),
                        'edited("\\"schema_version\\":%d", "\\"schema_version\\":%d")' % (ST, ST + 1))
        s2 = s2.replace('{"schema_version":%d,"specification_sha256"' % SF, '{"schema_version":%d,"specification_sha256"' % ST)
    if '/tests/' in p or p.endswith('tests.rs'):
        s2 = s2.replace('SPECIFICATION_SCHEMA_VERSION, %d)' % F, 'SPECIFICATION_SCHEMA_VERSION, %d)' % T)
        s2 = re.sub(r'expected: %d$' % F, 'expected: %d' % T, s2, flags=re.M)
        s2 = s2.replace('not %d"' % F, 'not %d"' % T)
    if s2 != s: open(p, 'w').write(s2); print('rs', os.path.relpath(p, repo))
