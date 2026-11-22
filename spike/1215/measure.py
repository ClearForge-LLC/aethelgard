"""AET-WO-1215 §1.1: three mechanical witnesses per specification version bump, read from git.

For each commit that set `SPECIFICATION_SCHEMA_VERSION` to N (N = 2..current), against its parent:
  keys    - the union of JSON key paths over every committed specification (a JSON object with an
            `mcu` object and a `schema_version`) at N in the commit, less those at N-1 in the parent
            (`[]` marks an array element), and the reverse;
  kinds   - refusal kind names (`=> "Name"` in crates/aethelgard-ir/src) added in the window from the
            previous bump commit to this one, so a rule landed by a work order that did not bump is seen,
            and of those the ones already present in the bump's parent (landed by another commit);
  fields  - Deserialize struct fields in crates/aethelgard-ir/src (`Struct.field`, `?` when the field
            carries a serde `default`), added or removed across the bump commit.
Usage: python3 -I spike/1215/measure.py REPO > spike/1215/out/witnesses.txt
"""
import json, re, subprocess, sys

repo = sys.argv[1]


def git(*a):
    return subprocess.run(['git', '-C', repo, *a], check=True, capture_output=True).stdout


def bumps():
    out = git('log', '--reverse', '--format=%H', '-G', 'SPECIFICATION_SCHEMA_VERSION: u32 = ',
              '--', 'crates/aethelgard-ir/src/lib.rs').decode().split()
    rows = []
    for c in out:
        m = re.search(rb'SPECIFICATION_SCHEMA_VERSION: u32 = (\d+)', git('show', f'{c}:crates/aethelgard-ir/src/lib.rs'))
        rows.append((int(m.group(1)), c))
    return rows


def skip(rel):
    return rel.startswith(('spike/', 'target/', 'fab/', 'docs/')) or '/target/' in rel


def files(c, pred):
    return [p for p in git('ls-tree', '-r', '--name-only', c).decode().splitlines() if pred(p) and not skip(p)]


def paths(v, pre=''):
    out = set()
    if isinstance(v, dict):
        for k, x in v.items():
            p = f'{pre}.{k}' if pre else k
            out.add(p)
            out |= paths(x, p)
    elif isinstance(v, list):
        for x in v:
            out |= paths(x, pre + '[]')
    return out


def spec_paths(c, version):
    union, n = set(), 0
    for p in files(c, lambda p: p.endswith('.json')):
        try:
            d = json.loads(git('show', f'{c}:{p}'))
        except Exception:
            continue
        if isinstance(d, dict) and isinstance(d.get('mcu'), dict) and d.get('schema_version') == version:
            union |= paths(d)
            n += 1
    return union, n


def ir_sources(c):
    return [git('show', f'{c}:{p}').decode() for p in files(c, lambda p: p.startswith('crates/aethelgard-ir/src/') and p.endswith('.rs'))]


def kinds(c):
    return {m for s in ir_sources(c) for m in re.findall(r'=> "([A-Z][A-Za-z0-9]+)"', s)}


STRUCT = re.compile(r'#\[derive\(([^)]*)\)\]\s*((?:#\[[^\]]*\]\s*)*)(?:pub(?:\([a-z]+\))? )?struct (\w+)\s*\{(.*?)\n\}', re.S)
FIELD = re.compile(r'((?:\s*#\[[^\]]*\]\s*|\s*///[^\n]*\n)*)\s*(?:pub(?:\([a-z]+\))? )?(r#)?(\w+):\s')


def fields(c):
    out = set()
    for s in ir_sources(c):
        for m in STRUCT.finditer(s):
            if 'Deserialize' not in m.group(1):
                continue
            body = m.group(4)
            for line_attrs, _, name in FIELD.findall(body):
                opt = '?' if 'default' in line_attrs else ''
                out.add(f'{m.group(3)}.{name}{opt}')
    return out


rows = bumps()
prev = None
for n, c in rows:
    parent = c + '^'
    a, na = spec_paths(c, n)
    b, nb = spec_paths(parent, n - 1)
    print(f'## version {n}  {c[:10]}  {git("log", "-1", "--format=%s", c).decode()[:110].strip()}')
    print(f'   specifications: {nb} at {n - 1} in parent, {na} at {n} in commit')
    if nb and na:
        print('   keys +', ' '.join(sorted(a - b)) or '-')
        print('   keys -', ' '.join(sorted(b - a)) or '-')
    k0 = kinds(prev if prev else parent)
    k1 = kinds(c)
    print('   kinds + (window)', ' '.join(sorted(k1 - k0)) or '-')
    print('   kinds - (window)', ' '.join(sorted(k0 - k1)) or '-')
    kb = kinds(parent)
    print('   kinds + (window, by a commit other than the bump)', ' '.join(sorted((kb - k0) & k1)) or '-')
    f0, f1 = fields(parent), fields(c)
    print('   fields +', ' '.join(sorted(f1 - f0)) or '-')
    print('   fields -', ' '.join(sorted(f0 - f1)) or '-')
    prev = c
