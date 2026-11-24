# AET-WO-4300k: Sentinel-03's specification to the ruling (spec 30). Run from the repo root: python3 -I spike/4300k/build/s03_spec.py
import json, collections, sys
OD = collections.OrderedDict
P = "examples/sentinel-03.json"
s = json.loads(open(P).read(), object_pairs_hook=OD)
H = json.loads(open("crates/aethelgard-cli/tests/fixtures/hse-crystal-16.json").read(), object_pairs_hook=OD)
FUSE = sys.argv[1] if len(sys.argv) > 1 else "SF-1206F700-2"

def insert_after(d, after, key, value):
    out = OD()
    for k, v in d.items():
        out[k] = v
        if k == after:
            out[key] = value
    d.clear(); d.update(out)

# (6) the crystal, as hse-crystal-16 declares it (AN2867 Rev 25, M3)
m = s["mcu"]
new = OD()
for k, v in m.items():
    if k == "hse_hz":
        new["hse"] = H["mcu"]["hse"]
    else:
        new[k] = v
s["mcu"] = new
# (9) Path B: TLV803S on NRST
insert_after(s["mcu"], "reset_switch", "reset_supervisor", OD([
    ("reference", "U12"), ("part", "TLV803SDBZR"),
    ("pull_up", OD([("reference", "R61"), ("part", "CRCW0603_F_K"), ("ohms", 100000)])),
    ("bypass", OD([("reference", "C62"), ("part", "GCM188R71H104KA57")])),
]))
# 4410 D3: U5's orientation and the board's forward edge (ruled D9)
for d in s["spi_devices"]:
    if d["reference"] == "U5":
        d["orientation"] = OD([("x", "right"), ("z", "up")])
insert_after(s["board"], "copper_layers", "forward", "top")
p = s["power"]
# 4S (Scotty, 2026-11-23)
p["battery"]["chemistry"] = "LiPo, 3S to 4S, high-voltage cells allowed"
p["battery"]["cells"]["max"] = 4
# (10) the input's protection
p["input"]["protection"] = OD([
    ("fuse", OD([("reference", "F1"), ("part", FUSE)])),
    ("fet", OD([("reference", "Q1"), ("part", "DMP6023LSS-13")])),
    ("zener", OD([("reference", "D1"), ("part", "MMSZ5242BT1G")])),
    ("gate_resistor", OD([("reference", "R60"), ("part", "CRCW0603_F_K"), ("ohms", 100000)])),
    ("source", "AET-WO-4300k (10), ruled Q1 (a) 2026-11-24T10:00Z: the 7 A fuse, as the hot-plug I2t bound (fuse-only R) clears no smaller rating; 2x the input amended"),
])
# A6 (ruled D8): two input ceramics per buck; (a): the copper is measured, the loss keeps its current
extra = {"U10": "C60", "U11": "C61"}
loss_at = {"U10": 2000, "U11": 679}
for r in p["regulators"]:
    ext = []
    for e in r["external"]:
        ext.append(e)
        if e["role"] == "cin":
            ext.append(OD([("role", "cin"), ("reference", extra[r["reference"]]), ("part", e["part"])]))
    r["external"] = ext
    j = r["junction"]
    r["junction"] = OD([("loss_milliwatts", j["loss_milliwatts"]), ("loss_at_milliamps", loss_at[r["reference"]])])
# (9): the supervisor and its pull-up load VDD_3V3 (extract's SupplySinkUncounted)
p["loads"].extend([
    OD([("domain", "VDD_3V3"), ("milliamps", 1), ("source", "TI SBVS157E p. 6, 7.5: IDD at most 30 uA at VDD 6 V, output unconnected; rounded up"), ("reference", "U12")]),
    OD([("domain", "VDD_3V3"), ("milliamps", 1), ("source", "derived: 3.465 V (3.3 V + 5 %) / 100 kOhm = 0.035 mA, rounded up"), ("reference", "R61")]),
])
# (2), D6, D13: the MCU's supply limited, at most 110 mV
p["drop_limits"] = [OD([("domain", "VDD_3V3"), ("max_millivolts", 110),
    ("source", "AET-WO-4300k D6 (ruled): VDD_3V3's solved minimum less this stays above TLV803S's VIT- max plus its typical hysteresis, 3030 mV")])]
# (b) OSH Park, read 2026-11-24 (docs.oshpark.com/services/four-layer/, page sha256 cf14c792...)
st = s["board"]["stackup"]
OSH = "oshpark-four-layer"
fab = OD([("name", "OSH Park"), ("documents", [OD([
    ("id", OSH), ("title", "OSH Park 4 layer service specifications"),
    ("url", "https://docs.oshpark.com/services/four-layer/"), ("read", "2026-11-24")])])])
