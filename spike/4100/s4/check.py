#!/usr/bin/env python3
"""S4: the anisotropic trial for an outer-layer piece over a plane at depth h < S, checked numerically.

Run: uv run -q --no-project --with numpy python spike/4100/s4/check.py <board.kicad_pcb> <extract.json>

The trial (FEEDBACK §S4): for a prism piece (convex base B: area A, perimeter p; height c; bottom
face at z = 0, the dielectric below), rho = d/S for z >= 0 (d: Euclidean distance to the piece) and
rho = sqrt((d_B/S)^2 + (z/h)^2) for z < 0 (d_B: planar distance to B); psi = max(0, 1 - rho).
Closed form of int |grad psi|^2 (times eps it bounds C):
  upper(A,p,c)   = c(pS + pi S^2)/S^2 + (A S + pi p S^2/4 + 2 pi S^3/3)/S^2      (today's, z >= 0)
  lower(A,p,h)   = A/h + pi p h/(8S) + pi p S/(8h) + 4 pi h/9 + 2 pi S^2/(9h)     (new, z < 0)
At h = S, lower == today's bottom end cap A/S + pi p/4 + 2 pi S/3.
"""
import json, math, re, sys
from fractions import Fraction as F

import numpy as np

S = 0.2            # mm, CLEARANCE_NM
W = 0.2            # mm, WIDTH_NM
C = 0.070          # mm, COPPER_NM
VIA_D = 0.6
EPS0 = 8.8541878202e-12 * 1e-3 * 1e15   # fF/mm (from above, as extract.rs)
EPSR = 5.4
EPS = EPS0 * EPSR                        # fF/mm
PI = math.pi
out = []

def say(s=""):
    print(s)
    out.append(s)

def upper(A, p, c, s=S):
    return c * (p * s + PI * s * s) / s**2 + (A * s + PI * p * s * s / 4 + 2 * PI * s**3 / 3) / s**2

def lower(A, p, h, s=S):
    return A / h + PI * p * h / (8 * s) + PI * p * s / (8 * h) + 4 * PI * h / 9 + 2 * PI * s * s / (9 * h)

def bottom_cap_today(A, p, s=S):
    return A / s + PI * p / 4 + 2 * PI * s / 3

# --- 1. Identity at h = S, exactly, with pi as a symbol: coefficients of (1, pi) -------------------
say("## 1. h = S reduces to today's bottom cap, coefficient by coefficient (exact rationals)")
for (A, p, s) in [(F(3), F(7), F(1, 5)), (F(1, 3), F(11, 2), F(2)), (F(5), F(1), F(1, 7))]:
    h = s
    new = (A / h, p * h / (8 * s) + p * s / (8 * h) + F(4, 9) * h + F(2, 9) * s * s / h)  # (rational, pi-coefficient)
    old = (A / s, p / 4 + F(2, 3) * s)
    say(f"  A={A} p={p} S={s}: lower(h=S) = {new[0]} + {new[1]}·pi ; today = {old[0]} + {old[1]}·pi ; equal: {new == old}")
# Per-length, the integer path: 280 fF/mm today, from (2cS + 2WS + pi S^2)/S^2.
def per_mm(h):
    return EPS * ((2 * C * S + W * S + PI * S * S / 2) / S**2 + W / h + PI * h / (4 * S) + PI * S / (4 * h))
say(f"  per-length at h = S: {per_mm(S):.4f} fF/mm (today's exact 279.30, rounded up 280)")

# --- 2. The closed form against direct quadrature of the same psi (3D finite differences) ----------
say("\n## 2. lower(A,p,h) against a 3D finite-difference quadrature of the same psi (box base a x b)")

def quad_lower(a, b, h, n):
    """Energy of psi in -h <= z <= 0 for a box base a x b: psi on nodes, squared differences per
    axis averaged over a cell's 4 parallel edges, times the cell's volume. Lateral spacing S/n,
    vertical spacing h/round(h n / S), so z = -h and z = 0 are grid planes."""
    g = S / n
    gz = h / max(1, round(h / g))
    xs = np.arange(-(a / 2 + S) - g, a / 2 + S + 1.5 * g, g)
    ys = np.arange(-(b / 2 + S) - g, b / 2 + S + 1.5 * g, g)
    zs = np.linspace(-h, 0.0, int(round(h / gz)) + 1)
    X, Y, Z = np.meshgrid(xs, ys, zs, indexing="ij")
    dB = np.hypot(np.maximum(np.abs(X) - a / 2, 0), np.maximum(np.abs(Y) - b / 2, 0))
    psi = np.maximum(0, 1 - np.sqrt((dB / S) ** 2 + (Z / h) ** 2))
    del X, Y, Z, dB
    sp = (g, g, gz)
    e = 0.0
    for ax in range(3):
        d2 = (np.diff(psi, axis=ax) / sp[ax]) ** 2
        o = [i for i in range(3) if i != ax]
        def cut(arr, i, lo):
            s = [slice(None)] * 3
            s[i] = slice(0, -1) if lo else slice(1, None)
            return arr[tuple(s)]
        cell = 0.25 * (cut(cut(d2, o[0], 1), o[1], 1) + cut(cut(d2, o[0], 0), o[1], 1)
                       + cut(cut(d2, o[0], 1), o[1], 0) + cut(cut(d2, o[0], 0), o[1], 0))
        e += cell.sum()
    return e * g * g * gz

