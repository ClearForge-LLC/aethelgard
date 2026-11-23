#!/usr/bin/env python3
"""AET-WO-4300j (1): the motor pull-down's bound and value, from held pages.

Run: python3 -I spike/4300j/derive.py   (no input; prints the derivation)

Every input below names its document, page and table, read from the PDF
page (rendered), never from a text extract. DERIVATION.md walks the steps.
"""

from fractions import Fraction as F

# ---- held inputs -----------------------------------------------------------
# The board's MCU rail: every committed motor board's `mcu.supply` is
# VDD_3V3, 3300 mV +/- 50 permille (measured in RESULTS.md, M4).
VDD_NOM_MV = 3300
VDD_TOL_PERMILLE = 50
# DS12117 Rev 10, Table 49 p. 137 (rev Y) = Table 146 p. 241 (rev V):
RPU_MAX = 50_000          # RPU, weak pull-up equivalent resistor, max, VIN = VSS
# DS12117 Rev 10, Table 49 p. 136 (rev Y) = Table 146 p. 241 (rev V),
# 1.62 V < VDD < 3.6 V, note 1 "compliant with CMOS requirements":
VIH_MIN_RATIO = F(7, 10)  # VIH min = 0.7 VDD
VIL_MAX_RATIO = F(3, 10)  # VIL max = 0.3 VDD
# DS12117 Rev 10, Table 49 p. 137 = Table 146 p. 241, Ilkg, 0 < VIN <= Max(VDDXXX):
# FT_xx and TT_xx +/-250 nA, FT_u +/-350 nA. The largest row is taken.
ILKG_MAX_NA = 350
# Vishay D/CRCW e3 (VISHAY_20035, 14-Apr-2026), p. 2, CRCW0603 e3, +/-100 ppm/K row:
# tolerance +/-1 %, E24; E96, 1 ohm to 10 Mohm. p. 2, "max. resistance change
# at P70 ... 8000 h <= 2 %" (stability class 1, held on the row as drift_ppm).
TOL = F(1, 100)
DRIFT = F(2, 100)

# ---- stated choices (decisions in STOP.md) ----------------------------------
# The ESC's input is not held (DS70005664A p. 7 Table 2-3 names the dsPIC pin,
# p. 16 Figure 3-3 draws no pull; no dsPIC datasheet is held). Stand-in: the
# ESC input is an STM32H753 input on the same rail: its VIH, VIL and Ilkg.
IIH_ESC_NA = ILKG_MAX_NA  # stand-in for AN4899's IIH (external device input current)
MARGIN_RATIO = F(1, 10)   # 10 % of VDD at each threshold

E96 = [100, 102, 105, 107, 110, 113, 115, 118, 121, 124, 127, 130, 133, 137,
       140, 143, 147, 150, 154, 158, 162, 165, 169, 174, 178, 182, 187, 191,
       196, 200, 205, 210, 215, 221, 226, 232, 237, 243, 249, 255, 261, 267,
       274, 280, 287, 294, 301, 309, 316, 324, 332, 340, 348, 357, 365, 374,
       383, 392, 402, 412, 422, 432, 442, 453, 464, 475, 487, 499, 511, 523,
       536, 549, 562, 576, 590, 604, 619, 634, 649, 665, 681, 698, 715, 732,
       750, 768, 787, 806, 825, 845, 866, 887, 909, 931, 953, 976]


def bound(vdd_mv, rpu=RPU_MAX, vih=VIH_MIN_RATIO, vil=VIL_MAX_RATIO,
          ilkg_na=ILKG_MAX_NA, iih_na=IIH_ESC_NA, margin=MARGIN_RATIO):
    """(floor, ceiling) in ohms, exact fractions, at one VDD."""
    vdd = F(vdd_mv, 1000)
    i = F(ilkg_na + iih_na, 10**9)
    # floor: idle line in passthrough, pull-up RPU to VDD, R_pd to GND,
    # leakage I leaving the node: (VDD - V)/RPU = V/R + I, V >= (vih + m) VDD
    v_hi = (vih + margin) * vdd
    floor = v_hi / ((vdd - v_hi) / rpu - i)
    # ceiling: reset, pin Hi-Z, leakage I into R_pd: I R <= (vil - m) VDD
    ceiling = (vil - margin) * vdd / i
    return floor, ceiling


