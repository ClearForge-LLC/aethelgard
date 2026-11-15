"""S4 exact and certified checks for the pieces check_exact.py does not cover.

1. Plane bowtie. Two equipotential holes of radius r_d, centres D apart, in an
   infinite sheet: R = (rho / (pi t)) acosh(D / (2 r_d)) exactly (bipolar
   coordinates). The bowtie's flow is admissible there (the rhombus is part of
   the sheet; a terminal takes any flux), so bowtie(beta, D) >= exact, for every
   beta and D. (rho/t = 1 here; the barrel's un-spread terms are not included.)
2. General fan. Copper K = a track of width w and length L ending at a via
   centre, plus the via pad disc r_p, less the hole r_d; terminals: the track's
   far end, and the hole's rim taken as one equipotential (the barrel top).
   The tube to the chosen cut plus fan_line is an admissible flow in K, so it
   bounds R(K) from above. A certified LOWER bound on R(K): the P1 primal
   energy on a grid domain K' containing K, with the hole terminal grown into
   the hole (both only raise conductance), gives G(K') <= E, so
   R(K) >= R(K') >= 1/E. Checked: bound >= 1/E, at straight and turned
   (45 and 90 degree, inside the pad) entries.
3. Pass through a pad. The same certified lower bound for two tracks joined
   through a rounded pad, against the tube plus the virtual segment.
"""
import math, sys, json
import numpy as np
sys.path.insert(0, "../s3")
from bracket import Mesh, stiffness, solve_fixed
from rbound import bowtie
from rbound2 import fan_line, best_cut

ok = True
c = {"rho20_ohm_mm": 1.0, "alpha": 0.0, "t_max_c": 20, "k_material": 1.0, "t_plane_min_mm": 1.0,
     "plating_min_mm": 0.018, "via_pad_mm": 0.6, "drill_mm": 0.3, "t_outer_min_mm": 1.0}
rd = 0.15
print("1. bowtie vs exact two-hole sheet (squares)")
for D in (1.0, 3.0, 10.0, 30.0):
    exact = math.acosh(D / (2 * rd)) / math.pi
    for bdeg in (75, 45, 20, 7, 2):
        b = bowtie(c, rd, math.radians(bdeg), D)
        held = b >= exact
        ok &= held
        print(f"   D {D:5.1f} beta {bdeg:2d}: bowtie {b:8.4f} exact {exact:7.4f} ratio {b/exact:6.2f} {'holds' if held else 'FAILS'}")

print("2. fan + tube vs a certified lower bound on R(K) (squares)")
W, rp = 0.2, 0.3
for name, poly in [("straight", [(-1.2, 0.0), (0.0, 0.0)]),
                   ("45 deg turn at 0.125", [(-1.0, -0.875 + 0.0), (-0.125 - 0.0, 0.0 - 0.0), (0.0, 0.0)]),
                   ("90 deg turn at 0.125", [(-0.125, -1.0), (-0.125, 0.0), (0.0, 0.0)])]:
    if name.startswith("45"):
        poly = [(-0.125 - 0.875 / math.sqrt(2) * math.sqrt(2) / math.sqrt(2), -0.875 / math.sqrt(2)), (-0.125, 0.0), (0.0, 0.0)]
    cut = best_cut(poly, (0.0, 0.0), c, W, 1.0)
    if cut is None:
        print(f"   {name}: no admissible cut"); ok = False; continue
    saved, p, s0, e = cut
    Lpath = sum(math.dist(a, b) for a, b in zip(poly[:-1], poly[1:]))
    bound = (Lpath - saved) / W + e
    # grid superset K' of K: elements touching K (centroid within h/sqrt2 of K)
    h = 0.004
    xs = np.arange(-1.3, 0.36, h); ys = np.arange(-1.1, 0.36, h)
    m = Mesh(xs, ys)
    cen = m.centroid()
    def dist_poly(pt):
        best = 1e9
        for a, b in zip(poly[:-1], poly[1:]):
            ax, ay, bx, by = *a, *b
            dx, dy = bx - ax, by - ay
            tt = max(0, min(1, ((pt[0]-ax)*dx + (pt[1]-ay)*dy) / (dx*dx + dy*dy)))
            best = min(best, math.hypot(pt[0]-ax-tt*dx, pt[1]-ay-tt*dy))
        return best
    grow = h * 1.5
    inK = np.array([(dist_poly(q) <= W / 2 + grow) or (math.hypot(*q) <= rp + grow) for q in cen])
    tris = m.tris[inK]
    used = np.zeros(m.n, bool); used[tris.ravel()] = True
    K = stiffness(m.p, tris, np.ones(len(tris)))
    far = poly[0]
    # far-end terminal: nodes within the track's first h of its start, on its cross-section side
    a0, a1 = poly[0], poly[1]
    tx, ty = (a1[0]-a0[0]) / math.dist(a0, a1), (a1[1]-a0[1]) / math.dist(a0, a1)
    term_far = np.where(used & (((m.p[:, 0]-a0[0])*tx + (m.p[:, 1]-a0[1])*ty) <= 0.0))[0]
    term_hole = np.where(used & (np.hypot(m.p[:, 0], m.p[:, 1]) <= rd + grow))[0]
    unused = np.where(~used)[0]
    _, E = solve_fixed(K, np.concatenate([term_far, term_hole, unused]),
                       np.concatenate([np.ones(len(term_far)), np.zeros(len(term_hole)), np.zeros(len(unused))]))
    R_lo = 1 / E
    held = bound >= R_lo
    ok &= held
    print(f"   {name}: tube+fan bound {bound:7.4f}, certified R >= {R_lo:7.4f}, ratio {bound/R_lo:5.2f} {'holds' if held else 'FAILS'}")
