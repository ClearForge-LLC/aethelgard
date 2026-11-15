"""S4 measurement, second version (rbound2): per MCU supply pin on VDD_3V3,
the bound from its best and worst stand-in source, every bound recomputed
from its own path. Usage: run_paths2.py <conditions.json>"""
import json, sys
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
    table, why = {}, Counter()
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
    good = [v[0] for v in table.values() if v]
    best_per_pin = [min(v[0] for k, v in table.items() if k[2] == m["num"] and v) for m in mcu if any(v for k, v in table.items() if k[2] == m["num"])]
    print(f"   best-stand-in range over pins: {min(best_per_pin)*1e3:.1f}..{max(best_per_pin)*1e3:.1f} mOhm; all pairs max {max(good)*1e3:.1f}; pairs bounded {len(good)} of {len(table)}; refusals {dict(why)}")
