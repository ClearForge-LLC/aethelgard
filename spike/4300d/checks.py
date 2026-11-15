"""AET-WO-4300d S3: the static checks, evaluated on the proposed tree (power-shape.json) with
guaranteed (min/max) datasheet values only. A typical value is never used as a bound; where only a
typical exists the check prints CONDITION. Every input names its source (held by sha256 in
SHA256SUMS; DS12117 Rev 10 sha256 607b9ead...c032). Integers are not needed for a probe: floats.
Usage: python3 -I checks.py > checks.out"""

# --- sources (page, table; read and spot-checked by rendering) -------------------------------
LMR = dict(  # TI SNVSAN3F (Nov 2020), LMR33630ADDA; limits over Tj -40..125 C at VIN = 12 V, VEN = 4 V
    vin_min_op_max=3.8,        # p7 7.5 "VIN minimum operating input voltage" MAX
    vin_rec_max=36.0,          # p6 7.3 recommended operating VIN 3.8..36 V
    vfb=(0.985, 1.015),        # p7 7.5 VFB min/max
    fsw_a=(340e3, 460e3),      # p8 7.5 "A" version min/max
    ton_min_max=108e-9,        # p8 7.6 tON-MIN DDA MAX
    toff_min_max=85e-9,        # p8 7.6 tOFF-MIN DDA MAX
    rds_hs_max=0.160,          # p8 7.5 DDA MAX
    rds_ls_max=0.110,          # p8 7.5 DDA MAX
    isc_max=5.05,              # p7 7.5 high-side current limit MAX (tested open loop, note 4)
    iout_rated=3.0,            # p6 7.3 recommended operating IOUT 0..3 A
    iq_max=34e-6,              # p7 7.5 IQ MAX (non-switching)
)
TLV18 = dict(  # TI SBVS320D (Sep 2024), TLV75518PDBV
    vout=(1.8 * 0.985, 1.8 * 1.015),  # p5 5.5 +-1.5 % full Tj (at IOUT 1 mA; load regulation typ only)
    vdo_max=0.400,             # p6 5.5 VDO, 500 mA, Tj <= 125 C, 1.8 <= VOUT < 2.5, DBV
    vin=(1.45, 5.5),           # p5 5.5
    iout_max=0.500,            # p4 5.3 recommended
    ignd_max=40e-6,            # p5 5.5 IGND MAX at 125 C
    rtheta_ja=231.1,           # p5 5.4 JEDEC DBV
)
CRCW = dict(tol=0.01, tcr=100e-6)  # Vishay 20035 (14-Apr-2026), CRCW0603 e3 1 %: +-100 ppm/K
T_SPAN = 65.0                      # K, 25 C to 85 C ambient max (DS12117 Table 111 suffix 6), a condition
DS = dict(  # DS12117 Rev 10, rev V section
    vdd=(1.62, 3.6),           # p209 Table 111
    bor3_fall_max=2.68,        # p212 Table 115 (equal to Table 16, the rev Y table power.rs cites)
    idd_max=0.658,             # p216 Table 120, Tj 125 C (1303 D4)
    rain_max_12b=1150.0,       # p273 Table 173, 12-bit, Tj 125 C
    vref_plus_max_is_vdda=True,  # p272 Table 173: VREF+ 1.62..VDDA; VAIN 0..VREF+ (p273)
)
XT30 = dict(rated_a=15.0)          # AMASS spec V1.2 XT30PW-M, "rated current 15A" (distributor copy)
ICM_VDDIO = (1.71, 1.95)           # TDK DS-000189 1.6 (held by 4210), via parts.json citation

# --- the declared tree (power-shape.json) -----------------------------------------------------
CELLS = {"2S": (2, 3.0, 4.2), "6S": (6, 3.0, 4.2), "6S-HV": (6, 3.0, 4.35)}
DOMAINS = {"VDD_5V": (5.0, 0.05), "VDD_3V3": (3.3, 0.05), "VDD_1V8": (1.8, 0.05)}
LOAD_5V = 2.0      # declared by the user (DS-009 ports), A
LOAD_1V8 = 0.020   # declared by the user, A
DCR_MAX = 0.030    # the inductor's DCR maximum: an inductor part row, NOT held here -> a condition


def rng(d):
    n, t = DOMAINS[d]
    return (n * (1 - t), n * (1 + t))


def divider_vout(top, bot, vfb):
    e = CRCW["tol"] + CRCW["tcr"] * T_SPAN
    lo = vfb[0] * (1 + top * (1 - e) / (bot * (1 + e)))
    hi = vfb[1] * (1 + top * (1 + e) / (bot * (1 - e)))
    return lo, hi


out = []
def check(name, ok, detail):
    out.append(f"{'PASS' if ok is True else 'FAIL' if ok is False else ok:9} {name}: {detail}")


# P3.1 input range
for k, (n, cmin, cmax) in CELLS.items():
    vmin, vmax = n * cmin, n * cmax
    check(f"input range {k}", vmin >= LMR["vin_min_op_max"] and vmax <= LMR["vin_rec_max"],
          f"battery {vmin:.2f}..{vmax:.2f} V against LMR33630 {LMR['vin_min_op_max']}..{LMR['vin_rec_max']} V")

# P3.3 VOUT inside the domain, at the source pad
v5 = divider_vout(100e3, 24.9e3, LMR["vfb"])
v33 = divider_vout(100e3, 43.2e3, LMR["vfb"])
for d, v in (("VDD_5V", v5), ("VDD_3V3", v33), ("VDD_1V8", TLV18["vout"])):
    lo, hi = rng(d)
    check(f"VOUT in {d}", v[0] >= lo and v[1] <= hi,
          f"{v[0]:.4f}..{v[1]:.4f} V inside {lo:.4f}..{hi:.4f}; margins {1e3*(v[0]-lo):.0f} / {1e3*(hi-v[1]):.0f} mV"
          + ("" if d != "VDD_1V8" else "; accuracy tested at 1 mA, load regulation typ only: CONDITION"))
