"""AET-WO-1303 S4, the graph and the per-path bound (second version, after
the advisor's review: own-net via holes, same-layer passes through a via,
hops through an intermediate pad, the pad-end and via-entry conditions).

Every bound reported is `path_bound`'s recomputation of one found path from
its own geometry; Dijkstra's costs only choose the path. Each piece's flow is
in `rbound.py`'s module doc; what is new here:
- A via's centre on a layer is its own node ('vc'). A path reaching it by a
  track must leave by the fan into the hole wall, and the reverse, so no path
  crosses a hole on one layer.
- A path never passes through a pad other than its two terminals: an
  intermediate pad is not shorted by anything, so it is not an equipotential.
- A via is used once: in by its fan, out by its barrel (or the reverse).
- Plane to plane through via B: the arriving sector's flow and the leaving
  one's are joined in B's barrel above the plane: f1 (sector in -> uniform at
  height d) plus f2 (uniform at d -> sector out) is divergence-free and stays in
  0 < z < d, and its energy is at most 2 (E1 + E2) (|a + b|^2 <= 2|a|^2 + 2|b|^2).
  Each plane edge already holds E1 or E2 once; the pass adds E1 + E2 again.
- Via entry: the tube runs to the cross-section at x_e = min(r_p, L - (w/2)
  sum tan(theta/2)) along the arriving segment (L its length, theta the bends
  at its far end), and x_e must exceed r_d * 1.05; otherwise the path is refused.
- Pad end: the end cross-section (width w, perpendicular to the segment) lies
  inside the pad's box, or the path is refused.
- Plane rhombi are cleared of foreign vias' clearance discs and of every
  other own-net via's drilled hole (radius r_d).
"""
import heapq
import math
from collections import defaultdict

from rbound import rho_max, fan, spread, unspread_uniform, bowtie, disc_hits_polygon, BETAS_DEG, key, tube_width_ok


def in_pad(q, r, p):
    """Whether the disc of radius r at q lies inside pad p's copper. The pad is
    the inner box (half sides w/2 - rc, h/2 - rc) grown by a disc of the corner
    radius rc; eroding that by a disc of radius r gives the inner box grown by
    rc - r (r <= rc) or shrunk by r - rc (r > rc)."""
    rc = p["corner"]
    hx, hy = p["w"] / 2 - rc, p["h"] / 2 - rc
    ax, ay = abs(q[0] - p["x"]), abs(q[1] - p["y"])
    if r <= rc:
        return math.hypot(max(0.0, ax - hx), max(0.0, ay - hy)) <= rc - r + 1e-9
    return ax <= hx - (r - rc) + 1e-9 and ay <= hy - (r - rc) + 1e-9