for (a, b) in [(0.2, 1.0), (0.6, 0.5)]:
    A, p = a * b, 2 * (a + b)
    for h in [0.075, 0.1, 0.15]:
        cf = lower(A, p, h)
        qs = [quad_lower(a, b, h, n) for n in (20, 40, 80)]
        rich = 2 * qs[2] - qs[1]   # first-order Richardson: the kinks make the error O(spacing)
        say(f"  box {a}x{b} mm, h={h}: closed form {cf:.5f}; quadrature n=20/40/80 {qs[0]:.5f} {qs[1]:.5f} {qs[2]:.5f}; "
            f"extrapolated {rich:.5f} (rel. diff {abs(rich - cf) / cf:.1e})")

# --- 3. Exact case for the new cap: zero-thickness stripline, planes at h above and below ----------
say("\n## 3. Exact case: symmetric zero-thickness stripline (Cohn; C' = 4 eps K(k')/K(k), k = sech(pi W/(4h)))")

def K(k):
    a, b = 1.0, math.sqrt(1 - k * k)
    for _ in range(60):
        a, b = (a + b) / 2, math.sqrt(a * b)
    return PI / (2 * a)

def Kp(k):
    """K(k') with k' = sqrt(1 - k^2), from k itself: AGM(1, k)."""
    a, b = 1.0, k
    for _ in range(80):
        a, b = (a + b) / 2, math.sqrt(a * b)
    return PI / (2 * a)

def strip_exact(w, h):   # per mm, eps = 1; ground separation b = 2h
    x = PI * w / (4 * h)
    k = 2 * math.exp(-x) / (1 + math.exp(-2 * x))   # sech(x), no overflow
    return 4 * Kp(k) / K(k)

# The modulus checked against the wide-strip limit: C'/eps -> 2 W/h (two plates) plus a constant.
for w in (2.0, 4.0, 8.0):
    h = 0.1
    say(f"  modulus check W={w} h={h}: exact {strip_exact(w, h):.4f}, two plates 2W/h {2 * w / h:.4f}, "
        f"difference {strip_exact(w, h) - 2 * w / h:.4f} (wide-strip limit 8 ln2 / pi = {8 * math.log(2) / PI:.4f})")
say("  per-length, eps = 1, W = 0.2 mm (trial: elliptical caps both sides, lateral S = 0.2 mm, c = 0):")
for h in (0.05, 0.075, 0.1, 0.15, 0.2):
    trial = 2 * (W / h + PI * h / (4 * S) + PI * S / (4 * h))
    ex = strip_exact(W, h)
    say(f"    h={h}: exact {ex:.4f}, trial {trial:.4f}, trial/exact {trial / ex:.3f}, bound holds: {trial >= ex}")
for w in (0.1, 0.5, 1.0):
    h = 0.1
    trial = 2 * (w / h + PI * h / (4 * S) + PI * S / (4 * h))
    say(f"    W={w}, h={h}: exact {strip_exact(w, h):.4f}, trial {trial:.4f}, holds: {trial >= strip_exact(w, h)}")

# --- 4. Microstrip, the whole trial: bound >= P1 energy in a grounded box >= C(box) >= C(open) ------
say("\n## 4. Microstrip W x c at h over a plane, eps = 1, per mm: the trial against a P1 (5-point) energy")
say("  The discrete energy of any admissible grid function is >= C of the boxed problem >= C in the open;")
say("  so trial >= P1 energy shows trial >= truth. The inscribed wire (radius c/2) is a lower reference.")

