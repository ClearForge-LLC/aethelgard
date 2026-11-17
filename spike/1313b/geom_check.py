#!/usr/bin/env python3
"""geom_check.py <layout.json>...: for each layout dumped by the spike's placer, check that every
keep-out box is inside the outline, that no two keep-out boxes overlap, and that every connector's
courtyard has its cable side (rotation 0: +y, 90: +x, 180: -y, 270: -x) on the outline's edge, within
one 50 um snap. Prints one line per layout; exit 1 if any check fails."""
import json
import sys

SNAP = 50_000
bad = 0
for path in sys.argv[1:]:
    d = json.load(open(path))
    x0, y0, x1, y1 = d["outline"]
    errs = []
    ps = d["parts"]
    for p in ps:
        k = p["keep"]
        if k[0] < x0 or k[1] < y0 or k[2] > x1 or k[3] > y1:
            errs.append(f"{p['ref']} outside")
    for i in range(len(ps)):
        a = ps[i]["keep"]
        for j in range(i + 1, len(ps)):
            b = ps[j]["keep"]
            if a[0] < b[2] and b[0] < a[2] and a[1] < b[3] and b[1] < a[3]:
                errs.append(f"{ps[i]['ref']}/{ps[j]['ref']} overlap")
    sides = {}
    for p in ps:
        if p["kind"] != "connector":
            continue
        c = p["court"]
        gap = {0: y1 - c[3], 90: x1 - c[2], 180: c[1] - y0, 270: c[0] - x0}[p["rot"]]
        sides.setdefault(p["rot"], []).append(p["ref"])
        if not 0 <= gap <= SNAP:
            errs.append(f"{p['ref']} cable side {gap} nm from its edge")
    names = {0: "bottom", 90: "right", 180: "top", 270: "left"}
    print(f"{path}: {'OK' if not errs else 'FAIL ' + '; '.join(errs)}; "
          + ", ".join(f"{names[r]} {' '.join(v)}" for r, v in sorted(sides.items())))
    bad += bool(errs)
sys.exit(1 if bad else 0)