def pick(floor, ceiling):
    """Smallest E96 value whose worst-case low (tolerance + drift) clears the
    floor and whose worst-case high stays under the ceiling."""
    for decade in (10**3, 10**4, 10**5, 10**6):
        for v in E96:
            r = v * decade // 100
            if r * (1 - TOL - DRIFT) >= floor and r * (1 + TOL + DRIFT) <= ceiling:
                return r
    return None


def main():
    vmin = VDD_NOM_MV * (1000 - VDD_TOL_PERMILLE) // 1000
    vmax = VDD_NOM_MV * (1000 + VDD_TOL_PERMILLE) // 1000
    print(f"VDD min {vmin} mV, max {vmax} mV (nominal {VDD_NOM_MV}, +/-{VDD_TOL_PERMILLE} permille)")
    rows = []
    for v in (vmin, VDD_NOM_MV, vmax):
        fl, ce = bound(v)
        rows.append((v, fl, ce))
        print(f"  at VDD {v} mV: floor {float(fl):10.1f} ohm   ceiling {float(ce):12.1f} ohm")
    floor = max(r[1] for r in rows)
    ceiling = min(r[2] for r in rows)
    print(f"BOUND over the rail: floor {float(floor):.1f} ohm (at VDD min), ceiling {float(ceiling):.1f} ohm (at VDD min)")
    r = pick(floor, ceiling)
    if r is None:
        print("NO E96 VALUE FITS: stop (WO §7)")
        return
    lo, hi = r * (1 - TOL - DRIFT), r * (1 + TOL + DRIFT)
    print(f"VALUE {r} ohm: worst case {float(lo):.0f} .. {float(hi):.0f} ohm")
    # the line at the value, the worst-case low part, VDD min
    vdd = F(vmin, 1000)
    i = F(ILKG_MAX_NA + IIH_ESC_NA, 10**9)
    v_idle = (vdd / RPU_MAX - i) / (1 / F(RPU_MAX) + 1 / lo)
    print(f"  passthrough idle at VDD min, R = {float(lo):.0f}: {float(v_idle)*1000:.1f} mV vs VIH min {float(VIH_MIN_RATIO*vdd)*1000:.1f} mV")
    print(f"  reset at VDD min, R = {float(hi):.0f}: {float(i*hi)*1000:.1f} mV vs VIL max {float(VIL_MAX_RATIO*vdd)*1000:.1f} mV")
    v402 = (vdd / RPU_MAX - i) / (1 / F(RPU_MAX) + 1 / F(40200))
    print(f"  4300i's 40200 ohm in passthrough at VDD min: {float(v402)*1000:.1f} mV, between VIL max {float(VIL_MAX_RATIO*vdd)*1000:.1f} and VIH min {float(VIH_MIN_RATIO*vdd)*1000:.1f} mV: neither level")
    print()
    print("sensitivity (not the bound): floor / ceiling at VDD min")
    for name, kw in [
        ("margin 0", dict(margin=F(0))),
        ("margin 5 % VDD", dict(margin=F(5, 100))),
        ("margin 10 % VDD (chosen)", {}),
        ("leakage 250 nA each end", dict(ilkg_na=250, iih_na=250)),
        ("ESC VIH 0.47 VDD + 0.25 V (note 2 row) ~ 0.546 VDD at 3.135 V", dict(vih=F(47, 100) + F(250, vmin))),
    ]:
        fl, ce = bound(vmin, **kw)
        print(f"  {name:62s} {float(fl):10.1f} {float(ce):12.1f}  -> E96 {pick(fl, ce)}")
    # cross-rail: FC pull-up at VDD min, ESC threshold at VDD max (not the bound)
    vfc, vesc = F(vmin, 1000), F(vmax, 1000)
    v_hi = (VIH_MIN_RATIO + MARGIN_RATIO) * vesc
    fl = v_hi / ((vfc - v_hi) / RPU_MAX - F(700, 10**9))
    print(f"  {'cross-rail: pull-up at VDD min, ESC VIH at VDD max':62s} {float(fl):10.1f}")


if __name__ == "__main__":
    main()