def build(board, net, c, w, planes):
    g = defaultdict(list)
    t_out = c["t_outer_min_mm"]
    r = rho_max(c)
    segs = [s for s in board.segs if s["net"] == net]
    vias = [v for v in board.vias if v["net"] == net]
    pads = [p for p in board.pads if p["net"] == net]
    vc = {}
    for i, v in enumerate(vias):
        for layer in ("F.Cu", "B.Cu"):
            vc[(layer, key((v["x"], v["y"])))] = ("vc", i, layer)
    node = lambda layer, pt: vc.get((layer, key(pt)), ("pt", layer, key(pt)))
    for s in segs:
        a, b = node(s["layer"], s["a"]), node(s["layer"], s["b"])
        cost = r * math.dist(s["a"], s["b"]) / (w * t_out)
        g[a].append((b, cost, ("seg", s)))
        g[b].append((a, cost, ("seg", s)))
    inside = lambda pt, p: abs(pt[0] - p["x"]) <= p["w"] / 2 + 1e-9 and abs(pt[1] - p["y"]) <= p["h"] / 2 + 1e-9
    for p in pads:
        pn = ("pad", p["ref"], p["num"])
        for s in segs:
            if s["layer"] != "F.Cu":
                continue
            for e in (s["a"], s["b"]):
                if inside(e, p):
                    n = node("F.Cu", e)
                    g[pn].append((n, 0.0, ("padend", p)))
                    g[n].append((pn, 0.0, ("padend", p)))
    # a pass through a pad: a straight virtual segment between two track ends
    # inside it, admissible when its tube (a stadium) lies in the pad's copper,
    # which `path_bound` checks at the path's width
    for p in pads:
        ends = sorted({key(e) for s in segs if s["layer"] == "F.Cu" for e in (s["a"], s["b"]) if inside(e, p)})
        for x in range(len(ends)):
            for y in range(x + 1, len(ends)):
                a, b = ends[x], ends[y]
                vs = dict(a=a, b=b, layer="F.Cu", net=net, pad=p)
                cost = r * math.dist(a, b) / (w * t_out)
                na, nb = node("F.Cu", a), node("F.Cu", b)
                g[na].append((nb, cost, ("seg", vs)))
                g[nb].append((na, cost, ("seg", vs)))
    rp, rd = c["via_pad_mm"] / 2, c["drill_mm"] / 2
    for i, v in enumerate(vias):
        f = fan(c, w, t_out, rp, rd)
        for layer in ("F.Cu", "B.Cu"):
            g[("vc", i, layer)].append((("via", i, layer), f, ("fan", i, layer)))
            g[("via", i, layer)].append((("vc", i, layer), f, ("fan", i, layer)))
        if not planes:
            g[("via", i, "F.Cu")].append((("via", i, "B.Cu"), 0.0, ("barrel", i)))
            g[("via", i, "B.Cu")].append((("via", i, "F.Cu"), 0.0, ("barrel", i)))
        else:
            g[("via", i, "F.Cu")].append((("plane", i), 0.0, ("barrel-plane", i)))
            g[("plane", i)].append((("via", i, "F.Cu"), 0.0, ("barrel-plane", i)))
    if planes:
        add_plane(g, board, net, c, vias)
    return g, vias


def add_plane(g, board, net, c, vias):
    rd = c["drill_mm"] / 2
    d = c["plane_depth_min_mm"] / 4
    foreign = [v for v in board.vias if v["net"] != net]
    rx, edge = c["antipad_radius_mm"], c["edge_mm"]
    x0, y0, x1, y1 = board.outline
    for i, A in enumerate(vias):
        for k, B in enumerate(vias):
            if k <= i:
                continue
            ax, ay, bx, by = A["x"], A["y"], B["x"], B["y"]
            D = math.dist((ax, ay), (bx, by))
            ux, uy = (bx - ax) / D, (by - ay) / D
            mx, my = (ax + bx) / 2, (ay + by) / 2
            own = [v for j, v in enumerate(vias) if j not in (i, k)]
            best = None
            for bdeg in BETAS_DEG:
                beta = math.radians(bdeg)
                h = D / 2 * math.tan(beta)
                rh = [(ax, ay), (mx - uy * h, my + ux * h), (bx, by), (mx + uy * h, my - ux * h)]
                if any(disc_hits_polygon((f["x"], f["y"]), rx, rh) for f in foreign):
                    continue
                if any(disc_hits_polygon((f["x"], f["y"]), rd, rh) for f in own):
                    continue
                if any(not (x0 + edge <= px <= x1 - edge and y0 + edge <= py <= y1 - edge) for px, py in rh):
                    continue
                cost = 2 * unspread_uniform(c, rd, beta, d) + bowtie(c, rd, beta, D)
                if best is None or cost < best[0]:
                    best = (cost, beta)
            if best:
                g[("plane", i)].append((("plane", k), best[0], ("plane", i, k, best[1], best[0])))
                g[("plane", k)].append((("plane", i), best[0], ("plane", i, k, best[1], best[0])))


ALLOWED = {  # node kind, arrived by -> may leave by
    ("vc", "seg"): {"fan"},
    ("vc", "fan"): {"seg"},
    ("via", "fan"): {"barrel", "barrel-plane"},
    ("via", "barrel"): {"fan"},
    ("via", "barrel-plane"): {"fan"},
    ("plane", "barrel-plane"): {"plane"},
    ("plane", "plane"): {"plane", "barrel-plane"},
}


