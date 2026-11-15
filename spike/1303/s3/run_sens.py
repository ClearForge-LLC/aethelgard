"""S3: which tolerance widens the interval. One input at a time at its two
extremes, the rest nominal (W 0.3, s 0.2, h 0.2, t 0.043, er 3.66, no mask);
Z is the bracket's midpoint (bracket width < 0.5 %)."""
from bracket import microstrip_odd, z_interval

def z(W=0.3, s=0.2, h=0.2, t=0.043, er=3.66, mask=0.0, er_mask=5.4):
    lay = [(-h, 0, er)] + ([(0, t + mask, er_mask)] if mask else [])
    d = microstrip_odd(W, s, h, t, lay, ratio=1.15); a = microstrip_odd(W, s, h, t, [], ratio=1.15)
    lo, hi = z_interval(d[0], a[0], d[1], a[1]); return (lo + hi) / 2

n = z(); print(f"nominal {n:.2f}")
for name, lo_kw, hi_kw in [("h 0.1797..0.2201", dict(h=0.1797), dict(h=0.2201)),
                           ("er 3.43..3.91", dict(er=3.91), dict(er=3.43)),
                           ("er 3.4..5.4", dict(er=5.4), dict(er=3.4)),
                           ("t 30..70 um", dict(t=0.070), dict(t=0.030)),
                           ("etch +-25 um", dict(W=0.325, s=0.175), dict(W=0.275, s=0.225)),
                           ("mask 0..50 um at er 5.4", dict(mask=0.05), dict()),
                           ("mask 0..25 um at er 4.0", dict(mask=0.025, er_mask=4.0), dict())]:
    a, b = z(**lo_kw), z(**hi_kw)
    print(f"{name:26}: {a:7.2f} .. {b:7.2f}  ({100*(a-n)/n:+.1f} % / {100*(b-n)/n:+.1f} %)")
