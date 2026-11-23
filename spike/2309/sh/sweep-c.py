#!/usr/bin/env python3
"""AET-WO-2309 §1.1 sweep C: read the v2 instrumentation log (sh/sweep-instrument.diff applied to main,
`cargo test --workspace --no-fail-fast`). Each `check_run` line is
`check_run <input sha256> <test thread> motor=.. full=.. status=.. <main's Ok/Err>`.
For each candidate rule, lists the distinct inputs it refuses, split into those main accepted
("accepted before, refused after") and those main refused already, by test.

Usage: sweep-c.py LOG
"""
import collections
import sys

rows = set()
for line in open(sys.argv[1]):
    parts = line.rstrip("\n").split(" ", 6)
    if parts[0] != "check_run":
        continue
    _, key, thread, motor, full, status, result = parts
    rows.add((key, thread, motor, full, status, result))
inputs = {r[0] for r in rows}
print(f"check_run calls logged: {len(rows)} distinct (input, test) pairs; {len(inputs)} distinct inputs")
for i, name in [(2, "motor"), (3, "full"), (4, "status")]:
    refused = [r for r in rows if r[i].endswith("REFUSES")]
    acc = collections.Counter(r[1] for r in refused if r[5].startswith("Ok"))
    rej = collections.defaultdict(collections.Counter)
    for r in refused:
        if not r[5].startswith("Ok"):
            rej[r[1]][r[5]] += 1
    print(f"\n## {name}: refuses {len({r[0] for r in refused})} distinct inputs")
    print(f"accepted before, refused after: {sum(acc.values())} (input, test) pairs in {len(acc)} tests")
    for t, n in sorted(acc.items()):
        print(f"  {t}: {n}")
    print(f"refused before already, by main's reason: {sum(sum(v.values()) for v in rej.values())} (input, test) pairs in {len(rej)} tests")
    for t, v in sorted(rej.items()):
        for reason, n in sorted(v.items()):
            print(f"  {t}: {n} x {reason}")
