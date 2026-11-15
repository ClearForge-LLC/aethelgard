"""AET-WO-1303 S4 probe: an upper bound on the copper resistance between two
pads of one supply net, by Thomson's principle.

PRINCIPLE. With the two terminals (pads, each taken as one equipotential: its
solder and lead short it) carrying unit current, any divergence-free current
density J in the copper with J.n = 0 on its free surfaces gives
R <= integral(rho |J|^2 dV). Removing copper never lowers R (Rayleigh), so a
flow confined to one path through the copper bounds the net's R between the
two pads, whatever else the net holds.

THE FLOW, piece by piece (unit current; angles in radians):
- Track chain. Current flows uniformly along the offset curves of the path's
  centreline, |y| <= w/2. At a bend of deviation theta the outer offset gains an
  arc y*theta and the inner one loses 2*y*tan(theta/2), so
  integral(l(y) dy) = L*w + (theta - 2 tan(theta/2)) w^2/8 <= L*w and
  R <= rho * L_centre / (w * t). Admissible only while every inner offset
  stays a curve: on each side of each segment, its length must be at least
  (w/2) * sum(tan(theta/2)) over the bends at its ends whose inside is on
  that side. Where that fails for w = W, the probe narrows the tube for the
  whole path (a narrower flow is still admissible).
- Pad end. The tube starts at the track end inside the pad; its end
  cross-section (width w) must lie inside the pad's box.
- Via end on an outer layer. The tube runs to the cross-section at x_e = r_p
  (the via pad's radius) from the via's centre; from that line, current flows
  along rays toward the centre, with angular density j(phi) =
  x_e / (w cos^2 phi), |phi| <= a = atan(w / (2 x_e)), to the hole's wall at
  r_d. Energy: (rho/t) * integral(j^2 ln(x_e / (r_d cos phi)))
  <= (rho/t) * J2 * ln(x_e / (r_d cos a)), with
  J2 = integral(j^2) = (x_e/w)^2 * 2 (tan a + tan^3 a / 3).
- Barrel (first version, `barrel`; superseded by `barrel2` below). The same angular density runs down the plating, whose area per
  radian is at least t_p (2 r_d - t_p) / 2 (plating inside the drilled hole;
  plating outside a finished hole has more). Turning from the arc facing the
  arriving track to the arc facing the leaving one (at most pi) shears the
  flow: R <= rho * 2 J2 / (t_p (2 r_d - t_p)) * (H + (pi r_d)^2 / H), taken at
  H_max and H_min, the larger.
- Plane (4 layers): `add_plane2` below. Admissible only if the rhombus A, B
  and the two ray ends holds plane copper: no foreign via's clearance disc and
  no board edge clearance crosses it, and the plane meets the barrel all round
  (a condition: the image measurement was blocked, FEEDBACK D9).

Not a flow this probe builds: passing through a via on one layer (a path is
forced to change layers at every via it uses).
"""

import heapq
import math
from collections import defaultdict


def rho_max(c):
    """rho at the condition temperature, with the foil/plating allowance, in
    ohm*mm: rho20 * (1 + alpha (T - 20)) * k."""
    return c["rho20_ohm_mm"] * (1 + c["alpha"] * (c["t_max_c"] - 20)) * c["k_material"]


def j2(xe, w):
    a = math.atan(w / (2 * xe))
    return (xe / w) ** 2 * 2 * (math.tan(a) + math.tan(a) ** 3 / 3), a


def fan(c, w, t, xe, rd):
    J2, a = j2(xe, w)
    return rho_max(c) / t * J2 * math.log(xe / (rd * math.cos(a)))


def barrel(c, w, xe, rd):
    """The first, unspread strip flow: kept only for FEEDBACK F2's comparison."""
    J2, _ = j2(xe, w)
    tp = c["plating_min_mm"]
    base = rho_max(c) * 2 * J2 / (tp * (2 * rd - tp))
    return max(base * (H + (math.pi * rd) ** 2 / H) for H in (c["h_min_mm"], c["h_max_mm"]))


def key(pt):
    return (round(pt[0], 4), round(pt[1], 4))


