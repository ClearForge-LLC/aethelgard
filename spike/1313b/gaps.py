#!/usr/bin/env python3
"""gaps.py <dump.json>...: the placer's own gaps on a placed board (AET-WO-1313b), from the spike's
dump: every keep-out box inside the outline, and every pair of keep-out boxes at least GAP_NM
(0.5 mm) apart, or IC_GAP_NM (1 mm) when both are ICs (not Device:, not a test point or switch).
Headroom blocks count as ICs. Prints the closest pair of each class and any violation."""
import json
import sys

GAP, IC_GAP = 500_000, 1_000_000
def ic(lib):
    return not lib.startswith("Device:") and lib not in ("Connector:TestPoint", "Switch:SW_Push")
def sep(a, b):
    dx = max(b[0] - a[2], a[0] - b[2], 0)
    dy = max(b[1] - a[3], a[1] - b[3], 0)
    overlap_x = a[0] < b[2] and b[0] < a[2]
    overlap_y = a[1] < b[3] and b[1] < a[3]
    if overlap_x and overlap_y:
        return -1
    return max(dx, dy) if (overlap_x or overlap_y) else max(dx, dy)
bad = 0
for path in sys.argv[1:]:
    d = json.load(open(path)); o = d["outline"]; ps = d["parts"]
    out = [p["ref"] for p in ps if p["keep"][0] < o[0] or p["keep"][1] < o[1] or p["keep"][2] > o[2] or p["keep"][3] > o[3]]
    worst = {True: (10**12, ""), False: (10**12, "")}
    viol = []
    for i in range(len(ps)):
        for j in range(i + 1, len(ps)):
            a, b = ps[i], ps[j]
            both = ic(a["lib"]) and ic(b["lib"])
            s = sep(a["keep"], b["keep"])
            need = IC_GAP if both else GAP
            if s < worst[both][0]:
                worst[both] = (s, f"{a['ref']}/{b['ref']}")
            if s < need:
                viol.append(f"{a['ref']}/{b['ref']} {s/1e6:.3f} mm < {need/1e6:.1f}")
    print(f"{path}: outside {out or 'none'}; closest IC pair {worst[True][0]/1e6:.3f} mm ({worst[True][1]}); closest other pair {worst[False][0]/1e6:.3f} mm ({worst[False][1]}); violations {len(viol)} {viol[:5]}")
    bad += bool(out or viol)
sys.exit(1 if bad else 0)
