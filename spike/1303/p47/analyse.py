"""P4.7: how KiCad 10.0.7's refill (--check-zones) joins each plane to each
via, from the refilled Gerber. Usage: analyse.py <board.kicad_pcb> <gerber dir> <stem>
For each own-net via: the fraction of 64 points on circles r = 0.32, 0.40,
0.55, 0.70 mm that lie in the zone's copper; every missing point is
attributed to a foreign via's knockout (within its pad 0.3 mm + the measured
foreign clearance + 0.01 mm), the board edge (within 0.5 + 0.01 mm), or
neither. A thermal relief would show missing points attributed to neither.
For each foreign via: the nearest zone copper to its 0.3 mm pad.
Gerber y is the board's y negated."""
import math, re, sys
import numpy as np
sys.path.insert(0, "../s4")
from pcb import Board

def regions(path):
    polys, net, inreg, poly = [], None, False, None
    for line in open(path):
        line = line.strip()
        m = re.match(r"%TO\.N,([^*]*)\*%", line)
        if m: net = m.group(1)
        if line.startswith("%TD"): net = None
        if line.startswith("G36"): inreg, poly = True, []
        elif line.startswith("G37"):
            inreg = False
            if poly: polys.append((net, np.array(poly)))
        elif inreg:
            m = re.match(r"X(-?\d+)Y(-?\d+)D0([12])\*", line)
            if m:
                x, y = int(m.group(1)) / 1e6, -int(m.group(2)) / 1e6
                if m.group(3) == "2" and poly:
                    polys.append((net, np.array(poly))); poly = []
                poly.append((x, y))
    return polys

def inside(pts, poly):
    x, y = pts[:, 0][:, None], pts[:, 1][:, None]
    x1, y1 = poly[:, 0][None, :], poly[:, 1][None, :]
    x2, y2 = np.roll(poly[:, 0], -1)[None, :], np.roll(poly[:, 1], -1)[None, :]
    cond = (y1 > y) != (y2 > y)
    with np.errstate(divide="ignore", invalid="ignore"):
        xint = (x2 - x1) * (y - y1) / (y2 - y1) + x1
    return (np.sum(cond & (x < xint), axis=1) % 2) == 1

board, gdir, stem = sys.argv[1], sys.argv[2], sys.argv[3]
b = Board(board)
x0, y0, x1, y1 = b.outline
for layer, net in (("In2_Cu", "VDD_3V3"), ("In1_Cu", "GND")):
    zone = [p for n, p in regions(f"{gdir}/{stem}-{layer}.gbr") if n == net]
    incu = lambda pts: np.any([inside(pts, p) for p in zone], axis=0)
    foreign = [v for v in b.vias if v["net"] != net]
    # foreign clearance first
    clear = []
    for v in foreign:
        for rr in np.arange(0.30, 1.5, 0.0025):
            ang = np.linspace(0, 2 * np.pi, 128, endpoint=False)
            pts = np.column_stack([v["x"] + rr * np.cos(ang), v["y"] + rr * np.sin(ang)])
            if incu(pts).any():
                clear.append(rr - 0.30); break
    fc = min(clear)
    own = [v for v in b.vias if v["net"] == net]
    solid, why = 0, {"foreign knockout": 0, "board edge": 0, "neither (a relief?)": 0}
    for v in own:
        allin = True
        for r in (0.32, 0.40, 0.55, 0.70):
            ang = np.linspace(0, 2 * np.pi, 64, endpoint=False)
            pts = np.column_stack([v["x"] + r * np.cos(ang), v["y"] + r * np.sin(ang)])
            ok = incu(pts)
            for q in pts[~ok]:
                allin = False
                if any(math.dist(q, (f["x"], f["y"])) <= 0.3 + fc + 0.01 for f in foreign):
                    why["foreign knockout"] += 1
                elif not (x0 + 0.51 <= q[0] <= x1 - 0.51 and y0 + 0.51 <= q[1] <= y1 - 0.51):
                    why["board edge"] += 1
                else:
                    why["neither (a relief?)"] += 1
        solid += allin
    print(f"== {stem} {layer} ({net}): foreign vias {len(foreign)}, nearest zone copper to a foreign via's pad: "
          f"min {fc:.4f} max {max(clear):.4f} mm; own vias {len(own)}, every sample in copper: {solid}; missing samples by cause: {why}")
