"""AET-WO-4300d S3: the static checks, evaluated on the proposed tree (power-shape.json) with
guaranteed (min/max) datasheet values. A typical value is never used as a bound. A limit the
datasheet gives "by design, not production tested" is used and named CONDITION. Every input names
its source (held by sha256 in SHA256SUMS; DS12117 Rev 10 sha256 607b9ead...c032). Floats suffice
for a probe. Second version, after the adversarial pass (FEEDBACK A1-A3, A5, A10): the regulation
band, IFB, resistor temperature to 125 C, drift, two divider options, the steady-state input bound.
Usage: python3 -I checks.py > checks.out"""

# --- sources (page, table; read and spot-checked by rendering) -------------------------------
LMR = dict(  # TI SNVSAN3F (Nov 2020), LMR33630ADDA; limits over Tj -40..125 C at VIN = 12 V, VEN = 4 V
    vin_min_op_max=3.8,        # p7 7.5 "VIN minimum operating input voltage" MAX
    vin_rec_max=36.0,          # p6 7.3 recommended operating VIN 3.8..36 V
    vfb=(0.985, 1.015),        # p7 7.5 VFB min/max
    ifb_max=50e-9,             # p7 7.5 IFB (current into FB) MAX
    reg_band=(-0.015, 0.025),  # p9 7.7 VOUT regulation 0 A..max load, wrt VIN 12 V / 1 A: NOT production tested
    fsw_a=(340e3, 460e3),      # p8 7.5 "A" version min/max
    ton_min_max=108e-9,        # p8 7.6 tON-MIN DDA MAX
    toff_min_max=85e-9,        # p8 7.6 tOFF-MIN DDA MAX
    rds_hs_max=0.160,          # p8 7.5 DDA MAX
    rds_ls_max=0.110,          # p8 7.5 DDA MAX
    isc_max=5.05,              # p7 7.5 high-side current limit MAX (tested open loop, note 4)
    iout_rated=3.0,            # p6 7.3 recommended operating IOUT 0..3 A
)
TLV18 = dict(  # TI SBVS320D (Sep 2024), TLV75518PDBV
    vout=(1.8 * 0.985, 1.8 * 1.015),  # p5 5.5 +-1.5 % full Tj (at VIN 2.3 V, IOUT 1 mA; line/load regulation typ only)
    vdo_max=0.400,             # p6 5.5 VDO, 500 mA, Tj <= 125 C, 1.8 <= VOUT < 2.5, DBV
    iout_max=0.500,            # p4 5.3 recommended
    ignd_max=40e-6,            # p5 5.5 IGND MAX at 125 C (IOUT = 0)
    rtheta_ja=231.1,           # p5 5.4 JEDEC DBV
)
RES = {  # per-resistor worst error = tolerance + TCR x T_SPAN + drift
    "CRCW0603 1% 100ppm": dict(tol=0.01, tcr=100e-6, drift=0.01),    # Vishay 20035: 1000 h load-life limit 1 %
    "TNPW0603 0.1% 25ppm": dict(tol=0.001, tcr=25e-6, drift=0.001),  # Vishay 28758 p1, p2: 8000 h <= 0.10 % (general)
}
T_SPAN = 100.0     # K: 25 C to 125 C at the resistor (beside the buck), a declared condition
DS = dict(  # DS12117 Rev 10, rev V section
    vdd=(1.62, 3.6),           # p209 Table 111
    bor3_fall_max=2.68,        # p212 Table 115 (equal to Table 16, the rev Y table power.rs cites)
    idd_max=0.658,             # p216 Table 120, VOS0, Tj 125 C (1303 D4; Table 111 note 5 caps VOS0 at 105 C)
    rain_max={16: 170.0, 14: 435.0, 12: 1150.0, 10: 5650.0, 8: 26500.0},  # p273 Table 173, Tj 125 C
)
XT30 = dict(rated_a=15.0)          # AMASS spec V1.2 XT30PW-M, "rated current 15A" (distributor copy)
ICM_VDDIO = (1.71, 1.95)           # TDK DS-000189 1.6 (held by 4210), via parts.json citation

# --- the declared tree (power-shape.json) -----------------------------------------------------
CELLS = {"2S": (2, 3.0, 4.2), "6S": (6, 3.0, 4.2), "6S-HV": (6, 3.0, 4.35)}
DOMAINS = {"VDD_5V": (5.0, 0.05), "VDD_3V3": (3.3, 0.05), "VDD_1V8": (1.8, 0.05)}
LOAD_5V = 2.0      # declared by the user (DS-009 ports), A
LOAD_1V8 = 0.020   # declared by the user, A
DCR_MAX = 0.030    # the inductor's DCR maximum: an inductor part row, NOT held here -> a condition
DIVIDERS = {  # (top, bottom): TI Table 9-2's values, and exact-ratio E96 pairs
    "TI Table 9-2": {"VDD_5V": (100e3, 24.9e3), "VDD_3V3": (100e3, 43.2e3)},
    "exact E96": {"VDD_5V": (102e3, 25.5e3), "VDD_3V3": (107e3, 46.4e3)},
}