def build(board, net, c, w):
    """A graph whose nodes are ('pt', layer, xy), ('pad', ref, num),
    ('via', i, layer) and ('plane', i); edges carry an upper bound."""
    g = defaultdict(list)
    t_out = c["t_outer_min_mm"]
    r = rho_max(c)
    segs = [s for s in board.segs if s["net"] == net]
    vias = [v for v in board.vias if v["net"] == net]
    pads = [p for p in board.pads if p["net"] == net]
    for s in segs:
        a, b = ("pt", s["layer"], key(s["a"])), ("pt", s["layer"], key(s["b"]))
        L = math.dist(s["a"], s["b"])
        g[a].append((b, r * L / (w * t_out), ("seg", s)))
        g[b].append((a, r * L / (w * t_out), ("seg", s)))
    inside = lambda pt, p: abs(pt[0] - p["x"]) <= p["w"] / 2 + 1e-9 and abs(pt[1] - p["y"]) <= p["h"] / 2 + 1e-9
    for p in pads:
        node = ("pad", p["ref"], p["num"])
        for s in segs:
            if s["layer"] != "F.Cu":
                continue
            for e in (s["a"], s["b"]):
                if inside(e, p):
                    g[node].append((("pt", "F.Cu", key(e)), 0.0, ("padend", p)))
                    g[("pt", "F.Cu", key(e))].append((node, 0.0, ("padend", p)))
    rp, rd = c["via_pad_mm"] / 2, c["drill_mm"] / 2
    xe = rp
    for i, v in enumerate(vias):
        for layer in ("F.Cu", "B.Cu"):
            pt = ("pt", layer, key((v["x"], v["y"])))
            vn = ("via", i, layer)
            g[pt].append((vn, fan(c, w, t_out, xe, rd), ("fan", i)))
            g[vn].append((pt, fan(c, w, t_out, xe, rd), ("fan", i)))
        bf, bb = ("via", i, "F.Cu"), ("via", i, "B.Cu")
        cost = barrel2(c, w, xe, rd, c["h_min_mm"], c["h_max_mm"])[0]
        g[bf].append((bb, cost, ("barrel", i)))
        g[bb].append((bf, cost, ("barrel", i)))
    return g, vias, segs, pads


def disc_hits_polygon(cen, r, poly):
    px, py = cen
    # inside?
    inside = False
    n = len(poly)
    for i in range(n):
        x1, y1 = poly[i]
        x2, y2 = poly[(i + 1) % n]
        if (y1 > py) != (y2 > py) and px < (x2 - x1) * (py - y1) / (y2 - y1) + x1:
            inside = not inside
    if inside:
        return True
    for i in range(n):
        x1, y1 = poly[i]
        x2, y2 = poly[(i + 1) % n]
        dx, dy = x2 - x1, y2 - y1
        tt = max(0, min(1, ((px - x1) * dx + (py - y1) * dy) / (dx * dx + dy * dy)))
        if math.dist((px, py), (x1 + tt * dx, y1 + tt * dy)) < r:
            return True
    return False


def shortest(g, src):
    """Dijkstra from src over states (node, arrived-by-fan), forbidding a
    same-layer pass through a via (in by a fan, out by a fan)."""
    start = (src, False)
    dist, prev = {start: 0.0}, {}
    pq, seq = [(0.0, 0, start)], 1
    done = set()
    while pq:
        d, _, st = heapq.heappop(pq)
        if st in done:
            continue
        done.add(st)
        u, by_fan = st
        for v, cost, tag in g[u]:
            if u[0] == "via" and by_fan and tag[0] == "fan":
                continue
            ns = (v, tag[0] == "fan" and v[0] == "via")
            nd = d + cost
            if nd < dist.get(ns, math.inf):
                dist[ns], prev[ns] = nd, (st, tag)
                heapq.heappush(pq, (nd, seq, ns))
                seq += 1
    return dist, prev


def path_to(prev, dist, target):
    """The bound to `target`, the edge tags in path order, and the node list."""
    best = min((s for s in dist if s[0] == target), key=lambda s: dist[s], default=None)
    if best is None:
        return None, [], []
    tags, nodes, s = [], [best[0]], best
    while s in prev:
        s, tag = prev[s]
        tags.append(tag)
        nodes.append(s[0])
    return dist[best], tags[::-1], nodes[::-1]


