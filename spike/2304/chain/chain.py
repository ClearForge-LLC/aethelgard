#!/usr/bin/env python3
"""AET-WO-2304 §1.4: the battery chain under the motors, statically (no Renode, no ngspice).

Run with `python3 -I spike/2304/chain/chain.py <mode> ...` from the repository root.

What is MIRRORED from aethelgard-ir (read at source, cd43b207), integer for integer:
- `battery_range` (power.rs L609): cells.min x cell_min_millivolts.
- the buck's output range (power.rs ~L1036-1068): VOUT = VFB (1 + RT/RB) +- IFB RT with each
  resistor's worst error (`resistor_error_ppm`, L531: tolerance + TCR x excursion + drift),
  then the regulation band.
- each regulator's load (`loads`, L1340-): the declared loads, the MCU's IDD max on its supply,
  and a child LDO's load plus its ground current.
- the buck's duty-limited headroom (`headroom`, L1448-1510):
  D = 1 - tOFF-MIN(max) x fSW(max); avail = D (VIN - I RDS_hi) - (1 - D) I RDS_lo - I DCR.
- the LDO's need (L1532): VOUT max + dropout max.
`selfcheck` reproduces IR's own numbers, read from unmodified `aethelgard validate` refusals
(see headroom_probe_spec.py and FEEDBACK §1.4).

What is THIS SPIKE'S MODEL (not IR's; IR has no input-path drop, its own note says so):
- the pack is an ideal source at cells.min x cell_min_millivolts behind one series resistance R
  (pack + leads + connector + VBAT copper, swept; where the ESC current joins is a topology
  argument, see `topology`);
- each battery-fed buck draws constant power from VBAT: P_in = VOUT_nom x I_load / eta, with
  eta = 1 (lossless: the least board current, so the largest R before a crossing) or the
  fixture's own declared efficiency figure (`junction.loss_milliwatts`' source text);
- the ESC current is a constant current on VBAT (sourced, or a labelled sweep);
- a buck below its stated minimum operating input (vin_millivolts.min, 3.8 V) has an UNSTATED
  output: the rule must then assume the rail can be anywhere, so it is counted as entering
  BOR3's band (conservative). In dropout above it, the rail is IR's `avail` (inferred physics:
  the duty limit read as an output, not as a refusal).
"""

import json
import math
import sys

SPEC03 = "crates/aethelgard-cli/tests/fixtures/sentinel-03.json"
SPEC02P = "crates/aethelgard-ir/tests/fixtures/sentinel-02-power.json"
PARTS = "crates/aethelgard-ir/data/power-parts.json"
# chaos/src/power.rs L51-72: VBOR3 falling 2.54-2.68 V (min-max), typical 2.61 V.
BOR3_FALLING_MAX_MV = 2680
BOR3_FALLING_TYP_MV = 2610


def div_ceil(a, b):
    q = abs(a) // abs(b)
    neg = (a < 0) != (b < 0)
    if neg:
        return -q
    return q + (1 if a % b != 0 else 0)


def trunc_div(a, b):
    # Rust i128 `/` truncates toward zero.
    q = abs(a) // abs(b)
    return -q if (a < 0) != (b < 0) else q


