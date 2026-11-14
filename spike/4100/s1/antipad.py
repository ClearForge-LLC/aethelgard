#!/usr/bin/env python3
"""S1: the gap from a refilled plane's copper to each foreign via's disk, read from KiCad's Gerber.

Usage: antipad.py <board.kicad_pcb> <layer.gbr> <plane-net>
Reads the G36/G37 regions (linear D01 only; an arc is refused), takes every via of another net
from the board, and prints the smallest distance from a region edge to a via centre, less the
via's radius (0.3 mm). Units: the file's %FSLAX46Y46% (mm, 6 decimals) are checked.
"""
import math, re, sys

board, gbr, net = sys.argv[1], sys.argv[2], sys.argv[3]
text = open(board).read()
vias = [(float(x), float(y), n) for x, y, n in re.findall(
    r'\t\(via\n\t\t\(at ([\d.]+) ([\d.]+)\)\n(?:\t\t[^\n]*\n)*?\t\t\(net "([^"]+)"\)', text)]
src = open(gbr).read()
assert "%FSLAX46Y46*%" in src and "%MOMM*%" in src
assert "G02" not in src and "G03" not in src, "arcs in the file"
regions, cur, inside, pos = [], [], False, (0, 0)
for line in src.splitlines():
    if line.startswith("G36"):
        inside, cur = True, []
    elif line.startswith("G37"):
        inside = False
        regions.append(cur)
    m = re.match(r"X(-?\d+)Y(-?\d+)D0([12])\*", line)
    if m and inside:
        p = (int(m.group(1)) / 1e6, -int(m.group(2)) / 1e6)  # Gerber Y is up; KiCad's is down
        if m.group(3) == "2":
            cur.append([p])
        else:
            cur[-1].append(p)

def seg_dist(p, a, b):
    ax, ay = a; bx, by = b; px, py = p
    dx, dy = bx - ax, by - ay
    L = dx * dx + dy * dy
    t = 0 if L == 0 else max(0, min(1, ((px - ax) * dx + (py - ay) * dy) / L))
    return math.hypot(px - (ax + t * dx), py - (ay + t * dy))

def inside_poly(p, poly):
    x, y, c = p[0], p[1], False
    for (ax, ay), (bx, by) in zip(poly, poly[1:] + poly[:1]):
        if (ay > y) != (by > y) and x < (bx - ax) * (y - ay) / (by - ay) + ax:
            c = not c
    return c

foreign = [v for v in vias if v[2] != net]
best, n_regions = None, len(regions)
for x, y, n in foreign:
    for r in regions:
        for poly in r:
            for a, b in zip(poly, poly[1:]):
                d = seg_dist((x, y), a, b) - 0.3
                if best is None or d < best[0]:
                    best = (d, n, x, y)
print(f"{gbr}: {n_regions} regions, {len(foreign)} foreign vias, smallest copper-to-via-disk gap "
      f"{best[0]:.4f} mm (via {best[1]} at {best[2]} {best[3]})" if best else "no regions")
