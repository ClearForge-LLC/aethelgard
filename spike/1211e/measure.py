#!/usr/bin/env python3
"""AET-WO-1211e measurement: every declared pull on a net that reaches an MCU
pin, its domain's maximum, the pin's I/O structure (DS12117 Rev 10 Table 8,
from table8.py's TSV) and that structure's tolerated input (Table 10 p. 103,
notes 3-5 p. 104); every reference-sets-trip part (the DRV8876) and its VREF;
every pin device's bulk need.

Usage: python3 -I measure.py [--table10|--table13] <table8.tsv> <crates/aethelgard-ir/data> <spec.json>...
Prints TSV rows; the last column is `ok` or `EXCEEDS`/`REFUSED`."""
import json, math, sys

RULE = sys.argv.pop(1) if sys.argv[1] in ("--table10", "--table13") else "--table13"

t8 = {}
for line in open(sys.argv[1]):
    name, pos, typ, struct, page, _ = line.rstrip("\n").split("\t")[:6]
    t8[name.split(" ")[0].split("-")[0]] = (struct, int(page))
data = sys.argv[2]
parts = json.load(open(f"{data}/parts.json"))
pins = {r["part"]: r for r in parts["pin_parts"]}
pp = {r["part"]: r for r in json.load(open(f"{data}/power-parts.json"))["parts"]}

def dmax(v): return math.ceil(v["nominal_millivolts"] * (1000 + v["tolerance_permille"]) / 1000)
def dmin(v): return v["nominal_millivolts"] * (1000 - min(v["tolerance_permille"], 1000)) // 1000


def tolerated(struct, vdd_min):
    """--table10 (the first run, as the WO wrote it): DS12117 Rev 10 Table 10
    (p. 103), FT_xxx min(min(VDD, VDDA, VDD33USB, VBAT) + 4.0 V, 6 V); TT_xx
    4.0 V; BOOT0 9.0 V; any other pin 4.0 V. --table13 (ruled D-A1): Table 13
    (p. 105), FT min(VDD + 3.6 V, < 5.5 V); TT VDD + 0.3 V. All supplies are on
    mcu.supply (the emitter's tie)."""
    if struct is None: return None
    if RULE == "--table13":
        if struct.startswith("FT"): return min(vdd_min + 3600, 5499)
        if struct.startswith("TT"): return vdd_min + 300
        return None
    if struct.startswith("FT"): return min(vdd_min + 4000, 6000)
    if struct.startswith("TT"): return 4000
    if struct == "B": return 9000
    return 4000

def row(f, kind, where, net, pin, to, vmax, vdd_min, extra=""):
    if RULE == "--table13" and to == MCU_SUPPLY[0]:
        print(f"{f}\t{kind}\t{where}\t{net}\t{pin or '(solver)'}\t-\t{to}\t{vmax}\t-\tok (same rail as mcu.supply)")
        return
    st = t8.get(pin)[0] if pin else None
    lim = tolerated(st, vdd_min) if st else None
    # an unpinned net may land on any GPIO: its worst is the lowest limit of
    # every I/O pin the table holds
    if pin is None:
        lim_any = min(x for s, _ in t8.values() if s.startswith(("FT", "TT")) for x in [tolerated(s, vdd_min)] if x is not None)
        verdict = "ok" if vmax <= lim_any else ("EXCEEDS-ANY" if vmax > min(vdd_min + 4000, 6000) else "EXCEEDS-TT")
        print(f"{f}\t{kind}\t{where}\t{net}\t(solver)\t-\t{to}\t{vmax}\t{lim_any}\t{verdict}{extra}")
    else:
        verdict = "ok" if lim is not None and vmax <= lim else ("NO-STRUCTURE" if lim is None else "EXCEEDS")
        print(f"{f}\t{kind}\t{where}\t{net}\t{pin}\t{st}\t{to}\t{vmax}\t{lim}\t{verdict}{extra}")

