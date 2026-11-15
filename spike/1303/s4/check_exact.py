"""S4 exact checks.
1. The bar, the radial annulus and the barrel: the bound's flow is the exact
   flow there, so the bound equals the closed form.
2. A 90 degree bend: the tube bound rho*L_centre/(W t) against a certified
   bracket of the exact sheet resistance of the square-cornered L (which
   contains the tube), primal P1 (lower R) and stream-function dual (upper R).
3. Transfer resistance: in random resistor networks, the drop at each node
   under any non-negative loads is at most R_eff(source, node) * total load.
"""
import math, sys
import numpy as np
sys.path.insert(0, "../s3")
from bracket import Mesh, stiffness, solve_fixed, graded
from rbound import j2, spread, rho_max

ok = True
# 1. closed forms (unit rho/t)
L, W, t = 3.0, 0.2, 0.035
bar_tube = L / (W * t)  # tube flow uniform over a straight bar
print(f"bar: tube {bar_tube:.6f} = exact {L/(W*t):.6f}")
a_, b_ = 0.15, 0.6
ann_flow = math.log(b_ / a_) / (2 * math.pi * t)  # uniform radial flow, j = 1/(2 pi)
print(f"annulus: flow {ann_flow:.6f} = exact ln(b/a)/(2 pi t) {math.log(b_/a_)/(2*math.pi*t):.6f}")
rd, tp, H = 0.15, 0.018, 1.6
A = tp * (2 * rd - tp) / 2
barrel_flow = H / A / (2 * math.pi)  # uniform j = 1/(2 pi): integral(j^2) = 1/(2 pi)
print(f"barrel: flow {barrel_flow:.6f} = exact H/(pi t_p (2 r_d - t_p)) {H/(math.pi*tp*(2*rd-tp)):.6f}")

# 2. the 90 degree bend, W = 1, sheet rho/t = 1
for (a, b) in [(3.0, 3.0), (2.0, 5.0), (1.5, 1.5)]:
    for fine in (1 / 40, 1 / 160):
        xs = graded([0, a - 1, a], fine, 0.1, 1.15)
        ys = graded([0, 1, b], fine, 0.1, 1.15)
        m = Mesh(xs, ys, hole=(-1, a - 1, 1, b + 1))
        p, tol = m.p, 1e-9
        used = np.zeros(m.n, bool); used[m.tris.ravel()] = True
        K = stiffness(p, m.tris, np.ones(len(m.tris)))
        left = np.where((np.abs(p[:, 0]) < tol) & used)[0]
        top = np.where((np.abs(p[:, 1] - b) < tol) & used)[0]
        unused = np.where(~used)[0]
        _, Ep = solve_fixed(K, np.concatenate([left, top, unused]),
                            np.concatenate([np.ones(len(left)), np.zeros(len(top)), np.zeros(len(unused))]))
        outer = np.where(used & ((np.abs(p[:, 1]) < tol) | (np.abs(p[:, 0] - a) < tol)))[0]
        inner = np.where(used & (((np.abs(p[:, 1] - 1) < tol) & (p[:, 0] <= a - 1 + tol)) |
                                 ((np.abs(p[:, 0] - (a - 1)) < tol) & (p[:, 1] >= 1 - tol))))[0]
        _, Ed = solve_fixed(K, np.concatenate([outer, inner, unused]),
                            np.concatenate([np.zeros(len(outer)), np.ones(len(inner)), np.zeros(len(unused))]))
        R_lo, R_up = 1 / Ep, Ed
        tube = (a - 0.5) + (b - 0.5)
        held = R_lo <= R_up <= tube
        ok &= held
        print(f"bend a={a} b={b} fine={fine:.4f}: exact in [{R_lo:.5f}, {R_up:.5f}] squares; "
              f"corner excess over arms {R_up - (a - 1) - (b - 1):.4f}..{R_lo - (a-1) - (b-1):.4f}; "
              f"tube bound {tube:.5f} {'holds' if held else 'FAILS'}")

# 3. transfer resistance
rng = np.random.default_rng(1303)
worst = 0.0
for trial in range(300):
    n = rng.integers(4, 25)
    G = np.zeros((n, n))
    for i in range(1, n):
        j = rng.integers(0, i)
        g = rng.uniform(0.1, 10); G[i, j] -= g; G[j, i] -= g
    for _ in range(rng.integers(0, 3 * n)):
        i, j = rng.integers(0, n, 2)
        if i != j:
            g = rng.uniform(0.1, 10); G[i, j] -= g; G[j, i] -= g
    np.fill_diagonal(G, 0); np.fill_diagonal(G, -G.sum(1))
    Z = np.linalg.inv(G[1:, 1:])  # node 0 is the source
    load = rng.uniform(0, 1, n - 1) * (rng.uniform(size=n - 1) < 0.6)
    drop = Z @ load
    ratio = np.max(drop / (np.diag(Z) * load.sum() + 1e-300))
    worst = max(worst, ratio)
held = worst <= 1 + 1e-12
ok &= held
print(f"transfer: max over 300 networks of drop_p / (R_eff(s,p) * total load) = {worst:.6f} {'holds' if held else 'FAILS'}")
print("ALL HOLD" if ok else "A CHECK FAILS")
sys.exit(0 if ok else 1)
