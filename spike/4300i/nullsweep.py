"""AET-WO-4300i §1.4, the null sweep: out/baseline.tsv (f50c29f4) against
out/after/baseline.tsv (the build), per specification: exit codes and kinds,
the solve's JSON paths that moved beyond specification_sha256, and whether
the emitted tree moved. A move is expected only on a motor board (emit) and
on Sentinel-03's MOTOR1 pin (solve). usage: nullsweep.py <base scratch> <after scratch>"""
import csv, json, os, sys
B, A = sys.argv[1], sys.argv[2]
here = os.path.dirname(os.path.abspath(__file__))
def load(p): return {r['name']: r for r in csv.DictReader(open(p), delimiter='\t')}
b = load(f'{here}/out/baseline.tsv'); a = load(f'{here}/out/after/baseline.tsv')
motor = {r['board'] for r in csv.DictReader(open(f'{here}/out/motors.tsv'), delimiter='\t') if r['drawn'] == 'yes'}
keys = ['validate_rc', 'validate_kind', 'solve_rc', 'solve_kind', 'emit_rc', 'emit_kind', 'emit_files']
print('name\tcodes_same\tsolve_moved\temit\trepeat\tmotor_board\texpected')
bad = 0
for n in b:
    x, y = b[n], a[n]
    codes = all(x[k] == y[k] for k in keys)
    sd = '-'
    pb, pa = f'{B}/solve/{n}.json', f'{A}/solve/{n}.json'
    if x['solve_rc'] == '0' and os.path.exists(pb) and os.path.exists(pa):
        diffs = []
        def walk(u, v, path):
            if isinstance(u, dict) and isinstance(v, dict):
                for k in sorted(set(u) | set(v)): walk(u.get(k), v.get(k), f'{path}/{k}')
            elif u != v: diffs.append(path)
        walk(json.load(open(pb)), json.load(open(pa)), '')
        sd = ','.join(d for d in diffs if d != '/specification_sha256') or 'sha-only'
    emit = 'same' if x['emit_tree_sha256'] == y['emit_tree_sha256'] else 'moved'
    ok = codes and y['repeat_identical'] == 'yes' and (emit == 'moved') == (n in motor) \
        and sd in ('-', 'sha-only', *(('/assignments/MOTOR1/pin',) if n == 'sentinel-03' else ()))
    bad += not ok
    print(f"{n}\t{codes}\t{sd}\t{emit}\t{y['repeat_identical']}\t{n in motor}\t{'yes' if ok else 'NO'}")
sys.exit(1 if bad else 0)
