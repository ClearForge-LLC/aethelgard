"""AET-WO-1303 S4 probe: a small reader for the emitted .kicad_pcb (spike only;
the build reads through `route::read`). Coordinates in mm."""

import math
import re

TOKEN = re.compile(r'\(|\)|"(?:[^"\\]|\\.)*"|[^\s()]+')


def parse(text):
    stack, cur = [], []
    for t in TOKEN.findall(text):
        if t == "(":
            stack.append(cur)
            cur = []
        elif t == ")":
            done = cur
            cur = stack.pop()
            cur.append(done)
        else:
            cur.append(t[1:-1] if t.startswith('"') else t)
    return cur[0]


def kids(node, head):
    return [c for c in node if isinstance(c, list) and c and c[0] == head]


def kid(node, head):
    k = kids(node, head)
    return k[0] if k else None


def rot(x, y, deg):
    # KiCad: positive angle turns counter-clockwise on screen, y grows down.
    a = math.radians(deg)
    return x * math.cos(a) + y * math.sin(a), -x * math.sin(a) + y * math.cos(a)


class Board:
    def __init__(self, path):
        root = parse(open(path).read())
        self.pads, self.segs, self.vias, self.zones = [], [], [], []
        for fp in kids(root, "footprint"):
            at = kid(fp, "at")
            fx, fy = float(at[1]), float(at[2])
            fr = float(at[3]) if len(at) > 3 else 0.0
            ref = next(p[2] for p in kids(fp, "property") if p[1] == "Reference")
            for pad in kids(fp, "pad"):
                pat = kid(pad, "at")
                px, py = float(pat[1]), float(pat[2])
                pr = float(pat[3]) if len(pat) > 3 else 0.0
                dx, dy = rot(px, py, fr)
                sx, sy = float(kid(pad, "size")[1]), float(kid(pad, "size")[2])
                net = kid(pad, "net")
                # total rotation of the pad's box
                tot = (pr) % 180
                if abs(tot - 90) < 1e-6:
                    sx, sy = sy, sx
                elif abs(tot) > 1e-6:
                    raise ValueError(f"{ref}.{pad[1]}: pad at {tot} deg")
                self.pads.append(
                    dict(ref=ref, num=pad[1], x=fx + dx, y=fy + dy, w=sx, h=sy,
                         net=net[1] if net else None, layers=kid(pad, "layers")[1:])
                )
        for s in kids(root, "segment"):
            st, en = kid(s, "start"), kid(s, "end")
            self.segs.append(
                dict(a=(float(st[1]), float(st[2])), b=(float(en[1]), float(en[2])),
                     w=float(kid(s, "width")[1]), layer=kid(s, "layer")[1], net=kid(s, "net")[1])
            )
        for v in kids(root, "via"):
            at = kid(v, "at")
            self.vias.append(
                dict(x=float(at[1]), y=float(at[2]), d=float(kid(v, "size")[1]),
                     drill=float(kid(v, "drill")[1]), net=kid(v, "net")[1])
            )
        self.outline = None
        for r in kids(root, "gr_rect"):
            if kid(r, "layer")[1] == "Edge.Cuts":
                st, en = kid(r, "start"), kid(r, "end")
                self.outline = (float(st[1]), float(st[2]), float(en[1]), float(en[2]))
        for z in kids(root, "zone"):
            self.zones.append(dict(net=kid(z, "net")[1], layer=kid(z, "layer")[1]))