def shortest(g, c, src, bans=frozenset()):
    """Dijkstra over states (node, arrived-by kind, detail), with ALLOWED's
    rules, no hop through a pad, plane passes priced, and `bans`: pairs
    (a, b) of consecutive step details a found path showed inadmissible
    (a track and the fan or pad end beside it, or a pad pass)."""
    rd, d = c["drill_mm"] / 2, c.get("plane_depth_min_mm", 1.0) / 4
    start = (src, None, None)
    dist, prev = {start: 0.0}, {}
    pq, seq, done = [(0.0, 0, start)], 1, set()
    while pq:
        dd, _, st = heapq.heappop(pq)
        if st in done:
            continue
        done.add(st)
        u, came, det = st
        if u[0] == "pad" and came is not None:
            continue
        for v, cost, tag in g[u]:
            kind = tag[0]
            if came is not None and (u[0], came) in ALLOWED and kind not in ALLOWED[(u[0], came)]:
                continue
            ndet = step_detail(tag)
            if ndet in bans or (det, ndet) in bans or (ndet, det) in bans:
                continue
            extra = 0.0
            if u[0] == "plane" and came == "plane" and kind == "plane":
                extra = unspread_uniform(c, rd, det[1], d) + unspread_uniform(c, rd, tag[3], d)
            ns = (v, kind, ndet)
            nd = dd + cost + extra
            if nd < dist.get(ns, math.inf):
                dist[ns], prev[ns] = nd, (st, tag)
                heapq.heappush(pq, (nd, seq, ns))
                seq += 1
    return dist, prev


def step_detail(tag):
    k = tag[0]
    if k == "seg":
        return ("seg", id(tag[1]))
    if k == "plane":
        return ("plane", tag[3])
    if k == "padend":
        return ("padend", tag[1]["ref"], tag[1]["num"])
    if k == "fan":
        return ("fan", tag[1], tag[2])
    return (k, tag[1])


def path_to(prev, dist, target):
    """The path to `target` as (Dijkstra cost, nodes, tags), tags[k] joining
    nodes[k] to nodes[k + 1]."""
    best = min((s for s in dist if s[0] == target), key=lambda s: dist[s], default=None)
    if best is None:
        return None, [], []
    tags, nodes, s = [], [best[0]], best
    while s in prev:
        s, tag = prev[s]
        tags.append(tag)
        nodes.append(s[0])
    return dist[best], nodes[::-1], tags[::-1]


