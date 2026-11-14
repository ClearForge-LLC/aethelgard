#!/usr/bin/env python3
"""AET-WO-1307: via count and total track length (mm) of a .kicad_pcb, as `<vias> <mm>`."""
import math, re, sys

t = open(sys.argv[1]).read()
vias = len(re.findall(r"^\s*\(via\b", t, re.M))
mm = 0.0
for s, e in re.findall(r"\(segment\s+\(start ([-\d.]+ [-\d.]+)\)\s+\(end ([-\d.]+ [-\d.]+)\)", t):
    (x0, y0), (x1, y1) = (map(float, s.split()), map(float, e.split()))
    mm += math.hypot(x1 - x0, y1 - y0)
print(vias, f"{mm:.1f}")