def tube_width_ok(nodes, W):
    """The widest tube <= W for which every bend on the path is admissible:
    on each side of each segment, its length >= (w/2) * sum(tan(theta/2))
    over the bends at its ends whose inside is on that side. Returns the
    width and the bend angles. Runs are maximal chains of track points on
    one layer, in path order."""
    runs, run = [], []
    for n in nodes:
        if n[0] == "pt" and run and run[-1][0] == "pt" and run[-1][1] == n[1]:
            run.append(n)
        else:
            if len(run) > 1:
                runs.append(run)
            run = [n] if n[0] == "pt" else []
    if len(run) > 1:
        runs.append(run)
    w, bends = W, []
    for run in runs:
        pts = [n[2] for n in run]
        segs = list(zip(pts[:-1], pts[1:]))
        turns = [None] * (len(segs) + 1)
        for i in range(1, len(segs)):
            (a0, b0), (a1, b1) = segs[i - 1], segs[i]
            v0 = (b0[0] - a0[0], b0[1] - a0[1])
            v1 = (b1[0] - a1[0], b1[1] - a1[1])
            cross = v0[0] * v1[1] - v0[1] * v1[0]
            dot = v0[0] * v1[0] + v0[1] * v1[1]
            th = abs(math.atan2(cross, dot))
            if th > 1e-9:
                turns[i] = (th, 1 if cross > 0 else -1)
                bends.append(th)
        for i, (a, b) in enumerate(segs):
            L = math.dist(a, b)
            for side in (1, -1):
                tot = sum(math.tan(tn[0] / 2) for tn in (turns[i], turns[i + 1]) if tn and tn[1] == side)
                if tot > 0:
                    w = min(w, 2 * L / tot)
    return w, bends


# ------------------------------------------------------------------ barrel v2
#
# The plating is the shell r_d - t_p < r < r_d. Take J = (J_r, J_phi, J_z) =
# (0, p(phi, z) r / A, q(phi, z) / A), A = t_p (2 r_d - t_p) / 2 its area per
# radian; it is divergence-free when (p, q) is, as a 2D flow in (phi, z), and
# its energy is rho * integral(p^2 M3 / A^2 + q^2 / A) <= (rho / A) *
# integral(q^2 + r_d^2 p^2) dphi dz, since M3 = integral(r^3 dr) <= r_d^2 A.
# q is the current per radian going down.
#
# Spread: the entry density j0 on |phi0| <= a is carried along straight lines
# to the uniform density 1/(2 pi) over a depth d: phi = (1 - l) phi0 + l Phi(phi0),
# l = z/d, Phi = 2 pi C(phi0) - pi, C the cumulative of j0. Then
# dphi/dphi0 = (1 - l) + l 2 pi j0, q = j0 / (dphi/dphi0), p = q (Phi - phi0)/d,
# and over l the integral of 1/(1 + l (x - 1)) is ln(x)/(x - 1), x = 2 pi j0:
#   E_spread = (rho/A) * integral(j0^2 ln(x)/(x-1) (d + r_d^2 (Phi - phi0)^2 / d) dphi0).
# Uniform in the middle: (rho/A) (H - 2d) / (2 pi). Un-spread at the other end
# to the arc facing the leaving track or plane: the same E_spread, mirrored.

from scipy.integrate import quad


def spread(c, w, xe, rd, d):
    tp = c["plating_min_mm"]
    A = tp * (2 * rd - tp) / 2
    a = math.atan(w / (2 * xe))
    k = xe / w

    def f(p0):
        j0 = k / math.cos(p0) ** 2
        x = 2 * math.pi * j0
        Phi = 2 * math.pi * k * (math.tan(p0) + math.tan(a)) - math.pi
        return j0 * j0 * math.log(x) / (x - 1) * (d + rd * rd * (Phi - p0) ** 2 / d)

    val, err = quad(f, -a, a, epsabs=1e-13, epsrel=1e-12, limit=200)
    return rho_max(c) / A * (val + 10 * err)


def barrel2(c, w, xe, rd, h_min, h_max):
    """Spread at both ends, uniform between: the bound at the worst H in
    [h_min, h_max], with d chosen once for all H (d <= h_min / 2)."""
    tp = c["plating_min_mm"]
    A = tp * (2 * rd - tp) / 2
    best = None
    for d in [h_min / 2 * f for f in (0.05, 0.1, 0.15, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0)]:
        val = 2 * spread(c, w, xe, rd, d) + rho_max(c) / A * (h_max - 2 * d) / (2 * math.pi)
        if best is None or val < best[0]:
            best = (val, d)
    return best


