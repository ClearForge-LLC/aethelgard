"""S4: the spread barrel flow against a certified lower bound.

The barrel bound is (rho/A) * integral(q^2 + r_d^2 p^2) over a 2D flow (p, q)
in (phi, z). With s = r_d phi this is exactly the energy of the same flow in an
isotropic periodic sheet of width 2 pi r_d, height H and sheet resistance
R_sq = rho r_d / A. So the spread flow, entering on the arc |phi| <= a with
density j0 and leaving the far edge after the uniform part, is an admissible
flow for that sheet with two terminals: the entry arc (one equipotential) and
the far edge. Hence spread + uniform >= R_sheet(exact) >= 1/E, E the P1 primal
energy of the periodic sheet, a certified lower bound. Units: R_sq = 1.
Checked for the symmetric entry (x_e = 0.3, the common case) and offset or
tilted general entries (spread_line), at the reference's and the 4-layer
fixture's barrel lengths.
"""
import math, sys
import numpy as np
import scipy.sparse as sp
sys.path.insert(0, "../s3")
from bracket import stiffness, solve_fixed
from rbound2 import spread_line

rd, tp = 0.15, 0.018
A = tp * (2 * rd - tp) / 2
c = {"rho20_ohm_mm": A / rd, "alpha": 0.0, "t_max_c": 20, "k_material": 1.0, "plating_min_mm": tp}  # rho/A = 1/rd -> R_sq = 1
ok = True
W = 0.2
for p, s0, H, d in [(0.3, 0.0, 1.6, 0.3), (0.3, 0.0, 1.07, 0.25), (0.2, 0.05, 1.6, 0.3), (0.17, -0.06, 2.0, 0.4), (0.25, 0.09, 0.34, 0.15)]:
    if H < d:
        continue
    bound = spread_line(c, W, p, s0, rd, d) + (1 / rd) * (H - d) / (2 * math.pi)
    # the entry arc: phi from atan((-W/2 - s0)/p) to atan((W/2 - s0)/p)
    ph_lo, ph_hi = math.atan((-W / 2 - s0) / p), math.atan((W / 2 - s0) / p)
    # periodic grid in (s, z)
    ns, nz = 360, max(60, int(H / (2 * math.pi * rd / 360)))
    L = 2 * math.pi * rd
    sv = np.arange(ns) * L / ns
    zv = np.linspace(0, H, nz + 1)
    idx = lambda i, j: j * ns + (i % ns)
    tris = []
    for j in range(nz):
        for i in range(ns):
            a_, b_, c_, d_ = idx(i, j), idx(i + 1, j), idx(i + 1, j + 1), idx(i, j + 1)
            tris += [(a_, b_, c_), (a_, c_, d_)]
    tris = np.array(tris)
    # coordinates: unwrap per element so the seam has positive area
    P = np.zeros((len(tris), 3, 2))
    for e, (a_, b_, c_) in enumerate(tris):
        for k, n_ in enumerate((a_, b_, c_)):
            i, j = n_ % ns, n_ // ns
            P[e, k] = (sv[i], zv[j])
        for k in range(3):
            if P[e, k, 0] < P[e, 0, 0] - L / 2:
                P[e, k, 0] += L
    # assemble with explicit element coordinates
    pts = P.reshape(-1, 2)
    local = np.arange(len(tris) * 3).reshape(-1, 3)
    Kl = stiffness(pts, local, np.ones(len(tris)), ndof=ns * (nz + 1), dofmap=tris)
    phi = (sv / rd + math.pi) % (2 * math.pi) - math.pi
    top = [idx(i, 0) for i in range(ns) if ph_lo - 1e-12 <= phi[i] <= ph_hi + 1e-12]
    bot = [idx(i, nz) for i in range(ns)]
    _, E = solve_fixed(Kl, np.array(top + bot), np.concatenate([np.ones(len(top)), np.zeros(len(bot))]))
    R_lo = 1 / E
    held = bound >= R_lo
    ok &= held
    print(f"p {p} s0 {s0:+.2f} H {H} d {d}: spread bound {bound:8.3f} sq, certified sheet R >= {R_lo:8.3f} sq, ratio {bound/R_lo:5.2f}, "
          f"uniform-only {H/(2*math.pi*rd):7.3f} sq: {'holds' if held else 'FAILS'}")
print("ALL HOLD" if ok else "A CHECK FAILS")
sys.exit(0 if ok else 1)