check("VDD_1V8 inside ICM-20948 VDDIO", TLV18["vout"][0] >= ICM_VDDIO[0] and TLV18["vout"][1] <= ICM_VDDIO[1],
      f"{TLV18['vout'][0]:.4f}..{TLV18['vout'][1]:.4f} V inside {ICM_VDDIO}")

# P3.2 dropout: LDO from the 3.3 V regulator's guaranteed minimum
need = TLV18["vout"][1] + TLV18["vdo_max"]
check("LDO dropout (TLV75518 from VDD_3V3)", v33[0] >= need, f"VIN min {v33[0]:.4f} >= VOUT max + VDO max = {need:.4f} V")
# Buck headroom at 2S minimum from guaranteed fields only: D <= 1 - tOFFmin_max * fsw_max
dmax = 1 - LMR["toff_min_max"] * LMR["fsw_a"][1]
vin = 2 * 3.0
i = LOAD_5V
vavail = dmax * (vin - i * LMR["rds_hs_max"]) - (1 - dmax) * i * LMR["rds_ls_max"] - i * DCR_MAX
check("buck headroom VDD_5V at 2S min (computed bound, no foldback credit)", vavail >= v5[1],
      f"D<= {dmax:.4f}; available {vavail:.3f} V >= VOUT max {v5[1]:.3f} V (tOFF limit stated at VIN 12 V, DCR declared: CONDITION)")

# P3.7 minimum on-time at the highest VIN and frequency (losses only raise D, so this is conservative)
for d, v in (("VDD_5V", v5), ("VDD_3V3", v33)):
    vmax = 6 * 4.35
    ton = (v[0] / vmax) / LMR["fsw_a"][1]
    check(f"min on-time {d} at {vmax:.2f} V", ton >= LMR["ton_min_max"], f"tON {1e9*ton:.0f} ns >= {1e9*LMR['ton_min_max']:.0f} ns")

# IOUT per regulator: its domain's loads plus each LDO child's input (IOUT + IGND max)
i33 = DS["idd_max"] + LOAD_1V8 + TLV18["ignd_max"]
check("IOUT VDD_3V3 (LMR33630)", i33 <= LMR["iout_rated"], f"{i33:.4f} A <= {LMR['iout_rated']} A (MCU 658 mA + LDO input); other 3.3 V devices add their cited maxima")
check("IOUT VDD_5V (LMR33630)", LOAD_5V <= LMR["iout_rated"], f"{LOAD_5V} A <= {LMR['iout_rated']} A")
check("IOUT VDD_1V8 (TLV75518)", LOAD_1V8 <= TLV18["iout_max"], f"{LOAD_1V8} A <= {TLV18['iout_max']} A")
# Power-in: no efficiency is guaranteed, so a buck's average input current is bounded by its HS limit max
iin = 2 * LMR["isc_max"]
check("power-in connector (XT30PW-M)", iin <= XT30["rated_a"],
      f"sum of buck ISC max {iin:.2f} A <= {XT30['rated_a']} A (ISC tested open loop, TI note 4: CONDITION)")

# P3.4 the MCU supply: VOUT max under VDD max; the copper's room against the domain and against BOR3
check("VDD_3V3 max <= MCU VDD max", v33[1] <= DS["vdd"][1], f"{v33[1]:.4f} <= {DS['vdd'][1]} V")
room_domain = v33[0] - rng("VDD_3V3")[0]
room_bor = v33[0] - DS["bor3_fall_max"]
check("copper room, domain floor", "INFO",
      f"{1e3*room_domain:.0f} mV over 3.135 V -> VDD+VSS R <= {1e3*room_domain/DS['idd_max']:.0f} mOhm at 658 mA")
check("copper room, BOR3 floor", "INFO",
      f"{1e3*room_bor:.0f} mV over VBOR3 falling max 2.68 V -> VDD+VSS R <= {1e3*room_bor/DS['idd_max']:.0f} mOhm at 658 mA")

# P3.5 the sense divider, 100 k over 10 k, at worst tolerance and TCR
e = CRCW["tol"] + CRCW["tcr"] * T_SPAN
kmax = 10e3 * (1 + e) / (10e3 * (1 + e) + 100e3 * (1 - e))
for k, (n, _, cmax) in CELLS.items():
    v = n * cmax * kmax
    check(f"sense full scale {k}", v <= rng("VDD_3V3")[0], f"{v:.3f} V <= VREF+ = VDDA min {rng('VDD_3V3')[0]:.3f} V (VREF+ tied to VDDA on VDD_3V3: a declaration)")
rth = 100e3 * 10e3 / 110e3
check("sense source impedance vs RAIN", rth <= DS["rain_max_12b"],
      f"Thevenin {rth:.0f} Ohm vs RAIN max {DS['rain_max_12b']:.0f} Ohm (12-bit); needs a pin reservoir capacitor or a sampling time the firmware owns")

# thermal of the LDO (not in the WO's list; a decision)
pd = (rng("VDD_3V3")[1] - TLV18["vout"][0]) * LOAD_1V8
check("TLV75518 junction rise (inferred, not in S3's list)", "INFO", f"{1e3*pd:.1f} mW x {TLV18['rtheta_ja']} C/W = {pd*TLV18['rtheta_ja']:.1f} K")

print("\n".join(out))