new = OD([("fabricator", fab)])
new.update(st); st.clear(); st.update(new)
def cite(d, c): d["cites"] = c
cite(st["finished_thickness_um"], OD([("fabricator", OSH)]))
for d in st["dielectrics"]:
    cite(d, OD([("fabricator", OSH)]))
cite(st["copper_max_um"], OD([("condition", "copper-max-2oz")]))
cite(st["relative_permittivity_max_permille"], OD([("condition", "permittivity-fr4-max")]))
st["relative_permittivity_max_permille"]["source"] = "the IPC-4101 FR-4 maximum, specified at 1 MHz; IPC-4101 is not held, so a condition (OSH Park states FR408HR 3.61 at 1 GHz, nominal)"
mins = [("outer_copper_min_um", 30, "outer-copper-min", "a declared condition: OSH Park states 1 oz outer copper, nominal, and no minimum"),
        ("plane_copper_min_um", 15, "plane-copper-min", "a declared condition: OSH Park states 1/2 oz inner copper, nominal, and no minimum"),
        ("plating_min_um", 18, "plating-min", "a declared condition: OSH Park states via plating 1 mil, nominal, and no minimum"),
        ("copper_temperature_max_c", 125, "copper-temperature-max", "a declared condition: no fabricator document states the copper's temperature limit")]
order = OD()
for k, v in st.items():
    order[k] = v
    if k == "relative_permittivity_max_permille":
        for key, val, cid, src in mins:
            order[key] = OD([("value", val), ("source", src), ("cites", OD([("condition", cid)]))])
st.clear(); st.update(order)
R = "OSH Park 4-layer, docs.oshpark.com/services/four-layer/, read 2026-11-24, Drill Specifications: "
st["fab_tolerances"] = OD([
    ("hole_position_um", OD([("value", 51), ("source", R + "Drill Positional Tolerance Max: 2mil (0.0508mm), rounded up"), ("cites", OD([("fabricator", OSH)]))])),
    ("hole_size_plus_um", OD([("value", 64), ("source", R + "Drill Size tolerance Max: +/- 2.5mil (0.0635mm), rounded up"), ("cites", OD([("fabricator", OSH)]))])),
    ("hole_size_minus_um", OD([("value", 64), ("source", R + "Drill Size tolerance Max: +/- 2.5mil (0.0635mm), rounded up"), ("cites", OD([("fabricator", OSH)]))])),
    ("track_width_permille", OD([("value", 200), ("source", "a declared condition: OSH Park publishes no track-width tolerance; 20 % is declared"), ("cites", OD([("condition", "track-width-tolerance")]))])),
    ("via_ring_um", OD([("value", 102), ("source", "OSH Park 4-layer, docs.oshpark.com/services/four-layer/, read 2026-11-24, Copper Specifications: Annular Ring 4mil (0.1016mm), rounded up"), ("cites", OD([("fabricator", OSH)]))])),
])
s["schema_version"] = 30
open(P, "w").write(json.dumps(s, indent=2, ensure_ascii=False) + "\n")
print("written", P, "fuse", FUSE)