def rng(d):
    n, t = DOMAINS[d]
    return (n * (1 - t), n * (1 + t))


def buck_vout(top, bot, res):
    """VOUT range at the regulated point: VFB x divider at worst error, plus IFB x top, times the
    regulation band (a design limit, not production tested)."""
    r = RES[res]
    e = r["tol"] + r["tcr"] * T_SPAN + r["drift"]
    lo = LMR["vfb"][0] * (1 + top * (1 - e) / (bot * (1 + e)))
    hi = LMR["vfb"][1] * (1 + top * (1 + e) / (bot * (1 - e))) + LMR["ifb_max"] * top * (1 + e)
    return lo * (1 + LMR["reg_band"][0]), hi * (1 + LMR["reg_band"][1])


out = []
def check(name, ok, detail):
    out.append(f"{'PASS' if ok is True else 'FAIL' if ok is False else ok:9} {name}: {detail}")


# P3.1 input range
for k, (n, cmin, cmax) in CELLS.items():
    vmin, vmax = n * cmin, n * cmax
    check(f"input range {k}", vmin >= LMR["vin_min_op_max"] and vmax <= LMR["vin_rec_max"],
          f"battery {vmin:.2f}..{vmax:.2f} V against LMR33630 {LMR['vin_min_op_max']}..{LMR['vin_rec_max']} V")

# P3.3 VOUT inside the domain, at the regulated point; every divider option against every resistor
V = {}
for dname, pairs in DIVIDERS.items():
    for res in RES:
        for d, (top, bot) in pairs.items():
            v = buck_vout(top, bot, res)
            V[(dname, res, d)] = v
            lo, hi = rng(d)
            check(f"VOUT in {d} [{dname}, {res}]", v[0] >= lo and v[1] <= hi,
                  f"{v[0]:.4f}..{v[1]:.4f} V inside {lo:.4f}..{hi:.4f}; margins {1e3*(v[0]-lo):.0f} / {1e3*(hi-v[1]):.0f} mV"
                  " (regulation band -1.5/+2.5 % by design, not production tested: CONDITION)")
CHOSEN = ("exact E96", "TNPW0603 0.1% 25ppm")
v5, v33 = V[CHOSEN + ("VDD_5V",)], V[CHOSEN + ("VDD_3V3",)]
lo, hi = rng("VDD_1V8")
check("VOUT in VDD_1V8 (TLV75518)", TLV18["vout"][0] >= lo and TLV18["vout"][1] <= hi,
      f"{TLV18['vout'][0]:.4f}..{TLV18['vout'][1]:.4f} inside {lo:.4f}..{hi:.4f} (tested at 1 mA, VIN 2.3 V; line/load regulation typ only: CONDITION)")
check("VDD_1V8 inside ICM-20948 VDDIO", TLV18["vout"][0] >= ICM_VDDIO[0] and TLV18["vout"][1] <= ICM_VDDIO[1],
      f"{TLV18['vout'][0]:.4f}..{TLV18['vout'][1]:.4f} V inside {ICM_VDDIO}")

# P3.2 dropout: LDO from the 3.3 V regulator's minimum
need = TLV18["vout"][1] + TLV18["vdo_max"]
check("LDO dropout (TLV75518 from VDD_3V3)", v33[0] >= need, f"VIN min {v33[0]:.4f} >= VOUT max + VDO max = {need:.4f} V")
# Buck headroom at 2S minimum from guaranteed fields only: D <= 1 - tOFFmin_max * fsw_max
dmax = 1 - LMR["toff_min_max"] * LMR["fsw_a"][1]
for d, v, i in (("VDD_5V", v5, LOAD_5V), ("VDD_3V3", v33, DS["idd_max"] + LOAD_1V8)):
    vin = 2 * 3.0
    vavail = dmax * (vin - i * LMR["rds_hs_max"]) - (1 - dmax) * i * LMR["rds_ls_max"] - i * DCR_MAX
    check(f"buck headroom {d} at 2S min (no foldback credit)", vavail >= v[1],
          f"D<= {dmax:.4f}; available {vavail:.3f} V >= VOUT max {v[1]:.3f} V at {i:.2f} A"
          " (tOFF, RDS(on), fSW stated at VIN 12 V; DCR declared; input-path drop excluded: CONDITION)")
