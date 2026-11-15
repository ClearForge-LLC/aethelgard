"""S3: microstrip_odd's dual (a cut and duplicated nodes, a separate code
path from stripline_odd's) under nested uniform refinement. A valid pair must
give C_lo <= C_up at every level, C_lo rising and C_up falling as the mesh
nests (each finer space contains the coarser). F1's bug showed the reverse."""
import sys
from bracket import microstrip_odd

ok = True
for W, g, h, t, er in [(0.2, 0.2, 0.2, 0.05, 4.0), (0.4, 0.2, 0.2, 0.05, 1.0)]:
    prev = None
    for f in (0.05, 0.025, 0.0125, 0.00625):
        lo, up, n = microstrip_odd(W, g, h, t, [(-h, 0, er)], X=g / 2 + W + 2.0, Z=2.0, fine=f, coarse=f, ratio=1.0)
        line = f"W {W} g {g} h {h} t {t} er {er} f {f}: C_lo {lo*1e12:9.4f} C_up {up*1e12:9.4f} pF/m ({n} nodes)"
        good = lo <= up and (prev is None or (lo >= prev[0] - 1e-18 and up <= prev[1] + 1e-18))
        ok &= good
        print(line, "ok" if good else "VIOLATES")
        prev = (lo, up)
print("NESTED OK" if ok else "NESTED FAILS")
sys.exit(0 if ok else 1)