MCU_SUPPLY = [None]
print("file\tkind\twhere\tnet\tpin\tstructure\tto\tpull_max_mV\ttolerated_mV\tverdict")
for f in sys.argv[3:]:
    s = json.load(open(f))
    if not isinstance(s.get("voltage_domains"), list) or not isinstance(s.get("mcu"), dict):
        print(f"{f}\tSKIP\tnot a specification (a topology or data file)"); continue
    doms = {v["id"]: v for v in s["voltage_domains"]}
    mcu = doms.get(s["mcu"]["supply"])
    if mcu is None:
        print(f"{f}\tSKIP\tmcu.supply undeclared"); continue
    vdd_min = dmin(mcu)
    MCU_SUPPLY[:] = [s["mcu"]["supply"]]
    for i, b in enumerate(s.get("i2c_buses", [])):
        for line in ("sda", "scl"):
            d = doms.get(b["domain"])
            if d is None: continue
            row(f, "i2c", f"i2c_buses[{i}].{line}", b[line]["net"], b[line]["pin"], b["domain"], dmax(d), vdd_min)
    p = s.get("power") or {}
    for i, c in enumerate(p.get("crossings", []) or []):
        d = doms.get(c["a_domain"])
        bus = next((b for b in s.get("spi_buses", []) if b["id"] == c["bus"]), None)
        dev = next((d for d in s.get("spi_devices", []) if d["bus"] == c["bus"]), None)
        if d and dev:
            row(f, "crossing-cs", f"power.crossings[{i}].cs_pull_up", dev["cs"]["net"], dev["cs"]["pin"], c["a_domain"], dmax(d), vdd_min)
    for net, sense in (p.get("sense") or {}).items() if isinstance(p.get("sense"), dict) else []:
        pass
    for i, sns in enumerate(p.get("sense", []) or []) if isinstance(p.get("sense"), list) else []:
        print(f"{f}\tsense\tpower.sense[{i}]\t{sns['net']}\t(solver)\t-\tdivider\t-\t-\tjudged-by-step-16")
    for i, dv in enumerate(s.get("pin_devices", []) or []):
        for k, sg in enumerate(dv["signals"]):
            pl = sg.get("pull")
            if not pl: continue
            if pl.get("mcu") == "up":
                row(f, "pin-device-mcu-pull", f"pin_devices[{i}].signals[{k}]", sg["net"], sg["pin"], s["mcu"]["supply"], dmax(mcu), vdd_min)
            elif pl.get("resistor"):
                to = pl["resistor"]["to"]
                row(f, "pin-device-resistor", f"pin_devices[{i}].signals[{k}]", sg["net"], sg["pin"], to, dmax(doms[to]) if to in doms else 0, vdd_min)
        prow = pins.get(dv["part"])
        if dv["part"].startswith("DRV8876"):
            vref = dv["straps"].get("VREF")
            tie = vref.get("tie") if vref else None
            if tie == "GND":
                print(f"{f}\ttrip\tpin_devices[{i}].straps.VREF\tVREF\t-\t-\tGND\t0\t-\tREFUSED (chops at 0)")
            elif tie and not any(a["device_pin"] == "IPROPI" for a in dv["analog"]):
                print(f"{f}\ttrip\tpin_devices[{i}].analog\tIPROPI\t-\t-\t-\t-\t-\tREFUSED (sense unbound)")
            elif tie:
                vmin = dmin(doms[tie])
                for a in dv["analog"]:
                    if a["device_pin"] != "IPROPI": continue
                    fs = a["full_scale"]["milliamps"]
                    band = prow["pins"]["IPROPI"]["error_band"]
                    worst = max((fs * 1000 * (100 + b["percent"]) // 100) if "percent" in b else fs * 1000 + b["absolute_ua"]
                                for b in band if b["load_ma"][0] <= fs <= b["load_ma"][1])
                    out_ua = -(-worst * prow["pins"]["IPROPI"]["ua_per_a_typical"] // 1_000_000)
                    r = a["transfer"]["current_output"]["resistor"]
                    rr = pp[r["part"]]["kind"]["resistor"]
                    t = dv.get("temperature_celsius") or ((s.get("power") or {}).get("conditions") or {}).get("board_temperature_celsius")
                    if t is None:
                        print(f"{f}\ttrip\tpin_devices[{i}]\t-\t-\t-\t-\t-\t-\tNO-TEMPERATURE (an older shape: TemperatureUndeclared or a parse refusal today)"); continue
                    ex = max(t["max"] - rr["tcr_reference_celsius"], rr["tcr_reference_celsius"] - t["min"], 0)
                    ppm = rr["tolerance_ppm"]["value"] + rr["tcr_ppm_per_kelvin"]["value"] * ex + rr["drift_ppm"]["value"]
                    r_hi = -(-r["ohms"] * (1_000_000 + ppm) // 1_000_000)
                    v_uv = out_ua * r_hi
                    trip_ma = vmin * 1000 * 1000 // (prow["pins"]["IPROPI"]["ua_per_a_typical"] * r_hi) if r_hi else 0
                    verdict = "ok" if v_uv <= vmin * 1000 else "REFUSED (chops below full scale)"
                    print(f"{f}\ttrip\tpin_devices[{i}].analog.IPROPI\t{a['net']}\t-\t-\tVREF={tie} min {vmin} mV\tV_IPROPI at fs {v_uv} uV (R_hi {r_hi}, out {out_ua} uA)\ttrip at typ A ~{trip_ma} mA vs fs {fs} mA\t{verdict}")
            else:
                print(f"{f}\ttrip\tpin_devices[{i}].straps.VREF\tVREF\t-\t-\t{vref}\t-\t-\tNOT-A-TIE")
        if dv["part"].startswith("DRV8876"):
            print(f"{f}\tbulk\tpin_devices[{i}]\t{dv['reference']}\t-\t-\tVM={dv['supplies'].get('VM')}\t-\t-\t{'declared' if dv.get('bulk') else 'REFUSED (bulk required)'}")
        elif dv.get("bulk"):
            print(f"{f}\tbulk\tpin_devices[{i}]\t{dv['reference']}\t-\t-\t-\t-\t-\tREFUSED (bulk not allowed)")