def path_bound(nodes, tags, vias, c, w, H, plane_depth):
    """Recompute one path's bound from its own geometry: (R, None), or
    (None, reason) when a condition of a piece's flow fails."""
    rho, t_out = rho_max(c), c["t_outer_min_mm"]
    rp, rd = c["via_pad_mm"] / 2, c["drill_mm"] / 2
    tp = c["plating_min_mm"]
    A = tp * (2 * rd - tp) / 2
    xy = lambda n: n[2] if n[0] == "pt" else (vias[n[1]]["x"], vias[n[1]]["y"])
    # oriented segments, by step index
    seg = {}
    for k, tag in enumerate(tags):
        if tag[0] == "seg":
            a, b = xy(nodes[k]), xy(nodes[k + 1])
            seg[k] = (a, b)
    def tan_half(k0, k1):
        (a0, b0), (a1, b1) = seg[k0], seg[k1]
        v0 = (b0[0] - a0[0], b0[1] - a0[1]); v1 = (b1[0] - a1[0], b1[1] - a1[1])
        return math.tan(abs(math.atan2(v0[0] * v1[1] - v0[1] * v1[0], v0[0] * v1[0] + v0[1] * v1[1])) / 2)
    for k, tag in enumerate(tags):
        if tag[0] == "seg" and "pad" in tag[1]:
            for q in seg[k]:
                if not in_pad(q, w / 2, tag[1]["pad"]):
                    return None, f"pad pass: the tube leaves {tag[1]['pad']['ref']}.{tag[1]['pad']['num']}", ("ban", step_detail(tag))
    total = sum(rho * math.dist(a, b) / (w * t_out) for a, b in seg.values())
    xes = defaultdict(list)  # per via: (p, s0) of each track end's cut
    for k, tag in enumerate(tags):
        if tag[0] == "fan":
            i = tag[1]
            centre = (vias[i]["x"], vias[i]["y"])
            step = -1 if (k - 1) in seg else 1
            kk = k + step
            poly = []
            while kk in seg:
                a, b = seg[kk]
                if step < 0:  # arriving: a -> b ends at the via
                    poly = [a] + (poly if poly else [b])
                else:  # leaving: reverse so it heads to the via
                    poly = [b] + (poly if poly else [a])
                kk += step
            cut = best_cut(poly, centre, c, w, t_out)
            if cut is None:
                return None, "via entry: no admissible cross-section inside the via pad", ("ban", (step_detail(tags[k + step]), step_detail(tag)))
            saved, p, s0, e = cut
            total += e - rho * saved / (w * t_out)
            xes[i].append((p, s0))
        elif tag[0] == "padend":
            p = tag[1]
            j = k + 1 if k + 1 in seg else k - 1
            a, b = seg[j]
            e, o = (a, b) if j > k else (b, a)
            L = math.dist(a, b)
            nx, ny = -(o[1] - e[1]) / L, (o[0] - e[0]) / L
            # the widest cross-section about e, along (nx, ny), inside the copper
            lo_, hi_ = 0.0, 2 * max(p["w"], p["h"])
            for _ in range(50):
                mid = (lo_ + hi_) / 2
                ok_ = all(in_pad((e[0] + sg * nx * mid / 2, e[1] + sg * ny * mid / 2), 0.0, p) for sg in (1, -1))
                lo_, hi_ = (mid, hi_) if ok_ else (lo_, mid)
            lim = lo_
            if lim < w - 1e-12:
                return None, f"pad end: the cross-section leaves {p['ref']}.{p['num']} (max width {lim:.3f})", lim
    dpl = plane_depth[0] / 4
    last_plane = None
    for k, tag in enumerate(tags):
        if tag[0] == "barrel":
            (pa, sa), (pb, sb) = xes[tag[1]][0], xes[tag[1]][-1]
            total += min(
                spread_line(c, w, pa, sa, rd, dd) + spread_line(c, w, pb, sb, rd, dd) + rho / A * (H[1] - 2 * dd) / (2 * math.pi)
                for dd in [H[0] / 2 * f for f in (0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0)]
            )
        elif tag[0] == "barrel-plane":
            pa, sa = xes[tag[1]][0]
            total += spread_line(c, w, pa, sa, rd, dpl) + rho / A * (plane_depth[1] - 2 * dpl) / (2 * math.pi)
        elif tag[0] == "plane":
            total += tag[4]
            if last_plane is not None and tags[k - 1][0] == "plane":
                total += unspread_uniform(c, rd, last_plane, dpl) + unspread_uniform(c, rd, tag[3], dpl)
            last_plane = tag[3]
    return total, None, None


# ---------------------------------------------------------- the general fan
#
# A cross-section line of width w, its points q(s) = m + s u (|s| <= w/2),
# the track's flow direction t normal to it, the via centre c ahead of every
# point ((c - q).t > 0), the whole line inside the via pad disc
# (|q - c| <= r_p) and outside the hole (|q - c| > r_d). With p = (c - m).t
# and s0 = (c - m).u, |q - c|^2 = p^2 + (s - s0)^2. The tube's uniform flux
# I/w through the line is carried along rays to the centre with angular
# density j = (I/w) |q - c|^2 / p (ds/dphi = |q - c|^2 / p), so
#   E_fan = (rho/t) (I/w)^2 / p * integral(|q - c|^2 ln(|q - c| / r_d) ds).
# Rays from points of the disc toward its centre stay in the disc, so the
# region is copper (the via's pad). At the hole wall that density enters the
# barrel, and the spread to uniform is rbound's E_spread with phi0(s) =
# atan((s - s0)/p), Phi(s) = 2 pi (s + w/2)/w - pi + phi0(0), and the measure
# dphi0 = p / |q - c|^2 ds.