dcr_even = (dmax * (6.0 - LOAD_5V * LMR["rds_hs_max"]) - (1 - dmax) * LOAD_5V * LMR["rds_ls_max"] - v5[1]) / LOAD_5V
check("break-even inductor DCR, VDD_5V at 2S min, 2 A", "INFO", f"{1e3*dcr_even:.0f} mOhm")

# P3.7 minimum on-time at the highest VIN and frequency (losses only raise D, so this is conservative)
for d, v in (("VDD_5V", v5), ("VDD_3V3", v33)):
    vmax = 6 * 4.35
    ton = (v[0] / vmax) / LMR["fsw_a"][1]
    check(f"min on-time {d} at {vmax:.2f} V", ton >= LMR["ton_min_max"], f"tON {1e9*ton:.0f} ns >= {1e9*LMR['ton_min_max']:.0f} ns")

# IOUT per regulator: its domain's loads plus each child's input current. Steady state, a buck's
# average input current is D x IOUT <= IOUT (plus its bias, not guaranteed): A3.
i33 = DS["idd_max"] + LOAD_1V8 + TLV18["ignd_max"]
check("IOUT VDD_3V3 (LMR33630)", i33 <= LMR["iout_rated"], f"{i33:.4f} A <= {LMR['iout_rated']} A (MCU 658 mA + LDO input)")
check("IOUT VDD_5V (LMR33630)", LOAD_5V <= LMR["iout_rated"], f"{LOAD_5V} A <= {LMR['iout_rated']} A")
check("IOUT VDD_1V8 (TLV75518)", LOAD_1V8 <= TLV18["iout_max"], f"{LOAD_1V8} A <= {TLV18['iout_max']} A")
check("IOUT VDD_5V if the 3.3 V buck hung off it (steady state)", LOAD_5V + i33 <= LMR["iout_rated"],
      f"{LOAD_5V + i33:.3f} A <= {LMR['iout_rated']} A (buck bias excluded: CONDITION)")
# Power-in: steady state, the sum of the bucks' outputs; fault and start-up, their HS limits
iss = LOAD_5V + i33
check("power-in, steady state (XT30PW-M)", iss <= XT30["rated_a"], f"{iss:.2f} A <= {XT30['rated_a']} A (bias and Cin inrush excluded: CONDITION)")
iin = 2 * LMR["isc_max"]
check("power-in, fault bound (XT30PW-M)", iin <= XT30["rated_a"],
      f"sum of buck ISC max {iin:.2f} A <= {XT30['rated_a']} A (ISC tested open loop, TI note 4; overshoot past tON-MIN: CONDITION)")

# P3.4 the MCU supply: VOUT max under VDD max; the copper's room against the domain and against BOR3
check("VDD_3V3 max <= MCU VDD max", v33[1] <= DS["vdd"][1], f"{v33[1]:.4f} <= {DS['vdd'][1]} V")
for floor, name in ((rng("VDD_3V3")[0], "domain floor 3.135 V"), (3.0, "VDD33USB / TXU0304 3.0 V row"),
                    (DS["bor3_fall_max"], "VBOR3 falling max 2.68 V")):
    room = v33[0] - floor
    check(f"copper room, {name}", "INFO", f"{1e3*room:.0f} mV -> VDD+VSS R <= {1e3*room/DS['idd_max']:.0f} mOhm at 658 mA (MCU current only)")

# P3.5 the sense divider, 100 k over 10 k, at worst error (CRCW: tolerance, TCR, drift)
r = RES["CRCW0603 1% 100ppm"]
e = r["tol"] + r["tcr"] * T_SPAN + r["drift"]
kmax = 10e3 * (1 + e) / (10e3 * (1 + e) + 100e3 * (1 - e))
for k, (n, _, cmax) in CELLS.items():
    v = n * cmax * kmax
    check(f"sense full scale {k}", v <= rng("VDD_3V3")[0], f"{v:.3f} V <= VREF+ = VDDA min {rng('VDD_3V3')[0]:.3f} V (VREF+ tied to VDDA: a declaration)")
rth = 100e3 * 10e3 / 110e3
for bits, rain in sorted(DS["rain_max"].items(), reverse=True):
    check(f"sense source impedance vs RAIN at {bits} bits", rth <= rain, f"Thevenin {rth:.0f} Ohm vs RAIN max {rain:.0f} Ohm (no sampling-time condition in Table 173)")

# thermal of the LDO (not in the WO's list; a decision)
pd = (rng("VDD_3V3")[1] - TLV18["vout"][0]) * LOAD_1V8
check("TLV75518 junction rise (inferred, not in S3's list)", "INFO", f"{1e3*pd:.1f} mW x {TLV18['rtheta_ja']} C/W = {pd*TLV18['rtheta_ja']:.1f} K")

print("\n".join(out))
