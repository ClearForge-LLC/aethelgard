#!/usr/bin/env python3
"""AET-WO-1313 §1.2: where the area goes, from a placed layout dumped by the spike's placer
(AET_1313_DUMP, spike/1313/placer.patch). Usage: python3 -I spike/1313/area.py <layout.json>...

For each layout: the outline; the sum of every keep-out box's area and the packing efficiency
(their ratio); the connector row's share (the band from the top of the highest bottom-edge
connector's keep-out box to the bottom edge, at full width; a connector at another rotation is
counted in its own edge's band); and the three largest empty rectangles inside the outline,
found greedily on a 0.25 mm grid (each found one is filled before the next), with the keep-out
boxes as the obstacles.
"""
import json
import sys

NM = 1_000_000
CELL = 250_000


def mm(v):
    return v / NM


def largest_empty(free):
    """The largest all-True rectangle in a 2D boolean grid: (area, row0, col0, rows, cols)."""
    h = len(free[0])
    heights = [0] * h
    best = (0, 0, 0, 0, 0)
    for r, row in enumerate(free):
        for c in range(h):
            heights[c] = heights[c] + 1 if row[c] else 0
        stack = []
        for c in range(h + 1):
            cur = heights[c] if c < h else 0
            start = c
            while stack and stack[-1][1] >= cur:
                s, hh = stack.pop()
                area = hh * (c - s)
                if area > best[0]:
                    best = (area, r - hh + 1, s, hh, c - s)
                start = s
            stack.append((start, cur))
    return best


def report(path):
    d = json.load(open(path))
    x0, y0, x1, y1 = d["outline"]
    w, hgt = x1 - x0, y1 - y0
    area = w * hgt
    keep = sum((p["keep"][2] - p["keep"][0]) * (p["keep"][3] - p["keep"][1]) for p in d["parts"])
    by_kind = {}
    for p in d["parts"]:
        k = p["keep"]
        by_kind[p["kind"]] = by_kind.get(p["kind"], 0) + (k[2] - k[0]) * (k[3] - k[1])
    conns = [p for p in d["parts"] if p["kind"] == "connector"]
    bottom = [p for p in conns if p["rot"] == 0]
    band = (y1 - min(p["keep"][1] for p in bottom)) * w if bottom else 0
    others = [p for p in conns if p["rot"] != 0]
    other_band = 0
    for rot in sorted({p["rot"] for p in others}):
        ps = [p for p in others if p["rot"] == rot]
        if rot == 180:  # top edge
            other_band += (max(p["keep"][3] for p in ps) - y0) * w
        elif rot in (90, 270):  # a side edge: the band nearer the edge they sit on
            left = sum(1 for p in ps if p["keep"][0] - x0 < x1 - p["keep"][2])
            if left * 2 >= len(ps):
                other_band += (max(p["keep"][2] for p in ps) - x0) * hgt
            else:
                other_band += (x1 - min(p["keep"][0] for p in ps)) * hgt
    cols = w // CELL
    rows = hgt // CELL
    free = [[True] * cols for _ in range(rows)]
    for p in d["parts"]:
        k = p["keep"]
        c0 = max(0, (k[0] - x0) // CELL)
        c1 = min(cols, -(-(k[2] - x0) // CELL))
        r0 = max(0, (k[1] - y0) // CELL)
        r1 = min(rows, -(-(k[3] - y0) // CELL))
        for r in range(r0, r1):
            row = free[r]
            for c in range(c0, c1):
                row[c] = False
    empties = []
    for _ in range(3):
        a, r0, c0, nr, nc = largest_empty(free)
        if a == 0:
            break
        empties.append((nc * CELL, nr * CELL, x0 + c0 * CELL, y0 + r0 * CELL))
        for r in range(r0, r0 + nr):
            for c in range(c0, c0 + nc):
                free[r][c] = False
    print(f"## {path}")
    print(f"outline {mm(w):.2f} x {mm(hgt):.2f} mm = {area / NM**2:.0f} mm2; parts {len(d['parts'])}")
    print(f"keep-out sum {keep / NM**2:.0f} mm2; packing efficiency {100 * keep / area:.1f} %")
    print("  by kind: " + ", ".join(f"{k} {v / NM**2:.0f} mm2 ({100 * v / area:.1f} %)" for k, v in sorted(by_kind.items())))
    print(f"connector row (bottom) band {band / NM**2:.0f} mm2 = {100 * band / area:.1f} % of the outline"
          + (f"; other edges {other_band / NM**2:.0f} mm2 = {100 * other_band / area:.1f} %" if others else ""))
    for i, (ew, eh, ex, ey) in enumerate(empties):
        print(f"empty rectangle {i + 1}: {mm(ew):.2f} x {mm(eh):.2f} mm = {ew * eh / NM**2:.0f} mm2"
              f" at ({mm(ex - x0):.2f}, {mm(ey - y0):.2f}) mm from the outline's top-left")


for p in sys.argv[1:]:
    report(p)
