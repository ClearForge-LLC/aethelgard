#!/usr/bin/env python3
"""AET-WO-1307 S4: for each .kicad_pcb, is every zone's outline exactly the board outline (the proposed judge)?

Prints, per board: zones, and per zone: polygon count, point count, equal-as-written (emitter order),
equal-as-a-rectangle (any start corner or direction), filled_polygon present.
"""
import re, sys

for path in sys.argv[1:]:
    t = open(path).read()
    m = re.search(r'\(gr_rect\s+\(start ([-\d.]+) ([-\d.]+)\)\s+\(end ([-\d.]+) ([-\d.]+)\)[^()]*(?:\([^()]*(?:\([^()]*\))*[^()]*\)\s*)*', t)
    x0, y0, x1, y1 = map(float, m.groups()[:4])
    want = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
    zones = t.split("\n\t(zone")[1:]
    out = []
    for z in zones:
        body = z.split("\n\t(", 1)[0] if "\n\t(" in z else z
        polys = re.findall(r"\(polygon\s+\(pts((?:\s+\(xy [-\d.]+ [-\d.]+\))+)", body)
        net = re.search(r'\(net "([^"]*)"\)', body).group(1)
        layer = re.search(r'\(layer "([^"]*)"\)', body).group(1)
        rows = []
        for p in polys:
            pts = [tuple(map(float, xy)) for xy in re.findall(r"\(xy ([-\d.]+) ([-\d.]+)\)", p)]
            rot = any(pts == (want[i:] + want[:i]) or pts == (want[::-1][i:] + want[::-1][:i]) for i in range(4))
            rows.append(f"{len(pts)}pts exact={pts == want} rect={rot}")
        out.append(f"{net}@{layer}: {len(polys)} polygon(s) [{'; '.join(rows)}] filled={'filled_polygon' in body}")
    print(f"{path}: outline ({x0},{y0})-({x1},{y1}); " + (" | ".join(out) or "no zones"))
