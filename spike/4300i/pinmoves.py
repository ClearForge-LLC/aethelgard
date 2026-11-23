"""AET-WO-4300i §7: every pin-bearing field of each solve against the baseline solve.
A pin-bearing field is any JSON value under a key named `pin` or `pins`, or a string shaped P<port><n>, anywhere.
usage: python3 -I pinmoves.py <base solve dir> <after solve dir> -> out/pin-moves.tsv on stdout"""
import json, os, re, sys
base, after = sys.argv[1:3]
PIN = re.compile(r"P[A-K]\d{1,2}$")
def pins(v, at, out):
    if isinstance(v, dict):
        for k, c in v.items():
            p = f"{at}/{k}"
            if k in ("pin", "pins") or (isinstance(c, str) and PIN.match(c)):
                out[p] = json.dumps(c, sort_keys=True)
            pins(c, p, out)
    elif isinstance(v, list):
        for i, c in enumerate(v):
            p = f"{at}/{i}"
            if isinstance(c, str) and PIN.match(c):
                out[p] = c
            pins(c, p, out)
print("board\tpath\tbase\tafter\tallowed")
bad = 0
names = sorted(f[:-5] for f in os.listdir(base) if f.endswith(".json"))
assert sorted(f[:-5] for f in os.listdir(after) if f.endswith(".json")) == names, "solved sets differ"
fields = 0
for n in names:
    a, b = {}, {}
    pins(json.load(open(os.path.join(base, n + ".json"))), "", a)
    pins(json.load(open(os.path.join(after, n + ".json"))), "", b)
    fields += len(a)
    for p in sorted(set(a) | set(b)):
        if a.get(p) != b.get(p):
            ok = n == "sentinel-03" and p == "/assignments/MOTOR1/pin" and a.get(p) == '"PE9"' and b.get(p) == '"PA8"'
            bad += not ok
            print(f"{n}\t{p}\t{a.get(p, '-')}\t{b.get(p, '-')}\t{'yes' if ok else 'NO'}")
print(f"# {len(names)} solves, {fields} pin-bearing fields compared, {bad} disallowed moves", file=sys.stderr)
sys.exit(1 if bad else 0)