class Board:
    def __init__(self, spec_path, parts_path=PARTS):
        self.spec = json.load(open(spec_path))
        self.parts = {r["part"]: r for r in json.load(open(parts_path))["parts"]}
        self.power = self.spec["power"]
        self.domains = {d["id"]: d for d in self.spec["voltage_domains"]}
        self.regs = {r["reference"]: r for r in self.power["regulators"]}

    def kind(self, part):
        k = self.parts[part]["kind"]
        (name, body), = k.items()
        return name, body

    def battery_min_mv(self):
        b = self.power["battery"]
        return b["cells"]["min"] * b["cell_min_millivolts"]["value"]

    def temp(self):
        return self.power["conditions"]["board_temperature_celsius"]

    def err_ppm(self, res):
        _, r = self.kind(res["part"])
        t = self.temp()
        ref = r["tcr_reference_celsius"]
        exc = max(t["max"] - ref, ref - t["min"], 0)
        return r["tolerance_ppm"]["value"] + r["tcr_ppm_per_kelvin"]["value"] * exc + r["drift_ppm"]["value"]

    def vout_uv(self, ref):
        reg = self.regs[ref]
        name, b = self.kind(reg["part"])
        if name == "ldo":
            v = b["vout_millivolts"]
            return v["min"] * 1000, v["max"] * 1000
        f = reg["feedback"]
        m = 1_000_000
        et, eb = self.err_ppm(f["top"]), self.err_ppm(f["bottom"])
        rt, rb = f["top"]["ohms"], f["bottom"]["ohms"]
        vfb = (b["vfb_microvolts"]["min"], b["vfb_microvolts"]["max"])
        rt_hi, rt_lo = rt * (m + et), rt * (m - et)
        rb_hi, rb_lo = rb * (m + eb), rb * (m - eb)
        ifb = b["ifb_max_nanoamps"]["value"]
        lo_nv = trunc_div(vfb[0] * 1000 * (rb_hi + rt_lo), rb_hi) - div_ceil(ifb * rt_hi, m)
        hi_nv = div_ceil(vfb[1] * 1000 * (rb_lo + rt_hi), rb_lo) + div_ceil(ifb * rt_hi, m)
        band = b["regulation_permille"]
        lo = trunc_div(lo_nv * (1000 + band["min"]), 1_000_000)
        hi = div_ceil(hi_nv * (1000 + band["max"]), 1_000_000)
        return max(lo, 0), hi

    def loads_ma(self):
        load = {}
        mcu_supply = self.spec["mcu"]["supply"]
        _, mcu = self.kind(self.spec["mcu"]["part"])
        regs = list(self.regs.values())
        for _ in range(len(regs) + 1):
            for r in regs:
                out = r["output"]
                if out in load:
                    continue
                kids = [c for c in regs if c["input"] == out]
                if any(c["output"] not in load for c in kids):
                    continue
                s = sum(l["milliamps"] for l in self.power["loads"] if l["domain"] == out)
                if out == mcu_supply:
                    s += mcu["idd_max_milliamps"]["value"]
                for c in kids:
                    cname, cb = self.kind(c["part"])
                    s += load[c["output"]]
                    if cname == "ldo":
                        s += div_ceil(cb["ignd_max_microamps"]["value"], 1000)
                load[out] = s
        return load

    def buck_avail_uv(self, ref, vin_mv, i_ma):
        """IR's `headroom` arithmetic (power.rs L1462-1491), at a given VIN and load."""
        reg = self.regs[ref]
        _, b = self.kind(reg["part"])
        toff = b["toff_min_max_ns"]["value"]
        fsw = b["fsw_hz"]["max"]
        d_ppm = 1_000_000 - div_ceil(toff * fsw, 1000)
        ind = next(e for e in reg["external"] if e["role"] == "inductor")
        _, l = self.kind(ind["part"])
        dcr = l["dcr_max_milliohms"]["value"]
        rhs = b["rds_high_max_milliohms"]["value"]
        rls = b["rds_low_max_milliohms"]["value"]
        vin_uv = vin_mv * 1000
        i = i_ma
        return (trunc_div(d_ppm * (vin_uv - i * rhs), 1_000_000)
                - div_ceil((1_000_000 - d_ppm) * i * rls, 1_000_000) - i * dcr), d_ppm

    def vin_min_mv(self, ref):
        _, b = self.kind(self.regs[ref]["part"])
        return b["vin_millivolts"]["min"]

    def regulation_vin_min_mv(self, ref):
        reg = self.regs[ref]
        _, b = self.kind(reg["part"])
        nominal = self.domains[reg["output"]]["nominal_millivolts"]
        for st in b["regulation_vin_min"]:
            if nominal >= st["from_vout_millivolts"]:
                return st["vin_min"]["value"]
        return None

    def eta_declared(self, ref):
        """The fixture's own efficiency figure, parsed from its junction loss source text
        (e.g. '90 % on the 36 V curve at 2 A'). Typical, at 36 V: labelled as such."""
        src = self.regs[ref].get("junction", {}).get("loss_milliwatts", {}).get("source", "")
        import re
        m = re.search(r"(\d+) % on the", src)
        return int(m.group(1)) / 100 if m else None


def selfcheck():
    """IR's numbers, read from unmodified `aethelgard validate` refusals (2026-11-19, cd43b207)."""
    b = Board(SPEC03)
    ok = True

    def eq(name, got, want):
        nonlocal ok
        flag = "ok" if got == want else "MISMATCH"
        ok &= got == want
        print(f"{flag:8} {name}: mirror {got}, IR {want}")

    eq("U11 vout_uv", b.vout_uv("U11"), (3144727, 3417204))
    eq("U10 vout_uv", b.vout_uv("U10"), (4818048, 5237403))
    ld = b.loads_ma()
    eq("load VDD_3V3 (IR 3648 with +2900)", ld["VDD_3V3"], 3648 - 2900)
    eq("load VDD_5V (IR 4900 with +2900)", ld["VDD_5V"], 4900 - 2900)
    for vin, i, want in [(3801, 3000, 3130245), (3801, 2748, 3174104), (3801, 2548, 3208914), (3900, 3000, 3225375)]:
        eq(f"U11 avail at {vin} mV {i} mA", b.buck_avail_uv("U11", vin, i), (want, 960900))
    print("selfcheck", "PASS" if ok else "FAIL")
    return ok


