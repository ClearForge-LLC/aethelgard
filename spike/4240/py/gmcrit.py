"""AN2867 Rev 10 §3.4 (DocID15287, pp. 13-14): gmcrit = 4*ESR*(2*pi*F)^2*(C0+CL)^2, must be < DS12117 Gm_crit_max 1.5 mA/V
(Table 33/130). §3.3 p. 13: CL = CL1*CL2/(CL1+CL2) + Cs, so CL1 = CL2 = 2*(CL - Cs). Figures from reads/crystals.md rows."""
from math import pi
GMAX = 1.5e-3
rows = [  # name, F MHz, ESR ohm, C0 pF (None = not stated), CL pF, AP-supported, cite, stock note
 ("ABM8AIG-8.000MHZ-1Z-T", 8, 500, 2.0, 18, True, "ABM8AIG 06-13-19 p1-2", "LCSC C5140594 8065"),
 ("ABM8AIG-8.000MHZ-10-1Z-T [CL 10 variant, constructed]", 8, 500, 2.0, 10, True, "same", "unknown"),
 ("ABM8AIG-8.000MHZ-8-1Z-T [CL 8 variant, constructed]", 8, 500, 2.0, 8, True, "same", "unknown"),
 ("ABM8AIG-16.000MHZ-12-2Z-T3 [constructed]", 16, 70, 2.0, 12, True, "ABM8AIG 06-13-19 p1 (ESR 70 Ω 16.000–19.999, C0 2.0), p2 (codes): builder re-read", "unknown (DigiKey 403)"),
 ("ABM8AIG-24.000MHZ-12-2Z-T3 [constructed]", 24, 50, 2.0, 12, True, "same", "unknown"),
 ("ABM8AIG-25.000MHZ-12-2Z-T3 [constructed]", 25, 50, 2.0, 12, True, "same", "unknown"),
 ("ABM8-24.000MHZ-D1G-T", 24, 50, 3.0, 18, True, "ABM8 07-29-20 p1", "LCSC C1986116 91"),
 ("ABM8-25.000MHZ-D2Y-T", 25, 50, 3.0, 18, True, "same", "LCSC C179643 776"),
 ("ABM8-12.000MHZ-D2Y-T (12 MHz: not an ArduPilot branch)", 12, 120, 3.0, 18, False, "same", "LCSC C6491123 1127"),
 ("ECS-160-12-33Q-JES (constructed)", 16, 80, 7.0, 12, True, "ECX-33Q Rev.2023 p1", "unknown"),
 ("ECS-80-18-33Q (8 MHz)", 8, 500, 7.0, 18, True, "same", "unknown"),
 ("ECS-160-8-33B2Q-VY (constructed)", 16, 70, 2.0, 8, True, "ECX-33B2Q Rev.2024 p1-2", "unknown"),
 ("NX3225GA-16MHZ-STD-CRG-2", 16, 80, None, 8, True, "NDK 2008E_NX3225GA_e (no C0)", "LCSC C156247 4690"),
 ("NX3225GA-24MHZ-STD-CRG-2", 24, 50, None, 8, True, "same", "LCSC C156248 5380"),
 ("NX3225GD-8MHZ-STD-CRA-3", 8, 500, None, 8, True, "NDK 2008E_NX3225GD_e (no C0)", "LCSC C889706 21485"),
 ("YXC X32258MSB4SI (YSX321SL)", 8, 180, 3.0, 20, True, "UNCITED (no doc no./rev)", "LCSC C2682774 92700"),
]
print("| Part | F MHz | ESR Ω | C0 pF | CL pF | gmcrit mA/V | ratio to 1.5 | verdict | C1=C2 at Cs 3 / 5 pF | cite | stock |")
print("|---|---|---|---|---|---|---|---|---|---|---|")
for n, f, esr, c0, cl, ap, cite, stock in rows:
    if c0 is None:
        g = "— (C0 not stated)"; ratio = "—"; v = "uncomputable"
        # bound shown for C0 <= 3 pF only as [inferred]
        gb = 4 * esr * (2 * pi * f * 1e6) ** 2 * ((3 + cl) * 1e-12) ** 2
        g += f"; {gb*1e3:.2f} if C0 ≤ 3 [inferred]"
    else:
        gg = 4 * esr * (2 * pi * f * 1e6) ** 2 * ((c0 + cl) * 1e-12) ** 2
        g = f"{gg*1e3:.3f}"; ratio = f"{GMAX/gg:.2f}×"; v = "PASS" if gg < GMAX else "**FAIL**"
    caps = f"{2*(cl-3):g} / {2*(cl-5):g} pF"
    print(f"| {n} | {f} | {esr} | {c0 if c0 is not None else 'n/s'} | {cl} | {g} | {ratio} | {v}{'' if ap else ' (not AP)'} | {caps} | {cite} | {stock} |")
