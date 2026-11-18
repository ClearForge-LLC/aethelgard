"""AET-WO-1303b: every cell and junction square the supply model counts as copper, checked
against an independent reading of the board (spike/1303/s4's pcb.py, as modified for 1303b:
widths, pads and via sizes as the file states them) and, on a plane, KiCad's refilled region.

Usage: check_copper.py <dump> <board.kicad_pcb> <net> [<gerber dir>]
<dump>: supply_stop's AET_DUMP_CELLS output (`cell <layer> x0 y0 side`, `square <layer> cx cy a drill`, nm).

Copper, own net only, in nm:
- F.Cu/B.Cu: each segment on the layer buffered to half its width (round caps), each pad on the layer
  (an SMD pad on F.Cu only; a drilled pad on both) as its rounded box, each via's pad disc; less every
  own-net drill (vias and drilled pads, every layer). Discs are polygons inscribed in the circle (copper
  only shrinks) and holes polygons circumscribed about it (holes only grow): the check is strict.
- In1/In2: KiCad's region for the net (check_cells.py's reading), plus the own-net via pad discs, less
  the drills.
A cell must be covered by its layer's copper (each square tested 1 nm inside its edges: the reader's
mm-to-nm conversion is floating point, and a cell edge on a pad edge would otherwise be flagged). A junction square, less its own drill's disc, must be
covered too: the square's rays and band are copper (flow::junction)."""
import sys, math
sys.path.insert(0, sys.argv[5] if len(sys.argv) > 5 else ".")
import numpy as np, shapely
from shapely.geometry import box, Point, LineString
from pcb import Board

dump, pcbpath, net = sys.argv[1], sys.argv[2], sys.argv[3]
gdir = sys.argv[4] if len(sys.argv) > 4 and sys.argv[4] != "-" else None
b = Board(pcbpath)
Q = 64
inscribed = lambda x, y, r: Point(x, y).buffer(r, quad_segs=Q)
circumscribed = lambda x, y, r: Point(x, y).buffer(r / math.cos(math.pi / (4 * Q)), quad_segs=Q)
nm = 1e6

def rounded(p):
    x0, y0 = (p["x"] - p["w"] / 2) * nm, (p["y"] - p["h"] / 2) * nm
    x1, y1 = (p["x"] + p["w"] / 2) * nm, (p["y"] + p["h"] / 2) * nm
    c = min(p["corner"], min(p["w"], p["h"]) / 2) * nm
    if c <= 0:
        return box(x0, y0, x1, y1)
    return box(x0 + c, y0 + c, x1 - c, y1 - c).buffer(c, quad_segs=Q) if (x1 - x0 > 2 * c or y1 - y0 > 2 * c) else inscribed((x0 + x1) / 2, (y0 + y1) / 2, c)

drills = [circumscribed(v["x"] * nm, v["y"] * nm, v["drill"] / 2 * nm) for v in b.vias if v["net"] == net]
drills += [circumscribed(p["x"] * nm, p["y"] * nm, p["drill"] / 2 * nm) for p in b.pads if p["net"] == net and p["drill"]]
holes = shapely.union_all(np.array(drills, dtype=object)) if drills else shapely.Polygon()
viapads = [inscribed(v["x"] * nm, v["y"] * nm, v["d"] / 2 * nm) for v in b.vias if v["net"] == net]

def outer(layer):
    parts = [LineString([(s["a"][0] * nm, s["a"][1] * nm), (s["b"][0] * nm, s["b"][1] * nm)]).buffer(s["w"] / 2 * nm, quad_segs=Q)
             for s in b.segs if s["net"] == net and s["layer"] == layer]
    for p in b.pads:
        if p["net"] != net:
            continue
        on = p["drill"] or (layer == "F.Cu" and any(l in ("F.Cu", "*.Cu") for l in p["layers"]))
        if on:
            parts.append(rounded(p))
    parts += viapads
    return shapely.union_all(np.array(parts, dtype=object)).difference(holes)