print("3. pass through a pad: tube + virtual segment vs a certified lower bound on R(K) (squares)")
# K = two tracks of width W ending inside a pad (box px x py, corner rc) plus the pad;
# terminals at the tracks' far ends. The bound: both tracks' centre lengths to
# their ends inside the pad, plus the straight virtual segment between the ends,
# all at width W (admissible when in_pad holds for the stadium and the bends'
# length condition holds, as rbound2 checks).
from rbound2 import in_pad
from rbound import tube_width_ok
for name, pad, e1, e2, f1, f2 in [
        ("straight across", dict(x=0, y=0, w=1.0, h=0.6, corner=0.15), (-0.375, 0.0), (0.375, 0.0), (-1.5, 0.0), (1.5, 0.0)),
        ("corner inside the pad", dict(x=0, y=0, w=1.0, h=1.0, corner=0.25), (-0.25, 0.0), (0.0, -0.25), (-1.5, 0.0), (0.0, -1.5)),
        ("offset ends", dict(x=0, y=0, w=1.2, h=0.6, corner=0.15), (-0.375, 0.125), (0.375, -0.125), (-1.5, 0.125), (1.5, -0.125))]:
    poly = [f1, e1, e2, f2]
    w, _ = tube_width_ok([("pt", "F.Cu", q) for q in poly], W)
    stad_ok = all(in_pad(q, w / 2, pad) for q in (e1, e2))
    if not stad_ok:
        print(f"   {name}: virtual segment not admissible"); ok = False; continue
    bound = sum(math.dist(a, b) for a, b in zip(poly[:-1], poly[1:])) / w
    h = 0.004
    xs = np.arange(-1.55, 1.56, h); ys = np.arange(-1.55, 0.6, h)
    m = Mesh(xs, ys); cen = m.centroid(); grow = h * 1.5
    def dseg(pt, a, b):
        dx, dy = b[0]-a[0], b[1]-a[1]
        tt = max(0, min(1, ((pt[0]-a[0])*dx + (pt[1]-a[1])*dy) / (dx*dx + dy*dy)))
        return math.hypot(pt[0]-a[0]-tt*dx, pt[1]-a[1]-tt*dy)
    def in_box_grown(pt):
        return abs(pt[0]-pad["x"]) <= pad["w"]/2 + grow and abs(pt[1]-pad["y"]) <= pad["h"]/2 + grow
    inK = np.array([in_box_grown(q) or dseg(q, f1, e1) <= W/2 + grow or dseg(q, e2, f2) <= W/2 + grow for q in cen])
    tris = m.tris[inK]
    used = np.zeros(m.n, bool); used[tris.ravel()] = True
    K = stiffness(m.p, tris, np.ones(len(tris)))
    def behind(a, b):
        tx, ty = (b[0]-a[0]) / math.dist(a, b), (b[1]-a[1]) / math.dist(a, b)
        return used & (((m.p[:, 0]-a[0])*tx + (m.p[:, 1]-a[1])*ty) <= 0.0) & (np.hypot(m.p[:, 0]-a[0], m.p[:, 1]-a[1]) <= W)
    t1 = np.where(behind(f1, e1))[0]; t2 = np.where(behind(f2, e2))[0]
    unused = np.where(~used)[0]
    _, E = solve_fixed(K, np.concatenate([t1, t2, unused]), np.concatenate([np.ones(len(t1)), np.zeros(len(t2)), np.zeros(len(unused))]))
    R_lo = 1 / E
    held = bound >= R_lo
    ok &= held
    print(f"   {name}: tube width {w:.3f}, bound {bound:7.4f}, certified R >= {R_lo:7.4f}, ratio {bound/R_lo:5.2f} {'holds' if held else 'FAILS'}")
print("4. via entry admissibility, brute force: every chosen cut's fan region is via-pad copper")
import random
random.seed(1303)
bad = tried = cuts = 0
for _ in range(400):
    # a random 2-3 segment approach on the 0.125 mm grid, 45-degree moves, ending at the centre
    pts = [(0.0, 0.0)]
    dirs = [(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)]
    for _k in range(random.randint(1, 3)):
        dx, dy = random.choice(dirs); n = random.randint(1, 4)
        last = pts[0]
        pts.insert(0, (last[0] - dx * 0.125 * n, last[1] - dy * 0.125 * n))
    if len(set(pts)) < len(pts):
        continue
    tried += 1
    cut = best_cut(pts, (0.0, 0.0), c, W, 1.0)
    if cut is None:
        continue
    cuts += 1
    saved, p, s0, e = cut
    # reconstruct the cut line from (p, s0): q(s) = c - p t - s0 u + s u, |s| <= W/2, any (t, u)
    # rotation-free test: points along rays from the line to the rim lie at radius in [r_d, max |q|]
    for s_ in np.linspace(-W / 2, W / 2, 41):
        rq = math.hypot(p, s_ - s0)
        if not (1.05 * rd < rq <= rp + 1e-9):
            bad += 1
print(f"   {tried} random approaches, {cuts} admissible cuts; points of cut lines outside the via-pad annulus: {bad}")
ok &= bad == 0
print("ALL HOLD" if ok else "A CHECK FAILS")
sys.exit(0 if ok else 1)
