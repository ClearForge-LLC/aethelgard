"""AET-WO-1303 S3 probe: a two-sided variational bracket on the odd-mode
capacitance per unit length of an edge-coupled pair, in two dimensions.

Upper bound (Dirichlet, Chew 7.1.48): the energy of any admissible trial
potential psi (1 on the strip, 0 on every ground) bounds C from above. The
trial here is the P1 minimiser on a graded mesh inside a GROUNDED box; adding
grounded conductors only raises C, so C_open <= C_box <= E(psi).

Lower bound (Thomson): for any divergence-free D carrying flux Q off the
strip, C >= Q^2 / integral(|D|^2 / eps). In 2D, D = rot(chi) is divergence
free for any continuous P1 chi; D.n = 0 on an insulating boundary means chi is
constant there; the flux off the strip is chi's jump around it. The trial is
the P1 minimiser inside an INSULATED box (D = 0 outside it), with chi = 0 on
the box, a jump Q across a cut from the strip to the plane, and chi free on
every conductor.

Odd mode: the symmetry plane x = 0 is a ground (an electric wall), so the
half-domain x >= 0 holds one strip, the wall, and the plane.

Both energies are evaluated in float64 on the computed P1 functions; the
solver's residual only moves each energy further from the discrete optimum,
on the safe side. Rounding of the energy sum is ~1e-13 relative.
"""

import numpy as np
import scipy.sparse as sp
import scipy.sparse.linalg as spla
from scipy.special import ellipk

EPS0 = 8.8541878188e-12  # F/m, CODATA 2022 (only ratios matter for Z)
C0 = 299792458.0


def graded(points, fine, coarse, ratio=1.25):
    """Sorted grid coordinates holding every `points`, spacing `fine` at each
    point and growing geometrically by `ratio` up to `coarse` between them."""
    pts = sorted(set(points))
    out = [pts[0]]
    for a, b in zip(pts[:-1], pts[1:]):
        # grow from both ends toward the middle
        left, right = [a], [b]
        step = fine
        while True:
            la, rb = left[-1], right[-1]
            if rb - la <= 2 * step:
                break
            left.append(la + step)
            right.append(rb - step)
            step = min(step * ratio, coarse)
        seg = left + right[::-1]
        mid = (left[-1] + right[-1]) / 2
        if right[-1] - left[-1] > 1.5 * step:
            seg = left + [mid] + right[::-1]
        out.extend(seg[1:])
    return np.array(sorted(set(np.round(out, 12))))


class Mesh:
    """Tensor grid over [0, X] x [z0, Z], each cell split into two triangles;
    cells inside the strip rectangle are removed (t > 0), or the strip is the
    segment z = 0 on the top boundary of a half-domain (stripline symmetry)."""

    def __init__(self, xs, zs, hole=None):
        self.xs, self.zs = xs, zs
        nx, nz = len(xs), len(zs)
        self.n = nx * nz
        idx = lambda i, j: j * nx + i
        tris = []
        for j in range(nz - 1):
            for i in range(nx - 1):
                if hole is not None:
                    x1, x2, z1, z2 = hole
                    cx = (xs[i] + xs[i + 1]) / 2
                    cz = (zs[j] + zs[j + 1]) / 2
                    if x1 < cx < x2 and z1 < cz < z2:
                        continue
                a, b, c, d = idx(i, j), idx(i + 1, j), idx(i + 1, j + 1), idx(i, j + 1)
                tris.append((a, b, c))
                tris.append((a, c, d))
        self.tris = np.array(tris)
        X, Z = np.meshgrid(xs, zs)
        self.p = np.column_stack([X.ravel(), Z.ravel()])

    def centroid(self):
        return self.p[self.tris].mean(axis=1)


def stiffness(p, tris, coef, ndof=None, dofmap=None):
    """Assemble sum over triangles of coef * grad(phi_i).grad(phi_j) * area."""
    P = p[tris]
    d1 = P[:, 1] - P[:, 0]
    d2 = P[:, 2] - P[:, 0]
    area = 0.5 * np.abs(d1[:, 0] * d2[:, 1] - d1[:, 1] * d2[:, 0])
    # gradients of barycentric functions
    b = np.stack([P[:, 1, 1] - P[:, 2, 1], P[:, 2, 1] - P[:, 0, 1], P[:, 0, 1] - P[:, 1, 1]], 1)
    c = np.stack([P[:, 2, 0] - P[:, 1, 0], P[:, 0, 0] - P[:, 2, 0], P[:, 1, 0] - P[:, 0, 0]], 1)
    g = 1.0 / (4 * area)
    T = dofmap if dofmap is not None else tris
    rows, cols, vals = [], [], []
    for i in range(3):
        for j in range(3):
            rows.append(T[:, i])
            cols.append(T[:, j])
            vals.append(coef * g * (b[:, i] * b[:, j] + c[:, i] * c[:, j]))
    n = ndof if ndof is not None else len(p)
    return sp.csr_matrix(
        (np.concatenate(vals), (np.concatenate(rows), np.concatenate(cols))), shape=(n, n)
    )