# (1), (d): every condition acknowledged by its id, with why it is accepted on this board (ruled (d)).
LIFE = "no maker lifecycle page is held; its status is read at order time from the distributor's BOM line (FEEDBACK)"
ACK = {
 "buck_junction_screened:U10": "TI's Figure 9-3 is typical; U10's 1170 mW is screened at the copper extract measures on this board (>= 15 cm2 needed), and the bench reads its case temperature at 2 A",
 "buck_junction_screened:U11": "TI's Figure 9-3 is typical; U11's 550 mW is screened at the copper extract measures (>= 7 cm2 needed); the bench reads its case temperature",
 "buck_timing_conditions:U10": "tOFF-MIN, RDS(on) and fSW are stated at VIN 12 V; U10 runs 9.0 to 17.4 V, inside the 3S-4S range the headroom and on-time checks use with margin",
 "buck_timing_conditions:U11": "tOFF-MIN, RDS(on) and fSW are stated at VIN 12 V; U11 runs 9.0 to 17.4 V, inside the 3S-4S range the headroom and on-time checks use with margin",
 "bulk_esr_condition:C1": "C1's ESR is stated at 20 C and -40 C only; the board's -20 C floor sits between them, and the doubled-pack bound does not rely on its damping",
 "datasheet_advance_information:Q1": "DS37198 is marked advance information in its text layer; Q1's VDS, VGS and RDS(on) margins are large (60 V vs 34.8 V), re-read when a final sheet is held",
 "derating_declared:X5R+X7R+X7S": "no maker DC-bias curve is held for the output and VCC ceramics; TI's 50 % rule is applied, and the cin ceramics use Murata's own curve",
 "dielectric_accepted:C10": "C10 is X7S (GCM32EC71H106KA03), accepted where TI's example says X7R or better (ruled D-B8); its effective value at 4S is counted from its own curve",
 "dielectric_accepted:C20": "C20 is X7S (GCM32EC71H106KA03), accepted where TI's example says X7R or better (ruled D-B8); its effective value at 4S is counted from its own curve",
 "dielectric_accepted:C60": "C60, U10's second cin (A6, ruled D8), is X7S like C10, accepted under ruling D-B8; the two give 10.87 uF effective at 17.4 V",
 "dielectric_accepted:C61": "C61, U11's second cin (A6, ruled D8), is X7S like C20, accepted under ruling D-B8; the two give 10.87 uF effective at 17.4 V",
 "drop_not_judged:VDD_5V": "VDD_5V feeds only J6.1 and J7.1 off-board (DS-009 ports); no on-board load pin exists for the supply gate (ruled D13)",
 "effective_no_minimum:U10.cout": "U10's output ceramics have no effective minimum in TI's row; their 44 uF is a placeholder derating until Murata's curve for GRM32ER61C226KE20 is held",
 "effective_no_minimum:U11.cout": "U11's output ceramics have no effective minimum in TI's row; their 44 uF is a placeholder derating until Murata's curve for GRM32ER71A226KE20 is held",
 "effective_typical:U10.cin": "Murata's DC-bias chart is typical at 25 C; U10's two cin are counted worst case (curve, -10 % tolerance, -22 % X7S) and clear 10 uF by 8.7 %",
 "effective_typical:U11.cin": "Murata's DC-bias chart is typical at 25 C; U11's two cin are counted worst case (curve, -10 % tolerance, -22 % X7S) and clear 10 uF by 8.7 %",
 "fuse_hot_plug_bound_fuse_only:F1": "F1's I2t bound counts only its own minimum resistance (no FET, copper, connector or pack minimum is held), so 7 A was ruled (Q1 (a)); holding one is a row",
 "hot_plug_doubled_pack:U10": "the ring at U10's VIN is bounded by twice the 17.4 V pack, a derivation (ruled D7), under 38 V; C1's damping only lowers it",
 "hot_plug_doubled_pack:U11": "the ring at U11's VIN is bounded by twice the 17.4 V pack, a derivation (ruled D7), under 38 V; C1's damping only lowers it",
 "hse_drive_level_unmeasured:HSE": "AN2867 Rev 25 Step 4 needs a measured Vpp at OSC_IN; Y1's drive level is a bench row before the order",
 "hse_safety_factor_unmeasured:HSE": "AN2867 Rev 25 Step 3's negative-resistance safety factor needs the board; gmcrit 0.555 mA/V is under the 1.5 mA/V limit; a bench row",
 "inductor_dcr_at_25c:U10": "L1's DCR is stated at 25 C and not raised for 85 C; the headroom check counts it, with the input path's 70 mV, and holds at 9.0 V",
 "inductor_dcr_at_25c:U11": "L2's DCR is stated at 25 C and not raised for 85 C; the headroom check counts it, with the input path's 70 mV, and holds at 9.0 V",
 "inductor_ripple_outside_guidance:U10": "8.2 uH meets Equation 5 but not Equation 4's 20-40 % ripple over the whole 3S-4S range (ruled D6); the peak current stays under the high-side limit",
 "inductor_ripple_outside_guidance:U11": "8.2 uH meets Equation 5 but not Equation 4's 20-40 % ripple over the whole 3S-4S range (ruled D6); the peak current stays under the high-side limit",
 "inrush_bulk_at_rated:C1": "C1 enters the hot-plug I2t at its rated 47 uF (no tolerance in its row); at +20 % the bound is 2.72 A2s, still under F1's 3.25 A2s",
 "isat_typical:L1": "IHLP-5050FD's 16 A saturation is typical only; it is 3.2 times U10's 5.05 A high-side limit maximum (ruled D-B3)",
 "isat_typical:L2": "IHLP-5050FD's 16 A saturation is typical only; it is 3.2 times U11's 5.05 A high-side limit maximum (ruled D-B3)",
 "loss_current_gap:U11": "U11's loss is read at 679 mA and its solved load is 749 mA; the 70 mA gap adds little loss at 81 % efficiency, and refusing it needs a ruling (4300d3 F9)",
 "mcu_load_excludes:STM32H753ZITx": "Table 120's run maximum excludes I/O source currents, VDDA and VDD33USB; USB is unused, and each I/O load is a declared power.loads row",
 "offboard_declared:J8.2": "J8's EXT_SCL off-board 20 pF is declared for a 100 mm JST cable and one remote pin; a longer cable or second device is outside the bound",
 "offboard_declared:J8.3": "J8's EXT_SDA off-board 20 pF is declared for a 100 mm JST cable and one remote pin; a longer cable or second device is outside the bound",
 "pin_capacitance_declared:U5": "TDK prints C_I < 10 pF typical only for the ICM-42688-P; U5's pins are counted at 10 pF, and IMU_SCK keeps 8.5 pF margin to its bound",
 "pin_capacitance_declared:U7": "Bosch prints no pin capacitance for the BMP581; U7 is counted at UM10204's 10 pF limit for an I2C pin",
 "plane_not_joined:U10.In2.Cu": "In2.Cu is the VDD_3V3 plane, which U10's GND pad cannot join; the screen counts only F.Cu, In1.Cu and B.Cu",
 "plane_not_joined:U11.In2.Cu": "In2.Cu is the VDD_3V3 plane, which U11's GND pad cannot join; the screen counts only F.Cu, In1.Cu and B.Cu",
 "power_in_rating_underated:J4": "J4 is rated 15 A by AMASS V1.2, with no derating held for 85 C; the branch draws 1.64 A, and no motor current crosses the board",
 "regulation_not_production_tested:U10": "TI states VOUT regulation as a design limit, not production tested; U10's 5 V feeds only the DS-009 ports J6 and J7, off the MCU's rail",
 "regulation_not_production_tested:U11": "TI states VOUT regulation as a design limit, not production tested; VDD_3V3's floor is held by its feedback, its drop limit and U12",
 "reservoir_cadc_typical:VBAT_SENSE": "CADC is typical 4 pF; C50's 100 nF is 3.05 times the 32.8 nF the single-conversion reservoir rule needs (AN2834 p. 45-46, ruled D-B3)",
 "stackup_condition:copper-max-2oz": "OSH Park states 1 oz outer and 1/2 oz inner copper, nominal; 70 um (2 oz) is the engine's stated upper bound for every layer",
 "stackup_condition:copper-temperature-max": "no fabricator page states the copper's temperature limit; 125 C is the declared ceiling the supply gate's resistivity uses",
 "stackup_condition:outer-copper-min": "OSH Park states 1 oz outer copper, nominal (35 um); 30 um is the declared minimum the supply gate's resistance uses",
 "stackup_condition:permittivity-fr4-max": "IPC-4101's FR-4 maximum is not held; OSH Park's FR408HR is 3.61 nominal, and 5.4 is the declared upper bound for the capacitance bound",
 "stackup_condition:plane-copper-min": "OSH Park states 1/2 oz inner copper, nominal (17.5 um); 15 um is the declared minimum for the GND and VDD_3V3 planes",
 "stackup_condition:plating-min": "OSH Park states via plating 1 mil, nominal (25 um); 18 um is the declared minimum the supply gate's via barrels use",
 "stackup_condition:track-width-tolerance": "OSH Park publishes no track-width tolerance; 20 % is declared for the supply gate's tracks, as JLCPCB states for its own process",
 "supervisor_hysteresis_typical:U12": "TLV803S's 40 mV hysteresis is typical only; its release at 3.03 V is 4 mV under VDD_3V3's 3034 mV floor at its pin (ruled D6); a bench row",
 "usb_pair_undeclared:PA11": "PA11/PA12 are reserved for USB (AET-WO-3400 F5) with no connector drawn on Sentinel-03, so no differential pair exists to judge",
 "vref_is_vdda:VBAT_SENSE": "Sentinel-03 has no separate VREF+; VDDA is VDD_3V3, and the sense's full scale is judged against VDD_3V3's minimum",
 "rated_only:U10.cboot": "TI's row states no effective minimum for U10's 0.1 uF boot capacitor; GCM188R71H104KA57 at 50 V rating sees 5 V across it",
 "rated_only:U10.cin_hf": "TI's row states no effective minimum for U10's 0.22 uF HF input capacitor; its bulk cin pair carries the 10 uF effective minimum",
 "rated_only:U10.cvcc": "TI's row states no effective minimum for U10's 1 uF VCC capacitor; GRM188R71C105KA12 at 16 V rating sees VCC, nominally 5 V and above 5.5 V in some conditions (SNVSAN3F pp. 6, 23)",
 "rated_only:U11.cboot": "TI's row states no effective minimum for U11's 0.1 uF boot capacitor; GCM188R71H104KA57 at 50 V rating sees 5 V across it",
 "rated_only:U11.cin_hf": "TI's row states no effective minimum for U11's 0.22 uF HF input capacitor; its bulk cin pair carries the 10 uF effective minimum",
 "rated_only:U11.cvcc": "TI's row states no effective minimum for U11's 1 uF VCC capacitor; GRM188R71C105KA12 at 16 V rating sees VCC, nominally 5 V and above 5.5 V in some conditions (SNVSAN3F pp. 6, 23)",
 "zener_below_test_current:D1": "D1's VZ is stated at 20 mA; through R60 it carries about 60 uA at 17.4 V, where VZ is lower, so Q1's gate stays under 12.6 V of its 20 V",
}
for part in ["CRCW0603_F_K","DMP6023LSS-13","GCM188R71H104KA57","GCM188R71H224KA64","GCM32EC71H106KA03","GRM188R71C105KA12",
             "GRM32ER61C226KE20","GRM32ER71A226KE20","IHLP5050FDER8R2M01","MMSZ5242BT1G","SF-1206F700-2","TLV803SDBZR","TNPW0603_B_E","UCZ1H470MCL1GS"]:
    ACK["lifecycle_unread:" + part] = part + ": " + LIFE
