#!/usr/bin/env python3
"""AET-WO-2309 §1.1 sweep C: read the instrumentation logs (sh/sweep-instrument.diff applied to main).
Each `check_run` line is `check_run <input sha256> <test thread> <name>=<value>... <main's Ok/Err>`,
where the names are the candidate rules (`ok` or `REFUSES`) and the input's `read` and `booted`.
A CLI the tests spawn logs with thread `main`. For each candidate rule, lists the distinct inputs it
refuses, split into those main accepted ("accepted before, refused after") and those main already
refused, by test; then the inputs by read and booted.

Usage: sweep-c.py LOG...
"""
import collections
import re
import sys

rows = set()
for log in sys.argv[1:]:
    for line in open(log):
        tok = line.rstrip("\n").split(" ")
        if tok[0] != "check_run":
            continue
        key, thread, i, fields = tok[1], tok[2], 3, {}
        while i < len(tok) and re.match(r"^[a-z_]+=", tok[i]):
            k, v = tok[i].split("=", 1)
            fields[k] = v
            i += 1
        rows.add((key, thread, tuple(sorted(fields.items())), " ".join(tok[i:])))
inputs = {r[0] for r in rows}
print(f"check_run: {len(rows)} distinct (input, test) pairs; {len(inputs)} distinct inputs; "
      f"{sum(1 for r in rows if r[1] == 'main')} pairs from a spawned CLI")
by_input = {r[0]: dict(r[2]) for r in rows}
# v2 logs carry no `read` or `booted` (v3 added them): printed only when every input has them.
if all("read" in f and "booted" in f for f in by_input.values()):
    rb = collections.Counter((f["read"], f["booted"]) for f in by_input.values())
    print("distinct inputs by (read, booted):", dict(rb))
names = sorted({k for r in rows for k, _ in r[2]} - {"read", "booted"})
for name in names:
    refused = [r for r in rows if dict(r[2]).get(name) == "REFUSES"]
    acc = collections.Counter(r[1] for r in refused if r[3].startswith("Ok"))
    rej = collections.defaultdict(collections.Counter)
    for r in refused:
        if not r[3].startswith("Ok"):
            rej[r[1]][r[3]] += 1
    print(f"\n## {name}: refuses {len({r[0] for r in refused})} distinct inputs")
    print(f"accepted before, refused after: {sum(acc.values())} (input, test) pairs in {len(acc)} tests")
    for t, n in sorted(acc.items()):
        print(f"  {t}: {n}")
    print(f"refused before already, by main's reason: {sum(sum(v.values()) for v in rej.values())} (input, test) pairs in {len(rej)} tests")
    for t, v in sorted(rej.items()):
        for reason, n in sorted(v.items()):
            print(f"  {t}: {n} x {reason}")