def solve_fixed(K, fixed_idx, fixed_val):
    """Minimise u^T K u with u[fixed_idx] = fixed_val; return u and the energy."""
    n = K.shape[0]
    u = np.zeros(n)
    u[fixed_idx] = fixed_val
    free = np.setdiff1d(np.arange(n), fixed_idx)
    Kff = K[free][:, free]
    rhs = -K[free][:, fixed_idx] @ u[fixed_idx]
    u[free] = spla.spsolve(Kff.tocsc(), rhs)
    return u, float(u @ (K @ u))


def eps_field(cent, layers, er_air=1.0):
    """layers: list of (z_lo, z_hi, er) regions (x-independent); air elsewhere."""
    er = np.full(len(cent), er_air)
    for zlo, zhi, e in layers:
        m = (cent[:, 1] > zlo) & (cent[:, 1] < zhi)
        er[m] = e
    return er


# ---------------------------------------------------------------- microstrip


def microstrip_odd(W, g, h, t, layers, X=None, Z=None, fine=None, coarse=None, ratio=1.2):
    """Odd-mode C per unit length (F/m) of one strip of an edge-coupled pair,
    bracketed: returns (C_lo, C_up). Strip: x in [g/2, g/2 + W], z in [0, t];
    plane at z = -h; wall x = 0; layers: dielectric regions."""
    x1, x2 = g / 2, g / 2 + W
    X = X or (x2 + 40 * h)
    Z = Z or (40 * h)
    fine = fine or min(W, g, h, t) / 40
    coarse = coarse or h
    xc = (x1 + x2) / 2
    extra_z = [zz for lo, hi, _ in layers for zz in (lo, hi) if -h < zz < Z]
    xs = graded([0, x1, xc, x2, X], fine, coarse, ratio)
    zs = graded([-h, 0, t, Z] + extra_z, fine, coarse, ratio)
    m = Mesh(xs, zs, hole=(x1, x2, 0, t))
    cent = m.centroid()
    er = eps_field(cent, layers)
    p = m.p
    tol = 1e-12
    on_strip = (
        (p[:, 0] >= x1 - tol) & (p[:, 0] <= x2 + tol) & (p[:, 1] >= -tol) & (p[:, 1] <= t + tol)
    )
    wall = np.abs(p[:, 0]) < tol
    plane = np.abs(p[:, 1] + h) < tol
    box = (np.abs(p[:, 0] - X) < tol) | (np.abs(p[:, 1] - Z) < tol)
    used = np.zeros(m.n, bool)
    used[m.tris.ravel()] = True

    # primal: psi = 1 on strip, 0 on wall, plane, box
    K = stiffness(p, m.tris, er)
    gnd = np.where((wall | plane | box) & used)[0]
    one = np.where(on_strip & used)[0]
    unused = np.where(~used)[0]
    fixed = np.concatenate([gnd, one, unused])
    vals = np.concatenate([np.zeros(len(gnd)), np.ones(len(one)), np.zeros(len(unused))])
    _, E_up = solve_fixed(K, fixed, vals)
    C_up = EPS0 * E_up

    # dual: chi = 0 on the insulated box; jump Q = 1 across the cut x = xc,
    # -h <= z <= 0, for elements to its right.
    cut = (np.abs(p[:, 0] - xc) < tol) & (p[:, 1] <= tol) & (p[:, 1] >= -h - tol)
    cut_nodes = np.where(cut)[0]
    copy = {n_: m.n + k for k, n_ in enumerate(cut_nodes)}
    dm = m.tris.copy()
    right = (cent[:, 0] > xc) & (cent[:, 1] < 0)
    for e in np.where(right)[0]:
        for k in range(3):
            if dm[e, k] in copy:
                dm[e, k] = copy[dm[e, k]]
    ndof = m.n + len(cut_nodes)
    Kd = stiffness(p, m.tris, 1.0 / er, ndof=ndof, dofmap=dm)
    # eliminate copies: chi_copy = chi_orig + 1
    T = sp.lil_matrix((ndof, m.n))
    for i in range(m.n):
        T[i, i] = 1.0
    q = np.zeros(ndof)
    for n_, c_ in copy.items():
        T[c_, n_] = 1.0
        q[c_] = 1.0
    T = T.tocsr()
    A = (T.T @ Kd @ T).tocsr()
    bvec = T.T @ (Kd @ q)
    zero = np.where(box & used)[0]
    fixed = np.concatenate([zero, unused])
    free = np.setdiff1d(np.arange(m.n), fixed)
    chi = np.zeros(m.n)
    chi[free] = spla.spsolve(A[free][:, free].tocsc(), -bvec[free])
    full = T @ chi + q
    E_dual = float(full @ (Kd @ full))
    C_lo = EPS0 / E_dual
    return C_lo, C_up, m.n


