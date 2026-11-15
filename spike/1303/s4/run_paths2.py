"""S4 measurement, second version (rbound2): per MCU supply pin on VDD_3V3,
the bound from its best and worst stand-in source, every bound recomputed
from its own path. Usage: run_paths2.py <conditions.json>"""
import json, sys
import numpy as np
from collections import Counter
from pcb import Board
from rbound import rho_max
from rbound2 import build, shortest, path_to, path_bound
from rbound import tube_width_ok

c = json.load(open(sys.argv[1]))
NET, W = "VDD_3V3", 0.2
for name, planes in [("ref/sentinel-01", False), ("four/sentinel-01-4l", True)]:
    b = Board(f"../boards/{name}.kicad_pcb")
    ck = dict(c, **c[name])
    H = (ck["h_min_mm"], ck["h_max_mm"])
    pdep = (ck.get("plane_depth_min_mm", 1.0), ck.get("plane_depth_max_mm", 1.0))
    pads = [p for p in b.pads if p["net"] == NET]
    mcu = [p for p in pads if p["ref"] == "U1"]
    srcs = [p for p in pads if p["ref"] != "U1"]
    g, vias = build(b, NET, ck, W, planes)
    print(f"== {name}: rho_max {rho_max(ck):.4e} ohm*mm; {len(mcu)} MCU pins, {len(srcs)} stand-ins")
    table, why, coefs = {}, Counter(), {}
    for s in srcs:
        cache = {}
        for m in mcu:
            bans = frozenset()
            R, reason = None, "no path under the flow rules"
            for attempt in range(40):  # ban what failed, search again
                if bans not in cache:
                    cache[bans] = shortest(g, ck, ("pad", s["ref"], s["num"]), bans)
                dist, prev = cache[bans]
                R0, nodes, tags = path_to(prev, dist, ("pad", m["ref"], m["num"]))
                if R0 is None:
                    R, reason = None, "no path under the flow rules"
                    break
                pts = [n if n[0] == "pt" else ("pt", n[2], (vias[n[1]]["x"], vias[n[1]]["y"])) if n[0] == "vc" else n for n in nodes]
                w, _ = tube_width_ok(pts, W)  # narrowed for the bends
                ban = None
                for _ in range(4):  # narrow again for a pad end, if needed
                    R, reason, lim = path_bound(nodes, tags, vias, ck, w, H, pdep)
                    if R is not None or not isinstance(lim, float) or lim <= 0.02:
                        ban = lim if isinstance(lim, tuple) else None
                        break
                    w = lim
                if R is not None or ban is None:
                    break
                bans = bans | {ban[1]}
            if R is not None and w < W - 1e-12:
                why["narrowed (bounded)"] += 1
            if R is not None:
                # the same path at other thicknesses: R = rho (a/t_out + b/t_pl + c/A(t_p)), exactly
                rho = rho_max(ck)
                A = lambda tp: tp * (2 * 0.15 - tp) / 2
                sets = [(ck["t_outer_min_mm"], ck["t_plane_min_mm"], ck["plating_min_mm"]),
                        (2 * ck["t_outer_min_mm"], ck["t_plane_min_mm"], ck["plating_min_mm"]),
                        (ck["t_outer_min_mm"], 2 * ck["t_plane_min_mm"], ck["plating_min_mm"]),
                        (ck["t_outer_min_mm"], ck["t_plane_min_mm"], 0.025)]
                vals, rows = [], []
                for to, tl, tp in sets:
                    cc = dict(ck, t_outer_min_mm=to, t_plane_min_mm=tl, plating_min_mm=tp)
                    vals.append(path_bound(nodes, tags, vias, cc, w, H, pdep)[0] / rho)
                    rows.append([1 / to, 1 / tl, 1 / A(tp)])
                coef, res, *_ = np.linalg.lstsq(np.array(rows), np.array(vals), rcond=None)
                fit = max(abs(np.array(rows) @ coef - np.array(vals)) / np.array(vals))
                coefs[(s["ref"], s["num"], m["num"])] = (coef, fit)
            if R is None:
                why[reason.split(":")[0]] += 1
            kinds = Counter(t[0] for t in tags)
            table[(s["ref"], s["num"], m["num"])] = (R, dict(kinds)) if R is not None else None
    for m in mcu:
        rows = [(v[0], k, v[1]) for k, v in table.items() if k[2] == m["num"] and v]
        nref = sum(1 for k, v in table.items() if k[2] == m["num"] and not v)
        if not rows:
            print(f"U1.{m['num']:>4}: REFUSED from every stand-in")
            continue
        lo, hi = min(rows), max(rows)
        print(f"U1.{m['num']:>4}: best {lo[0]*1e3:6.1f} mOhm from {lo[1][0]}.{lo[1][1]} {lo[2]} | worst {hi[0]*1e3:6.1f} from {hi[1][0]}.{hi[1][1]}"
              + (f" | refused from {nref}" if nref else ""))
    fits = [f for _, f in coefs.values()]
    print(f"   coefficients: R = rho_max (a/t_out + b/t_plane + c/A(t_p)), fitted exactly on 4 thickness sets per path; worst relative residual {max(fits):.1e}")
    for m in mcu:
        ks = [k for k in coefs if k[2] == m["num"] and table.get(k)]
        if not ks:
            continue
        k = min(ks, key=lambda k_: table[k_][0])
        a, b, cc = coefs[k][0]
        print(f"   U1.{m['num']:>4} best ({k[0]}.{k[1]}): a {a:9.2f} mm, b {b:9.2f} mm, c {cc:.4e} mm^3   [a, b in mm of squares x thickness; c x 1/A]")
    good = [v[0] for v in table.values() if v]
    best_per_pin = [min(v[0] for k, v in table.items() if k[2] == m["num"] and v) for m in mcu if any(v for k, v in table.items() if k[2] == m["num"])]
    print(f"   best-stand-in range over pins: {min(best_per_pin)*1e3:.1f}..{max(best_per_pin)*1e3:.1f} mOhm; all pairs max {max(good)*1e3:.1f}; pairs bounded {len(good)} of {len(table)}; refusals {dict(why)}")
