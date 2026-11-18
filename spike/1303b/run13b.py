"""AET-WO-1303b §1.1: per MCU supply pin, the closed-form (D5 (a)) bound from
the regulator's current-entry pad (VDD) and to the regulator's ground pad
(GND), on the boards whose specification has a regulator. The pieces are
1303's (spike/1303/s4 at d5bc8416), with 1303b's reader and plane-obstacle
changes (pcb.py, rbound2.py headers). Every reported bound is recomputed from
its own path, and its coefficients a, b, c fitted exactly on four thickness
sets: R = rho_max (a/t_out + b/t_plane + c/A(t_p)).
Usage: run13b.py <conditions.json> <hole_offset_mm> <boards dir>"""
import json, sys, math
import numpy as np
from collections import Counter
from pcb import Board
from rbound import rho_max, tube_width_ok
from rbound2 import build, shortest, path_to, path_bound

c0 = json.load(open(sys.argv[1]))
off = float(sys.argv[2])
BD = sys.argv[3]
SUFFIX = sys.argv[4] if len(sys.argv) > 4 else ""  # "-gnd": GND in the supply class
GV = (0.7, 0.3) if SUFFIX == "-gnd" else (0.6, 0.3)
RUNS = [
    # board, net, source (ref, pin filter), MCU, via pad/drill, planes, H, plane depth, W
    ("sentinel-02-power", "VDD_3V3", "L2", (0.7, 0.3), False, (0.34, 2.0), None, 0.2),
    ("sentinel-02-power", "GND", "U11", GV, False, (0.34, 2.0), None, 0.2),
    # the power fixture at four layers, constructed: sentinel-02-power with
    # Sentinel-03's layers and stackup (no 4-layer power fixture exists)
    ("s024l", "VDD_3V3", "L2", (0.7, 0.3), True, (1.25, 1.851), (1.07, 1.851), 0.2),
    ("s024l", "GND", "U11", GV, True, (1.25, 1.851), (0.179, 1.851), 0.2),
    ("sentinel-03", "VDD_3V3", "L2", (0.7, 0.3), True, (1.25, 1.851), (1.07, 1.851), 0.2),
    ("sentinel-03", "GND", "U11", GV, True, (1.25, 1.851), (0.179, 1.851), 0.2),
]
A = lambda tp, rd: tp * (2 * rd - tp) / 2
HOPS = []
for board, NET, SRC, (vp, vd), planes, H, pdep, W in RUNS:
    b = Board(f"{BD}/{board}{SUFFIX}/{board}.kicad_pcb")
    ck = dict(c0, via_pad_mm=vp, drill_mm=vd, hole_offset_mm=off)
    if pdep:
        ck["plane_depth_min_mm"], ck["plane_depth_max_mm"] = pdep
    pdep = pdep or (1.0, 1.0)
    pads = [p for p in b.pads if p["net"] == NET and p.get("entry", True)]
    mcu = [p for p in pads if p["ref"] == "U1"]
    srcs = [p for p in pads if p["ref"] == SRC]
    allmcu = [p for p in b.pads if p["net"] == NET and p["ref"] == "U1"]
    g, vias = build(b, NET, ck, W, planes)
    rho = rho_max(ck)
    print(f"== {board} {NET}: source {SRC} pads {[s['num'] for s in srcs]}; {len(allmcu)} MCU pins "
          f"({len(allmcu) - len(mcu)} not enterable); {len(vias)} vias {vp}/{vd} used of "
          f"{sum(1 for v in b.vias if v['net'] == NET)}; planes {planes}; offset {off} mm; rho_max {rho:.4e} ohm*mm")
    per_pin, why = {}, Counter()
    for m in mcu:
        best = None
        tried = Counter()
        for s in srcs:
            bans = frozenset()
            R, reason = None, "no path under the flow rules"
            for attempt in range(40):
                dist, prev = shortest(g, ck, ("pad", s["ref"], s["num"]), bans)
                R0, nodes, tags = path_to(prev, dist, ("pad", m["ref"], m["num"]))
                if R0 is None:
                    R, reason = None, "no path under the flow rules"
                    break
                vias_ = vias
                pts = [n if n[0] == "pt" else ("pt", n[2], (vias_[n[1]]["x"], vias_[n[1]]["y"])) if n[0] == "vc" else n for n in nodes]
                w, _ = tube_width_ok(pts, W)
                ban = None
                for _ in range(4):
                    R, reason, lim = path_bound(nodes, tags, vias, ck, w, H, pdep)
                    if R is not None or not isinstance(lim, float) or lim <= 0.02:
                        ban = lim if isinstance(lim, tuple) else None
                        break
                    w = lim
                if R is None:
                    tried[reason.split(":")[0]] += 1
                if R is not None or ban is None:
                    break
                bans = bans | {ban[1]}
            if R is None:
                why[(s["num"], reason.split(":")[0])] += 1
                continue
            sets = [(ck["t_outer_min_mm"], ck["t_plane_min_mm"], ck["plating_min_mm"]),
                    (2 * ck["t_outer_min_mm"], ck["t_plane_min_mm"], ck["plating_min_mm"]),
                    (ck["t_outer_min_mm"], 2 * ck["t_plane_min_mm"], ck["plating_min_mm"]),
                    (ck["t_outer_min_mm"], ck["t_plane_min_mm"], 0.025)]
            vals, rows = [], []
            for to, tl, tp in sets:
                cc = dict(ck, t_outer_min_mm=to, t_plane_min_mm=tl, plating_min_mm=tp)
                vals.append(path_bound(nodes, tags, vias, cc, w, H, pdep)[0] / rho)
                rows.append([1 / to, 1 / tl, 1 / A(tp, vd / 2)])
            coef, *_ = np.linalg.lstsq(np.array(rows), np.array(vals), rcond=None)
            fit = max(abs(np.array(rows) @ coef - np.array(vals)) / np.array(vals))
            kinds = dict(Counter(t[0] for t in tags))
            if best is None or R < best[0]:
                best = (R, s["num"], coef, fit, kinds, w, tags)
        per_pin[m["num"]] = best
        if best is None:
            per_pin[("why", m["num"])] = f"{reason}; pieces refused on the way: {dict(tried)}"
    # pins refused with nothing banned: which vias their stubs reach, and
    # whether those vias have any plane edge
    for m in mcu:
        if per_pin[m["num"]] is None and "{}" in per_pin[("why", m["num"])]:
            start = ("pad", m["ref"], m["num"])
            seen, q = {start}, [start]
            while q:
                n = q.pop()
                for nb, _, tg in g[n]:
                    if tg[0] in ("seg", "padend") and nb not in seen:
                        seen.add(nb); q.append(nb)
            vcs = sorted({n[1] for n in seen if n[0] == "vc"})
            pe = {i: sum(1 for nb, _, tg in g[("plane", i)] if tg[0] == "plane") for i in vcs}
            per_pin[("why", m["num"])] += f"; its stub reaches vias {vcs} with plane edges {pe}"
    for p in allmcu:
        if p not in mcu:
            print(f"  U1.{p['num']:>4}: REFUSED: pad not enterable ({p['shape']}, drill {p['drill']})")
    for m in mcu:
        bst = per_pin[m["num"]]
        if bst is None:
            print(f"  U1.{m['num']:>4}: REFUSED: {per_pin[('why', m['num'])]}")
            continue
        R, sn, (a, bb, cc), fit, kinds, w, tags = bst
        print(f"  U1.{m['num']:>4}: {R*1e3:7.1f} mOhm from {SRC}.{sn} (w {w:.3f}) a {a:8.2f} b {bb:8.2f} c {cc:.3e} fit {fit:.0e} {kinds}")
        for t in tags:
            if t[0] == "plane":
                _, i, k, beta, cost = t
                D = math.dist((vias[i]["x"], vias[i]["y"]), (vias[k]["x"], vias[k]["y"]))
                exact = rho / (math.pi * ck["t_plane_min_mm"]) * math.acosh(D / (2 * vd / 2))
                HOPS.append(cost / exact)
                print(f"        hop D {D:6.2f} mm beta {math.degrees(beta):4.1f} deg: bound {cost*1e3:6.1f} mOhm, exact two-hole sheet {exact*1e3:5.2f} mOhm, ratio {cost/exact:5.1f}")
    got = [v[0] for k, v in per_pin.items() if v and not isinstance(k, tuple)]
    print(f"  coverage {len(got)} of {len(allmcu)} pins" + (f"; range {min(got)*1e3:.1f}..{max(got)*1e3:.1f} mOhm" if got else "") + f"; refusals {dict(why)}")

if HOPS:
    print(f"== plane hops used: {len(HOPS)}; bound / exact two-hole sheet: min {min(HOPS):.1f}, median {sorted(HOPS)[len(HOPS)//2]:.1f}, max {max(HOPS):.1f}")