# ----------------------------------------------------- stripline (exact check)


def stripline_odd(W, g, b, er, fine=None, coarse=None, ratio=1.2, X=None):
    """Zero-thickness coupled stripline, planes at +-b/2, odd mode: by the
    z-symmetry only the lower half is solved, the strip being the segment
    z = 0, x in [g/2, g/2 + W] of its top boundary, and the rest of z = 0
    insulating. Returns (C_lo, C_up) per unit length for the full strip."""
    h = b / 2
    x1, x2 = g / 2, g / 2 + W
    X = X or (x2 + 30 * b)
    fine = fine or min(W, g, h) / 60
    coarse = coarse or h / 2
    xs = graded([0, x1, x2, X], fine, coarse, ratio)
    zs = graded([-h, 0], fine, coarse, ratio)
    m = Mesh(xs, zs)
    p = m.p
    tol = 1e-12
    top = np.abs(p[:, 1]) < tol
    strip = top & (p[:, 0] >= x1 - tol) & (p[:, 0] <= x2 + tol)
    wall = np.abs(p[:, 0]) < tol
    plane = np.abs(p[:, 1] + h) < tol
    right = np.abs(p[:, 0] - X) < tol
    er_e = np.full(len(m.tris), er)
    K = stiffness(p, m.tris, er_e)
    gnd = np.where(wall | plane | right)[0]
    one = np.where(strip)[0]
    fixed = np.concatenate([gnd, one])
    vals = np.concatenate([np.zeros(len(gnd)), np.ones(len(one))])
    # grounded box: the right side; the top off-strip is the symmetry plane
    _, E_up = solve_fixed(K, fixed, vals)
    # dual: insulating pieces of the boundary are z = 0 off the strip and x = X.
    # Every node of an insulating piece is fixed, its end points included: a
    # free tip would let chi vary along the insulating edge beside it, so
    # D.n != 0 there and the trial would not be admissible.
    A_side = top & (p[:, 0] <= x1 + tol)  # from the wall to the strip
    B_side = (top & (p[:, 0] >= x2 - tol)) | right  # from the strip to the plane
    Kd = stiffness(p, m.tris, 1.0 / er_e)
    ia, ib = np.where(A_side)[0], np.where(B_side)[0]
    fixed = np.concatenate([ia, ib])
    vals = np.concatenate([np.zeros(len(ia)), np.ones(len(ib))])
    _, E_dual = solve_fixed(Kd, fixed, vals)
    # both halves
    return 2 * EPS0 / E_dual, 2 * EPS0 * E_up, m.n


def cohn_odd(W, g, b, er):
    """Cohn (1955), zero-thickness coupled stripline, exact:
    Z0o = 30 pi / sqrt(er) * K(k_o') / K(k_o),
    k_o = tanh(pi W / 2b) * coth(pi (W + g) / 2b). Returns (C per metre, Z)."""
    ko = np.tanh(np.pi * W / (2 * b)) / np.tanh(np.pi * (W + g) / (2 * b))
    kp = np.sqrt(1 - ko**2)
    Z = 30 * np.pi / np.sqrt(er) * ellipk(kp**2) / ellipk(ko**2)
    C = np.sqrt(er) / (C0 * Z)
    return C, Z


def z_interval(Cd_lo, Ca_lo, Cd_up, Ca_up):
    """Z_diff = 2 Z_odd, Z_odd = 1 / (c0 sqrt(C C_air)), from the bounds."""
    lo = 2 / (C0 * np.sqrt(Cd_up * Ca_up))
    hi = 2 / (C0 * np.sqrt(Cd_lo * Ca_lo))
    return lo, hi
