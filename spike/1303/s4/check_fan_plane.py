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
print("ALL HOLD" if ok else "A CHECK FAILS")
sys.exit(0 if ok else 1)
