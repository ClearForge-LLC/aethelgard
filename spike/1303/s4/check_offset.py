"""S4: sensitivity of the via entry to a drilled hole off its pad's centre.
JLCPCB's hole position tolerance is +-0.075 mm (read, capabilities page). With
the hole centre c' anywhere within delta of the pad centre c, a cut line kept
within r_p - delta of c has rays to c' inside the pad disc (convexity), so the
fan stays in copper. The fan and the barrel spread are evaluated at c' on a
circle of radius delta (64 directions) and compared with the concentric case.
This is a sensitivity, not a proof of the maximum over the disc."""
import json, math
from rbound2 import fan_line, spread_line
c = json.load(open("conditions.json"))
rd, rp, w, t = 0.15, 0.3, 0.2, c["t_outer_min_mm"]
delta = 0.075
nominal = fan_line(c, w, t, rp, 0.0, rd) + spread_line(c, w, rp, 0.0, rd, 0.3)
print(f"via end (fan + spread), concentric, cut at x_e = 0.3: {nominal*1e3:.3f} mOhm")
for delta in (0.025, 0.05, 0.075):
    best = None
    # the cut must keep its ends within r_p - delta of the pad centre and
    # clear the hole (radius r_d about c') for every c' within delta
    for xe in [0.15 + k * 0.0025 for k in range(1, 61)]:
        if math.hypot(xe, w / 2) > rp - delta + 1e-12:
            continue
        worst, okc = 0.0, True
        for k in range(64):
            th = 2 * math.pi * k / 64
            p, s0 = xe + delta * math.cos(th), delta * math.sin(th)
            dmin = p if abs(s0) <= w / 2 else math.hypot(p, abs(s0) - w / 2)
            if p <= 0 or dmin <= 1.05 * rd:
                okc = False
                break
            worst = max(worst, fan_line(c, w, t, p, s0, rd) + spread_line(c, w, p, s0, rd, 0.3))
        if okc and (best is None or worst < best[0]):
            best = (worst, xe)
    if best is None:
        print(f"hole off by {delta} mm: NO admissible cut of width {w} mm (annular ring {rp - rd} mm)")
    else:
        print(f"hole off by {delta} mm: worst of 64 directions {best[0]*1e3:.3f} mOhm at x_e {best[1]:.4f} "
              f"(x{best[0]/nominal:.3f} the concentric value)")
