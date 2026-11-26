"""AET-WO-4400j (1), §5 "a growth fix that changes behaviour": two verdicts of the same cell (main's /dev/null stdin
against the held-open pipe) compared field by field; prints every path whose value differs, so the only differences
can be read off (memory, wall, the host's MIPS reading). Usage: same.py VERDICT_A VERDICT_B"""
import json, sys
a, b = (json.load(open(p)) for p in sys.argv[1:3])
diffs = []
def walk(x, y, path):
    if isinstance(x, dict) and isinstance(y, dict):
        for k in sorted(set(x) | set(y)):
            walk(x.get(k, "<absent>"), y.get(k, "<absent>"), f"{path}.{k}")
    elif isinstance(x, list) and isinstance(y, list) and len(x) == len(y):
        for i, (p, q) in enumerate(zip(x, y)):
            walk(p, q, f"{path}[{i}]")
    elif x != y:
        diffs.append((path, str(x)[:80], str(y)[:80]))
walk(a, b, "")
for p, x, y in diffs: print(f"{p}: {x}  ->  {y}")
print(f"{len(diffs)} differing paths")