def plane(layer):
    import re
    fn = {"In1.Cu": "input-In1_Cu.g1", "In2.Cu": "input-In2_Cu.g2"}[layer]
    ring = []
    for line in open(f"{gdir}/{fn}"):
        m = re.match(r"(?:X(-?\d+))?(?:Y(-?\d+))?D0([12])\*", line.strip())
        if m and inreg:
            x = int(m.group(1)) if m.group(1) else ring[-1][0]
            y = -int(m.group(2)) if m.group(2) else ring[-1][1]
            ring.append((x, y))
        if line.startswith("G36"):
            inreg = True
        elif line.startswith("G37"):
            inreg = False
        if not line.startswith("G36") and "inreg" not in dir():
            inreg = False
    r = np.array(ring, dtype=float)
    if (r[0] != r[-1]).any():
        r = np.vstack([r, r[:1]])
    faces = list(shapely.get_parts(shapely.polygonize([shapely.unary_union(shapely.linestrings(r))])))
    pts = shapely.get_coordinates(shapely.point_on_surface(np.array(faces, dtype=object)))
    px, py = pts[:, 0][:, None], pts[:, 1][:, None]
    x1, y1, x2, y2 = r[:-1, 0][None], r[:-1, 1][None], r[1:, 0][None], r[1:, 1][None]
    up, dn = (y1 <= py) & (y2 > py), (y1 > py) & (y2 <= py)
    with np.errstate(divide="ignore", invalid="ignore"):
        xi = x1 + (py - y1) * (x2 - x1) / (y2 - y1)
    odd = ((up | dn) & (xi > px)).sum(1) % 2 == 1
    region = shapely.union_all(np.array([f for f, k in zip(faces, odd) if k], dtype=object))
    return shapely.union_all(np.array([region] + viapads, dtype=object)).difference(holes)

cells, squares = {}, {}
for line in open(dump):
    w = line.split()
    if w[0] == "cell":
        cells.setdefault(w[1], []).append(tuple(int(v) for v in w[2:]))
    else:
        squares.setdefault(w[1], []).append(tuple(int(v) for v in w[2:]))