def p1_energy(w, c, h, g, box_w=6.0, box_h=3.0, iters=6000):
    nx, ny = int(round(box_w / g)), int(round(box_h / g))
    u = np.zeros((nx + 1, ny + 1))
    fixed = np.zeros_like(u, dtype=bool)
    fixed[0, :] = fixed[-1, :] = fixed[:, 0] = fixed[:, -1] = True     # plane at y = 0, box elsewhere
    i0, i1 = int(round((box_w / 2 - w / 2) / g)), int(round((box_w / 2 + w / 2) / g))
    j0, j1 = int(round(h / g)), int(round((h + c) / g))
    assert abs(i1 - i0 - w / g) < 1e-6 and abs(j1 - j0 - c / g) < 1e-6 and abs(j0 - h / g) < 1e-6
    fixed[i0:i1 + 1, j0:j1 + 1] = True
    u[i0:i1 + 1, j0:j1 + 1] = 1.0
    free = ~fixed
    def lap(v):
        r = np.zeros_like(v)
        r[1:-1, 1:-1] = 4 * v[1:-1, 1:-1] - v[:-2, 1:-1] - v[2:, 1:-1] - v[1:-1, :-2] - v[1:-1, 2:]
        return r
    # CG on the free nodes: minimise sum of squared edge differences.
    r = -lap(u); r[~free] = 0
    pdir = r.copy(); rr = (r * r).sum()
    for _ in range(iters):
        Ap = lap(pdir); Ap[~free] = 0
        alpha = rr / (pdir * Ap).sum()
        u += alpha * pdir
        r -= alpha * Ap
        rr2 = (r * r).sum()
        if rr2 < 1e-24:
            break
        pdir = r + (rr2 / rr) * pdir; rr = rr2
    return (np.diff(u, axis=0) ** 2).sum() + (np.diff(u, axis=1) ** 2).sum()

for h in (0.075, 0.1, 0.15, 0.2):
    trial = 2 * C / S + W / S + PI / 2 + W / h + PI * h / (4 * S) + PI * S / (4 * h)
    e = p1_energy(W, C, h, 0.005)
    a = C / 2
    wire = 2 * PI / math.acosh((h + a) / a)
    say(f"    h={h}: trial {trial:.4f} >= P1-in-box {e:.4f} >= truth; inscribed wire {wire:.4f}; "
        f"trial/P1 {trial / e:.3f}; holds: {trial >= e}")

# --- 5. The reference's I2C lines at 4 layers, h swept -------------------------------------------
say("\n## 5. The reference's I2C lines if their outer copper lay over a plane at h (bound, fF)")
board, ext = open(sys.argv[1]).read(), json.load(open(sys.argv[2]))

def pads_of(net):
    boxes = []
    for fp in re.split(r"\n\t\(footprint ", board)[1:]:
        for pad in re.findall(r"\n\t\t\(pad .*?\n\t\t\)", fp, re.S):
            if f'(net "{net}")' in pad:
                sz = re.search(r"\(size ([\d.]+) ([\d.]+)\)", pad)
                boxes.append((float(sz.group(1)), float(sz.group(2))))
    return boxes

def via_ff():
    r, H = VIA_D / 2, 2.0
    A, p = PI * r * r, 2 * PI * r
    return EPS * (H * (p * S + PI * S * S) + 2 * A * S + PI / 2 * p * S * S + 4 / 3 * PI * S**3) / S**2

for line in ext["lines"]:
    net, t = line["board_net"], line["terms"]
    boxes = pads_of(net)
    L, nseg, nvia = line["length_nm"] / 1e6, line["segments"], line["via_count"]
    assert len(boxes) == line["pad_count"], (net, boxes)
    end_A, end_p = PI * W * W / 4, PI * W          # a track's two round ends: a disk of diameter W
    def total(h):
        hh = min(h, S)
        per = EPS * ((2 * C * S + W * S + PI * S * S / 2) / S**2 + W / hh + PI * hh / (4 * S) + PI * S / (4 * hh))
        ends = EPS * (upper(end_A, end_p, C) + lower(end_A, end_p, hh))
        pads = sum(EPS * (upper(a * b, 2 * (a + b), C) + lower(a * b, 2 * (a + b), hh)) for a, b in boxes)
        return per * L + ends * nseg, nvia * via_ff(), pads, t["pins"]
    tr0, v0, pd0, pins = total(S)
    say(f"  {net}: L={L} mm, {nseg} tracks, {nvia} vias, pads {boxes}; extract today: trace {t['trace']} "
        f"vias {t['vias']} pads {t['pads']} pins {t['pins']} = {line['c_bound_femtofarads']} (declared {line['declared_femtofarads']})")
    say(f"    float model at h >= S: trace {tr0:.0f} vias {v0:.0f} pads {pd0:.0f} (matches extract to rounding)")
    # 0.1797: OSH Park 2113 prepreg 0.1999 - 0.0202 (published tolerance); 0.0994: JLC04161H-3313 nominal (no tolerance published).
    for h in (0.2, 0.1797, 0.15, 0.12, 0.1, 0.0994, 0.09, 0.075, 0.05):
        tr, v, pd, pins = total(h)
        tot = tr + v + pd + pins
        base = tr0 + v0 + pd0 + pins
        say(f"    h={h:6}: per-length {per_mm(min(h, S)):.1f} fF/mm; trace {tr:7.0f} vias {v:6.0f} pads {pd:6.0f} pins {pins} -> {tot:7.0f} fF "
            f"({tot - base:+.0f} vs the float model at h >= S, {100 * (tot - base) / base:+.2f} %; margin to declared {line['declared_femtofarads'] - tot:.0f})")

open(sys.argv[3] if len(sys.argv) > 3 else "/dev/null", "w").write("\n".join(out) + "\n")
