"""Usage: check_cells.py <board> <fill dir>
Each of <fill>/<board>-<net>-<h>.cells cell (layer x0 y0 side, nm, KiCad board coords) must lie
wholly in the copper of KiCad's refilled plane (<fill>/<board>/gerbers/input-In{1,2}_Cu.g{1,2}).
Copper = even-odd union of every G36 region ring (holes are rings), plus any D01 stroke outside a region
(none exist; asserted). Gerber y = -board y; units nm = Gerber 4.6 mm integers."""
import re, sys, json
import numpy as np, shapely
from shapely.geometry import Polygon, box
board, F = sys.argv[1], sys.argv[2]
G = f"{F}/{board}/gerbers"
PLANE = {"In1.Cu": ("input-In1_Cu.g1", "GND"), "In2.Cu": ("input-In2_Cu.g2", "VDD_3V3")}

def parse(path, want_net):
    rings, cur, inreg, net, strokes, flashes = [], None, False, None, 0, []
    x = y = 0
    for line in open(path):
        line = line.strip()
        m = re.match(r"%TO\.N,([^*]*)\*%", line)
        if m: net = m.group(1)
        if line.startswith("%TD"): net = None
        if line.startswith("G36"):
            inreg = True; cur = None; regnet = net; nreg = 0; continue
        if line.startswith("G37"):
            if cur and len(cur) > 2: rings.append((regnet, cur))
            inreg = False; continue
        m = re.match(r"(?:X(-?\d+))?(?:Y(-?\d+))?D0([123])\*", line)
        if not m: continue
        if m.group(1): x = int(m.group(1))
        if m.group(2): y = int(m.group(2))
        d = m.group(3)
        if inreg:
            if d == "2":
                if cur and len(cur) > 2: rings.append((regnet, cur))
                cur = [(x, -y)]
            else: cur.append((x, -y))
        elif d == "1": strokes += 1
        elif d == "3": flashes.append((x, -y, net))
    return rings, strokes, flashes

out = {}
for layer, (fn, pnet) in PLANE.items():
    rings, strokes, flashes = parse(f"{G}/{fn}", pnet)
    nets = {n for n, _ in rings}
    assert nets == {pnet}, nets
    assert strokes == 0, strokes
    # The region is ONE closed ring that reaches its holes by zero-width cut-ins (Gerber's way). Node the
    # linework, polygonize into faces, keep faces whose interior point has odd crossing parity AND nonzero winding
    # (they are compared; any disagreement is printed).
    assert len(rings) == 1
    r = np.array(rings[0][1], dtype=float)
    if (r[0] != r[-1]).any(): r = np.vstack([r, r[:1]])
    faces = list(shapely.get_parts(shapely.polygonize([shapely.unary_union(shapely.linestrings(r))])))
    pts = shapely.get_coordinates(shapely.point_on_surface(np.array(faces, dtype=object)))
    px, py = pts[:, 0][:, None], pts[:, 1][:, None]
    x1, y1, x2, y2 = r[:-1, 0][None], r[:-1, 1][None], r[1:, 0][None], r[1:, 1][None]
    up, dn = (y1 <= py) & (y2 > py), (y1 > py) & (y2 <= py)
    with np.errstate(divide="ignore", invalid="ignore"):
        xi = x1 + (py - y1) * (x2 - x1) / (y2 - y1)
    right = xi > px
    parity = ((up | dn) & right).sum(1) % 2 == 1
    wind = ((up & right).sum(1) - (dn & right).sum(1)) != 0
    print(f"  faces {len(faces)} odd {parity.sum()} nonzero {wind.sum()} disagree {(parity != wind).sum()}", flush=True)
    keep = [f for f, k in zip(faces, parity) if k]
    cu = shapely.union_all(np.array(keep, dtype=object))
    print(f"{board} {layer} net {pnet}: ring vertices {len(r)} copper area {cu.area/1e12:.3f} mm2 valid {cu.is_valid} bounds {[round(v/1e6,4) for v in cu.bounds]}", flush=True)
    shapely.prepare(cu)
    out[layer] = (cu, flashes)

def verify_transform():
    txt = open(f"{F}/{board}/prepared/input.kicad_pcb").read()
    vias = [(round(float(a)*1e6), round(float(b)*1e6)) for a, b in re.findall(r"\(via\s+\(at ([-\d.]+) ([-\d.]+)\)", txt)]
    fl = {(x, y) for _, (cu, fls) in out.items() for x, y, _ in fls}
    hit = sum(v in fl for v in vias)
    r = re.search(r"\(gr_rect\s+\(start ([-\d.]+) ([-\d.]+)\)\s+\(end ([-\d.]+) ([-\d.]+)\)[^)]*\)[^)]*\)[^)]*\)\s*\(fill no\)\s*\(layer \"Edge.Cuts\"", txt)
    print(f"transform check: {hit}/{len(vias)} board vias coincide exactly with a Gerber flash at (x, -y); Edge.Cuts rect {r.groups() if r else None}", flush=True)
verify_transform()

res = {}
for net in ("GND", "VDD_3V3"):
    for h in (62500, 31250):
        p = f"{F}/{board}-{net}-{h}.cells"
        lines = [l.split() for l in open(p)]
        lay = np.array([l[0] for l in lines]); a = np.array([[int(v) for v in l[1:]] for l in lines], dtype=np.int64)
        assert set(lay) <= set(PLANE), set(lay)
        bad = []
        for layer in PLANE:
            sel = np.where(lay == layer)[0]
            if not len(sel): continue
            cu = out[layer][0]
            ok = np.zeros(len(sel), bool)
            for i in range(0, len(sel), 200000):
                s = a[sel[i:i+200000]]
                bx = shapely.box(s[:,0], s[:,1], s[:,0]+s[:,2], s[:,1]+s[:,2])
                ok[i:i+200000] = shapely.contains(cu, bx)
            for j in sel[~ok]: bad.append((layer, *map(int, a[j])))
        print(f"RESULT {board} {net} h={h}: cells {len(lines)} layers {sorted(set(lay))} NOT inside {len(bad)}", flush=True)
        det = []
        for layer, x0, y0, s in bad:
            cu, fls = out[layer]
            b = box(x0, y0, x0+s, y0+s); d = b.difference(cu)
            pts = list(b.exterior.coords) + [c for g in getattr(d, "geoms", [d]) for c in getattr(g, "exterior", g).coords]
            prot = max(shapely.distance(cu, shapely.points(pts)))
            near = min(fls, key=lambda f: (f[0]-(x0+s/2))**2 + (f[1]-(y0+s/2))**2)
            dist = ((near[0]-(x0+s/2))**2 + (near[1]-(y0+s/2))**2) ** .5
            det.append(dict(layer=layer, x0=x0, y0=y0, side=s, uncovered_area_frac=d.area/(s*s), protrusion_nm=float(prot),
                            nearest_flash=near, nearest_flash_dist_nm=dist))
        det.sort(key=lambda r: -r["protrusion_nm"])
        res[f"{net}-{h}"] = dict(cells=len(lines), bad=len(bad), worst=det[:10], all=det)
json.dump(res, open(f"{F}/harness/result-{board}.json", "w"), indent=1)
for k, v in res.items():
    for r in v["worst"][:5]: print(k, r)