def operating_point(v_open, r_ohm, i_esc_a, p_board_w):
    """VBAT = v_open - R (I_esc + P/VBAT): the higher root, or None (no operating point:
    the constant-power input collapses)."""
    a = v_open - r_ohm * i_esc_a
    disc = a * a - 4 * r_ohm * p_board_w
    if a <= 0 or disc < 0:
        return None
    return (a + math.sqrt(disc)) / 2


def chain_at(b, v_open, r, i_esc, eta_mode):
    loads = b.loads_ma()
    bat_regs = [x for x in b.regs.values() if x["input"] == "battery"]
    p = 0.0
    for x in bat_regs:
        vnom = b.domains[x["output"]]["nominal_millivolts"] / 1000
        eta = 1.0 if eta_mode == "lossless" else b.eta_declared(x["reference"])
        p += vnom * loads[x["output"]] / 1000 / eta
    v = operating_point(v_open, r, i_esc, p)
    out = {"p_board_w": p, "vbat_v": v}
    if v is None:
        return out
    vmv = int(v * 1000)
    for x in bat_regs:
        ref = x["reference"]
        lo, hi = b.vout_uv(ref)
        avail, _ = b.buck_avail_uv(ref, vmv, loads[x["output"]])
        if vmv < b.vin_min_mv(ref):
            rail = None  # below the stated operating input: unstated
        else:
            rail = min(avail, (lo + hi) // 2)  # regulated near its set point, or IR's avail in dropout
        out[x["output"]] = {"ref": ref, "rail_uv": rail, "avail_uv": avail, "vout_lo_uv": lo,
                            "vin_min_mv": b.vin_min_mv(ref), "regulation_vin_min_mv": b.regulation_vin_min_mv(ref)}
    # A child LDO (the power fixture's U12).
    for x in b.regs.values():
        if x["input"] != "battery" and x["input"] in out and out[x["input"]]["rail_uv"] is not None:
            _, l = b.kind(x["part"])
            need = l["vout_millivolts"]["max"] + l["dropout_max_millivolts"]["value"]
            vin = out[x["input"]]["rail_uv"] / 1000
            out[x["output"]] = {"ref": x["ref"] if "ref" in x else x["reference"], "need_mv": need,
                                "rail_uv": min(l["vout_millivolts"]["min"] * 1000, int((vin - l["dropout_max_millivolts"]["value"]) * 1000)) if vin >= need else None}
    return out


def first_r(pred, lo=0.0, hi=1000.0, steps=60):
    """The least R (ohm) at which pred(R) is true, by bisection; pred is monotone in R."""
    if not pred(hi):
        return None
    if pred(lo):
        return 0.0
    for _ in range(steps):
        mid = (lo + hi) / 2
        if pred(mid):
            hi = mid
        else:
            lo = mid
    return hi


def thresholds(b, i_esc, eta_mode):
    v_open = b.battery_min_mv() / 1000
    loads = b.loads_ma()

    def collapse(r):
        return chain_at(b, v_open, r, i_esc, eta_mode)["vbat_v"] is None

    def vbat_below(mv):
        def f(r):
            o = chain_at(b, v_open, r, i_esc, eta_mode)
            return o["vbat_v"] is None or o["vbat_v"] * 1000 < mv
        return f

    def rail_below(dom, uv):
        def f(r):
            o = chain_at(b, v_open, r, i_esc, eta_mode)
            if o["vbat_v"] is None or dom not in o or o[dom]["rail_uv"] is None:
                return True
            return o[dom]["rail_uv"] < uv
        return f

    res = {"v_open_v": v_open, "i_esc_a": i_esc, "eta": eta_mode,
           "p_board_w_at_nominal": chain_at(b, v_open, 0.0, i_esc, eta_mode)["p_board_w"]}
    res["R_collapse"] = first_r(collapse)
    u11 = next(x for x in b.regs.values() if x["output"] == b.spec["mcu"]["supply"])
    res["R_3v3_unstated"] = first_r(vbat_below(b.vin_min_mv(u11["reference"])))
    # Which limit binds: the operating point's last V_bat before it vanishes, against U11's floor.
    if res["R_collapse"] is not None:
        a = v_open - res["R_collapse"] * i_esc
        res["vbat_at_collapse_v"] = a / 2
        res["binding"] = "collapse" if a / 2 >= b.vin_min_mv(u11["reference"]) / 1000 else "U11 below 3.8 V"
    else:
        res["binding"] = "U11 below 3.8 V"
    res["R_3v3_below_3v8_only"] = first_r(lambda r: (lambda o: o["vbat_v"] is not None and o["vbat_v"] * 1000 < b.vin_min_mv(u11["reference"]))(chain_at(b, v_open, r, i_esc, eta_mode)), hi=res["R_collapse"] or 1000.0)
    res["R_3v3_avail_below_bor3_max"] = first_r(rail_below(u11["output"], BOR3_FALLING_MAX_MV * 1000))
    res["R_3v3_enters_bor3_band"] = min(x for x in (res["R_3v3_unstated"], res["R_3v3_avail_below_bor3_max"]) if x is not None)
    # The deck's own rule (spike/2304/lockstep `deck`): the bucks run down to the datasheet's
    # typical shutdown (about 3 V, SNVSAN3F 8.3.4) with VDD_3V3 = avail in between, which stays
    # above BOR3's 2.68 V maximum down to V_bat ~2.93 V; so the deck's VDD_3V3 first falls through
    # BOR3 when V_bat < 3.0 V or there is no operating point. Adversarial M1: the static column
    # above counts below 3.8 V (U11's stated input range) instead.
    res["R_3v3_deck_rule_vbat_below_3v0"] = first_r(vbat_below(3000))
    for x in b.regs.values():
        if x["input"] == "battery" and x["output"] != u11["output"]:
            d = b.domains[x["output"]]
            band_lo = d["nominal_millivolts"] * (1000 - d["tolerance_permille"]) // 1000
            res[f"R_{x['output']}_leaves_tolerance"] = first_r(rail_below(x["output"], band_lo * 1000))
            res[f"R_{x['output']}_band_unstated"] = first_r(vbat_below(b.regulation_vin_min_mv(x["reference"])))
    res["R_3v3_band_unstated"] = first_r(vbat_below(b.regulation_vin_min_mv(u11["reference"])))
    if "VDD_1V8" in b.domains:
        d = b.domains["VDD_1V8"]
        band_lo = d["nominal_millivolts"] * (1000 - d["tolerance_permille"]) // 1000
        res["R_VDD_1V8_leaves_tolerance"] = first_r(rail_below("VDD_1V8", band_lo * 1000))
    return res


def run(cases_path, r_points):
    cases = json.load(open(cases_path))
    out = {"bor3_falling_max_mv": BOR3_FALLING_MAX_MV, "boards": {}}
    for name, path in (("sentinel-03", SPEC03), ("sentinel-02-power", SPEC02P)):
        b = Board(path)
        rows = []
        for c in cases["cases"]:
            for eta in ("lossless", "declared"):
                t = thresholds(b, c["i_esc_a"], eta)
                t["case"] = c["name"]
                t["label"] = c["label"]
                # Margins at the stated credible R values.
                t["at"] = []
                for r in r_points:
                    o = chain_at(b, t["v_open_v"], r, c["i_esc_a"], eta)
                    row = {"R": r, "vbat_v": o["vbat_v"]}
                    for dom in b.domains:
                        if dom in o and o[dom].get("rail_uv") is not None:
                            row[dom + "_mv"] = o[dom]["rail_uv"] / 1000
                    if o["vbat_v"] is not None:
                        row["margin_vbat_to_3v3_unstated_mv"] = o["vbat_v"] * 1000 - b.vin_min_mv("U11")
                        row["margin_vbat_to_U10_regulation_vin_min_mv"] = o["vbat_v"] * 1000 - b.regulation_vin_min_mv("U10")
                        if "VDD_3V3" in o and o["VDD_3V3"]["rail_uv"] is not None:
                            row["margin_3v3_to_bor3_max_mv"] = o["VDD_3V3"]["rail_uv"] / 1000 - BOR3_FALLING_MAX_MV
                    t["at"].append(row)
                rows.append(t)
        out["boards"][name] = {"loads_ma": b.loads_ma(), "battery_min_mv": b.battery_min_mv(),
                               "vout_uv": {k: b.vout_uv(k) for k in b.regs if b.kind(b.regs[k]["part"])[0] == "buck"},
                               "eta_declared": {k: b.eta_declared(k) for k in b.regs if b.regs[k]["input"] == "battery"},
                               "rows": rows}
    json.dump(out, sys.stdout, indent=1, sort_keys=True)
    print()


if __name__ == "__main__":
    mode = sys.argv[1]
    if mode == "selfcheck":
        sys.exit(0 if selfcheck() else 1)
    elif mode == "run":
        run(sys.argv[2], [float(x) for x in sys.argv[3].split(",")])
    else:
        sys.exit("usage: chain.py selfcheck | run CASES.json R1,R2,...")
