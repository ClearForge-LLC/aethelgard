"""S3 exact check: the bracket against Cohn's exact zero-thickness coupled
stripline, odd mode (homogeneous dielectric), at two mesh levels."""
import sys
from bracket import stripline_odd, cohn_odd

print("W/b g/b er level | C_exact pF/m | C_lo  C_up | lo/exact up/exact | nodes")
ok = True
for W, g, b, er in [(0.2, 0.2, 0.4, 4.0), (0.1, 0.3, 0.4, 1.0), (0.5, 0.1, 0.4, 4.4), (0.3, 0.5, 1.0, 3.0)]:
    ex, Z = cohn_odd(W, g, b, er)
    for level, kw in [("coarse", {}), ("fine", dict(fine=min(W, g, b / 2) / 400, ratio=1.1))]:
        lo, up, n = stripline_odd(W, g, b, er, **kw)
        held = lo <= ex <= up
        ok &= held
        print(f"{W/b:.2f} {g/b:.2f} {er:.1f} {level:6} | {ex*1e12:9.3f} | {lo*1e12:9.3f} {up*1e12:9.3f} | "
              f"{lo/ex:.5f} {up/ex:.5f} | {n} {'holds' if held else 'FAILS'}")
print("ALL HOLD" if ok else "A CASE FAILS")
sys.exit(0 if ok else 1)