from scipy.integrate import quad as _quad


def fan_line(c, w, t, p, s0, rd):
    f = lambda s: (p * p + (s - s0) ** 2) * math.log(math.sqrt(p * p + (s - s0) ** 2) / rd)
    val, err = _quad(f, -w / 2, w / 2, epsabs=1e-14, epsrel=1e-12)
    return rho_max(c) / t / (w * w) / p * (val + 10 * err)


def spread_line(c, w, p, s0, rd, d):
    tp = c["plating_min_mm"]
    A = tp * (2 * rd - tp) / 2
    ph0 = math.atan(-s0 / p)

    def f(s):
        r2 = p * p + (s - s0) ** 2
        j0 = r2 / (w * p)
        x = 2 * math.pi * j0
        L = math.log(x) / (x - 1) if abs(x - 1) > 1e-12 else 1.0
        phi = math.atan((s - s0) / p)
        Phi = 2 * math.pi * (s + w / 2) / w - math.pi + ph0
        return j0 * j0 * L * (d + rd * rd * (Phi - phi) ** 2 / d) * (p / r2)

    val, err = _quad(f, -w / 2, w / 2, epsabs=1e-14, epsrel=1e-12)
    return rho_max(c) / A * (val + 10 * err)


def best_cut(poly, centre, c, w, t_out):
    """poly: points heading to the via centre (poly[-1] == centre), at least
    one segment. Returns (saved tube length, p, s0, fan energy) for the best
    admissible cross-section on the last three segments, or None."""
    rp, rd = c["via_pad_mm"] / 2, c["drill_mm"] / 2
    rho = rho_max(c)
    best = None
    n = len(poly)
    after = 0.0  # polyline length from the segment's far end to the centre
    for m in range(n - 1, max(0, n - 4), -1):
        a, b = poly[m - 1], poly[m]
        L = math.dist(a, b)
        if L < 1e-12:
            continue
        tx, ty = (b[0] - a[0]) / L, (b[1] - a[1]) / L
        ux, uy = -ty, tx
        # bend at a (between poly[m-2]->a and a->b): the tube needs
        # (w/2) tan(theta/2) of straight track after a
        need = 0.0
        if m - 2 >= 0:
            a0 = poly[m - 2]
            v0 = (a[0] - a0[0], a[1] - a0[1])
            th = abs(math.atan2(v0[0] * ty - v0[1] * tx, v0[0] * tx + v0[1] * ty))
            need = (w / 2) * math.tan(th / 2)
        for k in range(1, 41):
            sc = L * k / 40  # distance from a along the segment
            if sc < need - 1e-12:
                continue
            mx, my = a[0] + tx * sc, a[1] + ty * sc
            qs = [(mx + sg * ux * w / 2, my + sg * uy * w / 2) for sg in (1, -1)]
            if any(math.dist(q, centre) > rp + 1e-9 for q in qs):
                continue
            p = (centre[0] - mx) * tx + (centre[1] - my) * ty
            s0 = (centre[0] - mx) * ux + (centre[1] - my) * uy
            if p <= 0 or any((centre[0] - q[0]) * tx + (centre[1] - q[1]) * ty <= 0 for q in qs):
                continue
            # distance from the centre to the line segment
            dmin = p if abs(s0) <= w / 2 else math.hypot(p, abs(s0) - w / 2)
            if dmin <= 1.05 * rd:
                continue
            saved = (L - sc) + after
            e = fan_line(c, w, t_out, p, s0, rd)
            cost = e - rho * saved / (w * t_out)
            if best is None or cost < best[0]:
                best = (cost, saved, p, s0, e)
        after += L
    return None if best is None else best[1:]
