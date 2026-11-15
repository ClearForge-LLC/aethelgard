"""S4 measurement: the bound from each stand-in source pad to each MCU supply
pin on VDD_3V3, on the reference and the 4-layer fixture.
Usage: run_paths.py <conditions.json>"""
import json, math, sys
from pcb import Board
from rbound import build, add_plane2 as add_plane, shortest, path_to, tube_width_ok, rho_max

c = json.load(open(sys.argv[1]))
NET = "VDD_3V3"
for name, planes in [("ref/sentinel-01", False), ("four/sentinel-01-4l", True)]:
    b = Board(f"../boards/{name}.kicad_pcb")
    W = 0.2
    pads = [p for p in b.pads if p["net"] == NET]
    mcu = [p for p in pads if p["ref"] == "U1"]
    srcs = [p for p in pads if p["ref"] != "U1"]
    ck = dict(c, **c[name])
    print(f"== {name}: rho_max {rho_max(ck):.4e} ohm*mm; {len(mcu)} MCU pins, {len(srcs)} stand-in sources")
    table = {}
    narrowed = 0
    for s in srcs:
        w = W
        for _ in range(6):
            g, vias, segs, _ = build(b, NET, ck, w)
            blocked = add_plane(g, b, NET, ck, w, vias, b.outline) if planes else 0
            dist, prev = shortest(g, ("pad", s["ref"], s["num"]))
            worst_w = w
            res = {}
            for m in mcu:
                R, tags, nodes = path_to(prev, dist, ("pad", m["ref"], m["num"]))
                if R is None:
                    res[m["num"]] = (None, [])
                    continue
                ww, _ = tube_width_ok(nodes, W)
                worst_w = min(worst_w, ww)
                res[m["num"]] = (R, tags)
            if worst_w >= w - 1e-12:
                break
            w = worst_w
            narrowed += 1
        for m in mcu:
            R, tags = res[m["num"]]
            if R is None:
                table[(s["ref"], s["num"], m["num"])] = (None, w, "refused")
                continue
            kinds = {}
            for t in tags:
                kinds[t[0]] = kinds.get(t[0], 0) + 1
            table[(s["ref"], s["num"], m["num"])] = (R, w, kinds)
    for m in mcu:
        rows = [(v[0], k, v[1], v[2]) for k, v in table.items() if k[2] == m["num"] and v[0] is not None]
        refused = [k[0] + "." + k[1] for k, v in table.items() if k[2] == m["num"] and v[0] is None]
        if not rows:
            print(f"U1.{m['num']:>4}: REFUSED from every stand-in (no admissible plane flow)")
            continue
        lo, hi = min(rows), max(rows)
        print(f"U1.{m['num']:>4}: best {lo[0]*1e3:7.1f} mOhm from {lo[1][0]}.{lo[1][1]} (w {lo[2]:.3f}, {lo[3]}) | worst {hi[0]*1e3:7.1f} mOhm from {hi[1][0]}.{hi[1][1]}" + (f" | refused from {len(refused)}" if refused else ""))
    allR = [v[0] for v in table.values() if v[0] is not None]
    print(f"   over all pairs: min {min(allR)*1e3:.1f} max {max(allR)*1e3:.1f} mOhm; sources needing a narrowed tube: {narrowed}")
    if planes:
        print(f"   plane rhombi blocked (last build): {blocked}")
