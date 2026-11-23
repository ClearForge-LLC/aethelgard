#!/usr/bin/env python3
"""AET-WO-1322 (3): what route::read's cap would count on each board.
Prints, per .kicad_pcb under the given roots, the copper items MAX_ITEMS
counts today (segments, vias, every footprint pad), the front-courtyard
edges route::read builds (fp_line 1, fp_rect 4, fp_circle 4, fp_poly one per
point, on F.CrtYd) and the footprint keepout zones; then the largest.
Usage: count.py <root>..."""
import os, re, sys

TOK = re.compile(r'"(?:[^"\\]|\\.)*"|[()]|[^\s()"]+')

def parse(text):
    stack, root = [], []
    cur = root
    for t in TOK.findall(text):
        if t == "(":
            stack.append(cur); n = []; cur.append(n); cur = n
        elif t == ")":
            cur = stack.pop()
        else:
            cur.append(t)
    return root[0]

def lists(n, head=None):
    return [c for c in n[1:] if isinstance(c, list) and (head is None or (c and c[0] == head))]

def layer(n):
    for c in lists(n, "layer"):
        return c[1].strip('"') if len(c) > 1 else None

def count(path):
    b = parse(open(path, encoding="utf-8").read())
    segs = len(lists(b, "segment")); vias = len(lists(b, "via"))
    pads = edges = keep = 0
    for fp in lists(b, "footprint"):
        pads += len(lists(fp, "pad"))
        keep += len(lists(fp, "zone"))
        for g in lists(fp):
            if not (g and isinstance(g[0], str) and g[0].startswith("fp_")) or layer(g) != "F.CrtYd":
                continue
            if g[0] == "fp_line": edges += 1
            elif g[0] in ("fp_rect", "fp_circle"): edges += 4
            elif g[0] == "fp_poly":
                for p in lists(g, "pts"): edges += len(lists(p, "xy"))
    return segs + vias + pads, edges, keep

rows = []
for root in sys.argv[1:]:
    for d, _, fs in os.walk(root):
        for f in fs:
            if f.endswith(".kicad_pcb"):
                p = os.path.join(d, f)
                try:
                    rows.append((*count(p), p))
                except Exception as e:
                    print(f"unread {p}: {e}", file=sys.stderr)
rows.sort(key=lambda r: r[0] + r[1] + r[2])
for r in rows[-5:]:
    print(f"items {r[0]} edges {r[1]} keepouts {r[2]} total {r[0]+r[1]+r[2]} {r[3]}")
print(f"{len(rows)} boards; largest total {rows[-1][0]+rows[-1][1]+rows[-1][2]}; largest items {max(r[0] for r in rows)}; largest edges {max(r[1] for r in rows)}")