# ------------------------------------------------------------------ plane v2
#
# At the plane's depth the barrel's current is uniform (1/(2 pi) per radian),
# so it can be un-spread to any density. Take uniform 1/(2 beta) over the
# sector |phi| <= beta facing the other via. Un-spreading uniform 1/(2 pi) to
# uniform 1/(2 beta) over depth d is the spread above with j0 = 1/(2 beta),
# x = pi/beta and Phi - phi0 = phi0 (pi/beta - 1):
#   E_un = (rho/A) j0^2 L(x) (2 beta d + r_d^2 (pi/beta - 1)^2 (2 beta^3 / 3) / d).
# In the plane, rays from A to the perpendicular bisector, mirrored into B:
#   E_plane <= 2 (rho / t_plane) (1 / (2 beta)) ln(D / (2 r_d cos beta)).
# Admissible only when the rhombus A, B, and the two ray ends holds copper.

BETAS_DEG = (75, 60, 45, 30, 20, 12, 7, 4, 2)


def unspread_uniform(c, rd, beta, d):
    tp = c["plating_min_mm"]
    A = tp * (2 * rd - tp) / 2
    j0 = 1 / (2 * beta)
    x = math.pi / beta
    L = math.log(x) / (x - 1)
    return rho_max(c) / A * j0 * j0 * L * (2 * beta * d + rd * rd * (math.pi / beta - 1) ** 2 * (2 * beta**3 / 3) / d)


def bowtie(c, rd, beta, D):
    return 2 * rho_max(c) / c["t_plane_min_mm"] / (2 * beta) * math.log(D / (2 * rd * math.cos(beta)))


def add_plane2(g, board, net, c, w, vias, outline):
    rd, xe = c["drill_mm"] / 2, c["via_pad_mm"] / 2
    tp = c["plating_min_mm"]
    A = tp * (2 * rd - tp) / 2
    dmin, dmax = c["plane_depth_min_mm"], c["plane_depth_max_mm"]
    d = dmin / 4  # spread over d, un-spread over d: both within the shortest barrel
    foreign = [v for v in board.vias if v["net"] != net]
    rx, edge = c["antipad_radius_mm"], c["edge_mm"]
    x0, y0, x1, y1 = outline
    blocked = 0
    for i, Av in enumerate(vias):
        for k, Bv in enumerate(vias):
            if k <= i:
                continue
            ax, ay, bx, by = Av["x"], Av["y"], Bv["x"], Bv["y"]
            D = math.dist((ax, ay), (bx, by))
            if D <= 2 * rd:
                continue
            ux, uy = (bx - ax) / D, (by - ay) / D
            mx, my = (ax + bx) / 2, (ay + by) / 2
            best = None
            for bdeg in BETAS_DEG:
                beta = math.radians(bdeg)
                h = D / 2 * math.tan(beta)
                rh = [(ax, ay), (mx - uy * h, my + ux * h), (bx, by), (mx + uy * h, my - ux * h)]
                if any(disc_hits_polygon((f["x"], f["y"]), rx, rh) for f in foreign) or any(
                    not (x0 + edge <= px <= x1 - edge and y0 + edge <= py <= y1 - edge) for px, py in rh
                ):
                    continue
                cost = 2 * unspread_uniform(c, rd, beta, d) + bowtie(c, rd, beta, D)
                if best is None or cost < best[0]:
                    best = (cost, bdeg)
            if best is None:
                blocked += 1
                continue
            g[("plane", i)].append((("plane", k), best[0], ("plane", i, k, best[1])))
            g[("plane", k)].append((("plane", i), best[0], ("plane", i, k, best[1])))
    for i, _ in enumerate(vias):
        cost = spread(c, w, xe, rd, d) + rho_max(c) / A * (dmax - 2 * d) / (2 * math.pi)
        g[("via", i, "F.Cu")].append((("plane", i), cost, ("barrel-plane", i)))
        g[("plane", i)].append((("via", i, "F.Cu"), cost, ("barrel-plane", i)))
    return blocked
