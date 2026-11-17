#!/usr/bin/env python3
"""tp_dist.py <dump.json> [ref...]: for each test pad (Connector:TestPoint) on a placed board, the
distance from its pad to the nearest other pad of its net (any part); for a GND pad, also the
distance to the nearest signal test pad, and to each named ref. Prints a summary line first:
signal pads' median and max distance to their net (mm)."""
import json
import math
import statistics
import sys

d = json.load(open(sys.argv[1]))
named = sys.argv[2:]
pads = [(p["ref"], p["lib"], x, y, n) for p in d["parts"] for x, y, n in p["pads"]]
tps = [(r, x, y, n) for r, lib, x, y, n in pads if lib == "Connector:TestPoint"]
dist = lambda a, b: math.hypot(a[0] - b[0], a[1] - b[1]) / 1e6
rows, sig = [], []
for r, x, y, n in tps:
    others = [(px, py) for rr, lib, px, py, nn in pads if nn == n and rr != r]
    near = min((dist((x, y), o) for o in others), default=float("nan"))
    if n == "GND":
        sp = [(rr, (xx, yy)) for rr, xx, yy, nn in tps if nn != "GND"]
        ns = min((dist((x, y), q) for _, q in sp), default=float("nan"))
        extra = " ".join(f"{rr}={dist((x, y), q):.1f}" for rr, q in sp if rr in named)
        rows.append(f"{r} GND nearest-signal-TP {ns:.1f} {extra}")
    else:
        sig.append(near)
        rows.append(f"{r} {n} {near:.1f}")
print(f"signal test pads: {len(sig)}, median {statistics.median(sig):.1f} mm, max {max(sig):.1f} mm to their net's nearest other pad")
for row in sorted(rows):
    print("  " + row)