for k, v in ACK.items():
    assert 1 <= len(v.encode()) <= 200 and v.isascii() and v == v.strip(), (k, len(v.encode()))
s = json.loads(open(P).read(), object_pairs_hook=OD)
s["acknowledge_conditions"] = [OD([("id", k), ("reason", ACK[k])]) for k in sorted(ACK)]
open(P, "w").write(json.dumps(s, indent=2, ensure_ascii=False) + "\n")
print("acknowledged", len(ACK))

# D10 (ruled): the ledger's 46 register/class entries, each reason specific to this board
# (drafted from the ledger and the topology, sources in acks-unwitnessed-notes.md), and the four held
# reasons the crystal and the new pins made false, rewritten.
s = json.loads(open(P).read(), object_pairs_hook=OD)
held = s["acknowledge_unwitnessed"]
FIX = {
    ("USART1.CR1", "ignored-by-model"): "Not stored by Renode's USART (4300b FEEDBACK section 0; M0, M1 measured P4.1): console USART1 on PA9/PA10, 8 data bits, FIFOEN set, no wake-up, interrupts off. The bench row reads CR1 back.",
    ("GPIOB.OTYPER", "ignored-by-model"): "OTYPER is not stored by Renode (4300b FEEDBACK section 10): PB6/PB7 (I2C1) and PB10/PB11 (I2C2, EXT) open-drain, the rest push-pull. 3302 B2 reads it on silicon; the bench row compares it.",
    ("RCC.CR", "ignored-by-model"): "D2CKRDY: not polled (Q-F), a tag reading 0 (STM32H7_RCC.cs L175). The rest: tags or read-only flags of the RCC_CR words the HSE bring-up writes. Bench rows: HSIKERON, CSIKERON, HSEBYP, D2CKRDY.",
    ("RCC.D1CCIPR", "ignored-by-model"): "A tag in Renode (STM32H7_RCC.cs L401, reads 0 whatever is written, 4230 FEEDBACK section 2). SPI1 and SPI2 run on pll1_q here (the topology), not per_ck. The bench row reads it.",
}
for a in held:
    k = (a["register"], a["class"])
    if k in FIX:
        a["reason"] = FIX[k]
new = json.loads(open("spike/4300k/build/acks-unwitnessed.json").read(), object_pairs_hook=OD)
index = {(a["register"], a["class"]): i for i, a in enumerate(held)}
for e in new:
    k = (e["register"], e["class"])
    entry = OD([("register", e["register"]), ("class", e["class"]), ("fields", e["fields"]), ("reason", e["reason"])])
    if k in index:
        held[index[k]] = entry
    else:
        held.append(entry)
for a in held:
    r = a["reason"]
    assert 1 <= len(r.encode()) <= 200 and r.isascii() and r == r.strip(), (a["register"], len(r.encode()))
s["acknowledge_unwitnessed"] = [a for a in held if a["register"] != "FLASH.ACR"]
open(P, "w").write(json.dumps(s, indent=2, ensure_ascii=False) + "\n")
print("unwitnessed entries", len(s["acknowledge_unwitnessed"]))