# The exact pass, for a cell the polygons flag on F.Cu/B.Cu: the reader's geometry rounded to whole nm
# and tested in integers. A cell counts when each of its corners lies in one closed convex shape (a track's
# stadium, a via pad's disc, a pad's rounded box), or, failing that, each of its four sub-squares does
# (twice), and it is clear of every own-net drill. Polygons inscribe arcs, so a corner exactly on an arc
# is flagged there; this pass decides it.
N = lambda v: round(v * 1e6)
def shapes(layer):
    out = []
    for sg in b.segs:
        if sg["net"] == net and sg["layer"] == layer:
            out.append(("st", (N(sg["a"][0]), N(sg["a"][1])), (N(sg["b"][0]), N(sg["b"][1])), N(sg["w"]) / 2))
    for v in b.vias:
        if v["net"] == net:
            out.append(("disc", (N(v["x"]), N(v["y"])), N(v["d"]) / 2))
    for p in b.pads:
        if p["net"] == net and (p["drill"] or (layer == "F.Cu" and any(l in ("F.Cu", "*.Cu") for l in p["layers"]))):
            c = min(N(p["corner"]), min(N(p["w"]), N(p["h"])) // 2)
            x0, y0 = N(p["x"]) - N(p["w"]) / 2, N(p["y"]) - N(p["h"]) / 2
            x1, y1 = N(p["x"]) + N(p["w"]) / 2, N(p["y"]) + N(p["h"]) / 2
            out.append(("rr", x0 + c, y0 + c, x1 - c, y1 - c, c))
    return out
def holds(sh, q):
    x, y = q
    if sh[0] == "disc":
        (cx, cy), r = sh[1], sh[2]
        return (x - cx) ** 2 + (y - cy) ** 2 <= r * r
    if sh[0] == "rr":
        _, x0, y0, x1, y1, r = sh
        dx, dy = max(x0 - x, 0, x - x1), max(y0 - y, 0, y - y1)
        return dx * dx + dy * dy <= r * r
    _, a, bb, hw = sh
    dx, dy = bb[0] - a[0], bb[1] - a[1]
    px, py = x - a[0], y - a[1]
    l2, dot = dx * dx + dy * dy, px * dx + py * dy
    if dot <= 0 or l2 == 0:
        return px * px + py * py <= hw * hw
    if dot >= l2:
        return (x - bb[0]) ** 2 + (y - bb[1]) ** 2 <= hw * hw
    return (px * dy - py * dx) ** 2 <= hw * hw * l2
def square_in(shs, x0, y0, x1, y1, depth):
    if any(all(holds(sh, q) for q in ((x0, y0), (x1, y0), (x0, y1), (x1, y1))) for sh in shs):
        return True
    if depth == 0:
        return False
    mx, my = (x0 + x1) / 2, (y0 + y1) / 2
    return all(square_in(shs, *c, depth - 1) for c in ((x0, y0, mx, my), (mx, y0, x1, my), (x0, my, mx, y1), (mx, my, x1, y1)))
drill_discs = [((N(v["x"]), N(v["y"])), N(v["drill"]) / 2) for v in b.vias if v["net"] == net]
drill_discs += [((N(p["x"]), N(p["y"])), N(p["drill"]) / 2) for p in b.pads if p["net"] == net and p["drill"]]
def clear(x0, y0, x1, y1):
    for (cx, cy), r in drill_discs:
        dx, dy = max(x0 - cx, 0, cx - x1), max(y0 - cy, 0, cy - y1)
        if dx * dx + dy * dy < r * r:
            return False
    return True

bad_total = 0
for layer in sorted(set(cells) | set(squares)):
    if layer in ("F.Cu", "B.Cu"):
        cu = outer(layer)
    elif gdir:
        cu = plane(layer)
    else:
        print(f"{layer}: no Gerber given, not checked"); continue
    shapely.prepare(cu)
    cs = np.array(cells.get(layer, []), dtype=float).reshape(-1, 3)
    # each square is shrunk by 1 nm: the reader converts mm to nm in floating point
    boxes = shapely.box(cs[:, 0] + 1, cs[:, 1] + 1, cs[:, 0] + cs[:, 2] - 1, cs[:, 1] + cs[:, 2] - 1)
    ok = shapely.covers(cu, boxes) if len(boxes) else np.array([], dtype=bool)
    bad = [tuple(int(v) for v in cs[i]) for i in np.nonzero(~ok)[0]]
    if layer in ("F.Cu", "B.Cu") and bad:
        shs = shapes(layer)
        exact = [c for c in bad if not (square_in(shs, c[0], c[1], c[0] + c[2], c[1] + c[2], 2) and clear(c[0], c[1], c[0] + c[2], c[1] + c[2]))]
        print(f"   {layer}: {len(bad)} flagged by the polygons, {len(bad) - len(exact)} of them inside copper exactly (a corner on an arc)")
        bad = exact
    sq = squares.get(layer, [])
    sqbad = []
    for cx, cy, a, d in sq:
        g = box(cx - a + 1, cy - a + 1, cx + a - 1, cy + a - 1).difference(circumscribed(cx, cy, d / 2))
        if not cu.covers(g):
            sqbad.append((cx, cy, a, d, round(g.difference(cu).area / 1e6, 3)))
    bad_total += len(bad) + len(sqbad)
    print(f"{layer}: cells {len(cs)} not covered {len(bad)}; squares {len(sq)} not covered {len(sqbad)}")
    for c in bad[:10]:
        print(f"   cell {c}")
    for s in sqbad[:10]:
        print(f"   square at {s[:2]} half-side {s[2]} drill {s[3]}: uncovered area {s[4]} um2")
print(f"TOTAL not covered: {bad_total}")
