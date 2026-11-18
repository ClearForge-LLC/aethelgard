"""AET-WO-4400f: the leaf paths where two simulate verdicts differ (python3 -I diffv.py A B [MAX]).
Lists are compared by index; each differing leaf prints once, numbers as A -> B."""
import json, sys
a, b = json.load(open(sys.argv[1])), json.load(open(sys.argv[2]))
cap = int(sys.argv[3]) if len(sys.argv) > 3 else 40
out = []
def walk(x, y, p):
    if type(x) != type(y):
        out.append((p, x, y)); return
    if isinstance(x, dict):
        for k in sorted(set(x) | set(y)):
            walk(x.get(k), y.get(k), f"{p}.{k}")
    elif isinstance(x, list):
        if len(x) != len(y):
            out.append((p + ".len", len(x), len(y)))
        for i, (u, v) in enumerate(zip(x, y)):
            walk(u, v, f"{p}[{i}]")
    elif x != y:
        out.append((p, x, y))
walk(a, b, "")
print(f"{len(out)} differing leaves")
for p, x, y in out[:cap]:
    print(f"  {p}: {str(x)[:90]!s} -> {str(y)[:90]!s}")
