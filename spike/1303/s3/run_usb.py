"""S3 measurement: the certified Z_diff interval of an edge-coupled microstrip
pair over the 4-layer fixture's In1 plane, and the closed forms beside it.

Inputs (each labelled in FEEDBACK):
- h: OSH Park 2113 prepreg 7.87 mil +- 0.797 mil -> [0.1797, 0.2201] mm (read).
- er: max 5.4 (the fixture's declared IPC-4101 maximum, read); min: OSH Park
  gives Dk 3.61 at 1 GHz and Isola FR408HR 3.72 typical at 100 MHz, with no
  tolerance (read) -> no declared minimum exists; the probe takes 3.4 as a
  stated condition and also shows a +-5 % band around 3.61..3.72.
- t: outer copper 1 oz + plating, 1.7 mil (43 um) nominal, no tolerance (read);
  max 70 um (declared condition); min 30 um (condition).
- W, gap: etch +-0.025 mm about fixed centres (condition; no fabricator figure).
- mask: none for the lower C bound; for the upper, a layer at er 5.4 over
  everything at z in [0, t + 0.021]: OSH Park's solder resist is 0.6 mil
  +- 0.2 (read), so at most 0.0203 mm, rounded up; its permittivity maximum is
  a condition (5.4, the laminate's). A thinner or lower-permittivity mask
  only lowers C.
- A Z interval can sit inside 76.5..103.5 ohm only if hi/lo <= 103.5/76.5 =
  1.353, whatever W; the table prints hi/lo.
"""
import math
from bracket import microstrip_odd, z_interval

def bracket_z(W, g, h_lo, h_hi, t_lo, t_hi, er_lo, er_hi, etch=0.025, mask=0.021):
    # Highest C: W wide, gap narrow, h thin, t thick, er high, mask present.
    Wmax, gmin = W + etch, g - etch
    Wmin, gmax = W - etch, g + etch
    hiC = microstrip_odd(Wmax, gmin, h_lo, t_hi, [(-h_lo, 0, er_hi), (0, t_hi + mask, er_hi)], ratio=1.15)
    hiCa = microstrip_odd(Wmax, gmin, h_lo, t_hi, [], ratio=1.15)
    loC = microstrip_odd(Wmin, gmax, h_hi, t_lo, [(-h_hi, 0, er_lo)], ratio=1.15)
    loCa = microstrip_odd(Wmin, gmax, h_hi, t_lo, [], ratio=1.15)
    return z_interval(loC[0], loCa[0], hiC[1], hiCa[1])


def ipc2141(W, s, h, t, er):
    z0 = 87 / math.sqrt(er + 1.41) * math.log(5.98 * h / (0.8 * W + t))
    return 2 * z0 * (1 - 0.48 * math.exp(-0.96 * s / h))


print("== A. Fixed geometry (no tolerance): bracket vs IPC-2141 closed form")
print(" W    s    h     t     er  | Zdiff bracket (ohm)     | IPC-2141 | inside?")
for W, s, h, t, er in [(0.2, 0.2, 0.2, 0.043, 3.61), (0.3, 0.2, 0.2, 0.043, 3.61), (0.35, 0.15, 0.2, 0.043, 3.61),
                       (0.2, 0.2, 0.2, 0.043, 5.4), (0.1, 0.1, 0.2, 0.035, 4.0), (0.4, 0.4, 0.2, 0.035, 4.0)]:
    d = microstrip_odd(W, s, h, t, [(-h, 0, er)], ratio=1.15)
    a = microstrip_odd(W, s, h, t, [], ratio=1.15)
    lo, hi = z_interval(d[0], a[0], d[1], a[1])
    z = ipc2141(W, s, h, t, er)
    print(f" {W:.2f} {s:.2f} {h:.3f} {t:.3f} {er:4.2f} | [{lo:7.2f}, {hi:7.2f}] w {100*(hi-lo)/lo:.2f}% | {z:7.2f} | "
          f"{'yes' if lo <= z <= hi else 'NO, off by %.1f%%' % (100 * (z - (lo if z < lo else hi)) / (lo if z < lo else hi))}")

print("\n== B. Certified interval over the stackup's tolerances (s = 0.2 mm)")
print(" W     er range      | Zdiff interval (ohm) | centre | half-width | hi/lo (<= 1.353 needed) | inside 76.5..103.5?")
for W in (0.2, 0.3, 0.35, 0.4):
    for er_lo, er_hi, label in [(3.4, 5.4, "3.4..5.4"), (3.43, 3.91, "3.61..3.72 +-5%")]:
        lo, hi = bracket_z(W, 0.2, 0.1797, 0.2201, 0.030, 0.070, er_lo, er_hi)
        c = (lo + hi) / 2
        print(f" {W:.2f} {label:15} | [{lo:7.2f}, {hi:7.2f}] | {c:6.1f} | {100*(hi-lo)/2/c:5.1f} % | {hi/lo:.3f} | "
              f"{'yes' if lo >= 76.5 and hi <= 103.5 else 'no'}")

print("\n== B2. The same with the etch tolerance halved (+-12.5 um) and t from 35 um, to show what tighter declared inputs buy")
for W in (0.3, 0.35):
    lo, hi = bracket_z(W, 0.2, 0.1797, 0.2201, 0.035, 0.070, 3.43, 3.91, etch=0.0125)
    print(f" {W:.2f} 3.61..3.72 +-5% | [{lo:7.2f}, {hi:7.2f}] | hi/lo {hi/lo:.3f} | {'yes' if lo >= 76.5 and hi <= 103.5 else 'no'}")

print("\n== C. 1305's shell trial as the upper C (single line, h = 0.1797, W 0.2, t 0.07, er 5.4)")
# 4100 S4 per-length at h = 0.1797 mm: 285.1 fF/mm at er 5.4 (float model); air: / 5.4
Cs, Csa = 285.1e-12 / 1e-3 * 1e-3, 285.1e-12 / 5.4  # F/m
d = microstrip_odd(0.2, 40.0, 0.1797, 0.07, [(-0.1797, 0, 5.4)], ratio=1.15)
a = microstrip_odd(0.2, 40.0, 0.1797, 0.07, [], ratio=1.15)
c0 = 299792458.0
z_shell = 1 / (c0 * math.sqrt(Cs * Csa))
z_true = (1 / (c0 * math.sqrt(d[1] * a[1])), 1 / (c0 * math.sqrt(d[0] * a[0])))
print(f" C shell {Cs*1e12:.1f} pF/m vs bracket [{d[0]*1e12:.1f}, {d[1]*1e12:.1f}] (x{Cs/d[1]:.2f}); "
      f"C_air shell {Csa*1e12:.1f} vs [{a[0]*1e12:.1f}, {a[1]*1e12:.1f}] (x{Csa/a[1]:.2f})")
print(f" effective permittivity the bracket implies, C/C_air: {d[1]/a[1]:.2f}..{d[0]/a[0]:.2f}; the shell takes 5.4 everywhere, "
      f"a factor {5.4/(d[1]/a[1]):.2f}; times the air-case looseness {Csa/a[1]:.2f} = {5.4/(d[1]/a[1])*Csa/a[1]:.2f}")
print(f" Z0 lower bound from the shell {z_shell:.1f} ohm vs certified [{z_true[0]:.1f}, {z_true[1]:.1f}]: "
      f"{100*(1 - z_shell/z_true[0]):.1f} % below")
