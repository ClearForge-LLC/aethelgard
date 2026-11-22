"""AET-WO-1216 walk 1: every path onto an MCU pin from anything but
`mcu.supply` or GND, read from the specification and its solved topology.

For each path it gives the most the pin can see while `mcu.supply` is below
1 V, with its source, or "not stated". It knows the specification's fields;
walk 2 (drawn.py) knows only the drawn netlist, and compare.py reconciles
the two.

usage: python3 -I spec.py <spec.json> <solve.json or -> <power-parts.json> <parts.json>
       -> TSV on stdout, one row per path (header first)

The worst voltage is open-circuit, at the source's maximum:
- a domain: nominal x (1 + tolerance), rounded up;
- the battery: cells.max x cell_max_millivolts;
- a divider: the battery's maximum at the ratio's top corner, each resistor
  at tolerance + TCR x the board's widest excursion from the TCR reference
  + drift (the same composition 1211e used for the trip);
- a pull: the domain's maximum through the pull (open circuit: the pin's
  own leakage is the only load, so the pin sees the rail).
Series ohms is the Thevenin resistance the source drives the pin through
(for F5's injected current), "0" for a push-pull output, "-" where unknown.
"""
import json
import math
import sys

HEADER = ["net", "pin", "structure", "class", "source", "rail", "can_lead", "worst_mv",
          "series_ohms", "cite", "powered_by"]


def dom_max(d):
    return math.ceil(d["nominal_millivolts"] * (1000 + d["tolerance_permille"]) / 1000)


def main(spec_path, solve_path, pp_path, parts_path):
    s = json.load(open(spec_path))
    t = json.load(open(solve_path)) if solve_path != "-" else None
    pp = {p["part"]: p for p in json.load(open(pp_path))["parts"]}
    pin_parts = {p["part"]: p for p in json.load(open(parts_path)).get("pin_parts", [])}
    mcu = s["mcu"]
    supply = mcu["supply"]
    io = pp.get(mcu["part"], {}).get("kind", {}).get("mcu", {}).get("io", {}).get("pins", {})
    doms = {d["id"]: d for d in s.get("voltage_domains", [])}
    power = s.get("power")
    rows = []

    # ---- MCU nets and their pins ------------------------------------------------
    pin_of = {}
    if t:
        for k, v in t.get("assignments", {}).items():
            pin_of[k] = v["pin"]
        for k, v in t.get("gpio", {}).items():
            pin_of[k] = v["pin"]
        for k, v in t.get("adc_inputs", {}).items():
            pin_of[k] = v["pin"]
    else:  # refused by solve: the declared pins only, else "unsolved"
        def declared(net, pin):
            if net:
                pin_of[net] = pin or "unsolved"
        for b in s.get("i2c_buses", []):
            declared(b["sda"]["net"], b["sda"].get("pin"))
            declared(b["scl"]["net"], b["scl"].get("pin"))
        for d in s.get("spi_devices", []):
            declared(d["cs"]["net"], d["cs"].get("pin"))
        for p in s.get("serial_ports", []) or []:
            for e in ("tx", "rx"):
                if p.get(e):
                    declared(p[e]["net"], p[e].get("pin"))
        for m in s.get("motor_outputs", []) or []:
            declared(m["label"], m.get("pin"))
        for d in s.get("pin_devices", []) or []:
            for g in d.get("signals", []):
                declared(g["net"], g.get("pin"))
            for a in d.get("analog", []):
                declared(a["net"], a.get("pin"))
        for x in (power or {}).get("sense", []):
            declared(x["net"], None)
    for net, pin in (("NRST", "NRST"), ("SWDIO", "PA13"), ("SWCLK", "PA14"), ("BOOT0", "BOOT0")):
        pin_of.setdefault(net, pin)

    def structure(pin):
        if pin == "NRST":
            return "RST"
        if pin == "BOOT0":
            return "B"
        return io.get(pin, {}).get("structure", "unheld")

    # ---- which rails can be up while mcu.supply is below 1 V ----------------------
    source_of = {}  # domain -> regulator
    if power:
        for r in power.get("regulators", []):
            source_of[r["output"]] = r

    def chain(dom):
        out = []
        while dom in source_of:
            out.append(dom)
            dom = source_of[dom]["input"]
        out.append(dom)
        return out

    def can_lead(rail):
        if rail == "battery":
            return "yes: the tree's root"
        if rail == supply:
            return "no: mcu.supply"
        if not power:
            return "yes: no power tree, no order"
        if supply in chain(rail)[1:]:
            return "no: downstream of mcu.supply (%s)" % " <- ".join(chain(rail))
        # a sibling of mcu.supply: led unless an EN is wired to the other's PG
        return "yes: a sibling of mcu.supply, ordered by no wire (%s; %s)" % (
            " <- ".join(chain(rail)), " <- ".join(chain(supply)))

    def worst_rail(rail):
        if rail == "battery":
            b = power["battery"]
            return b["cells"]["max"] * b["cell_max_millivolts"]["value"]
        return dom_max(doms[rail])

    def res_err_ppm(part):
        r = pp[part]["kind"]["resistor"]
        temps = (power or {}).get("conditions", {}).get("board_temperature_celsius")
        lo, hi = (temps["min"], temps["max"]) if temps else (-40, 125)
        dt = max(abs(hi - r["tcr_reference_celsius"]), abs(lo - r["tcr_reference_celsius"]))
        return r["tolerance_ppm"]["value"] + r["tcr_ppm_per_kelvin"]["value"] * dt + r["drift_ppm"]["value"]

    def add(net, cls, src, rail, worst, series, cite, powered_by="-"):
        pin = pin_of.get(net, "not an MCU net")
        lead = can_lead(rail) if rail not in ("off-board", "-") else (
            "yes: off-board" if rail == "off-board" else "-")
        if lead.startswith("no:") and isinstance(worst, int):
            worst = "<=VDD"
        rows.append([net, pin, structure(pin), cls, src, rail, lead, str(worst), str(series), cite, powered_by])

    mcu_nets = set(pin_of)

    # 1. I2C pull-ups
    for i, b in enumerate(s.get("i2c_buses", [])):
        for line in ("sda", "scl"):
            net = b[line]["net"]
            if b["domain"] != supply:
                add(net, "i2c_pull", f"i2c_buses[{i}] {b['pull_up_ohms']} ohm", b["domain"],
                    worst_rail(b["domain"]), b["pull_up_ohms"], f"i2c_buses[{i}].domain")
    # 2. devices on a bus, powered from another domain (push-pull MISO, open-drain I2C)
    for i, d in enumerate(s.get("spi_devices", [])):
        if d["supply"] != supply and not any(c["bus"] == d["bus"] for c in (power or {}).get("crossings", [])):
            for n, sig in ((x, y) for x, y in t["spi_lines"].items()) if t else []:
                pass
            miso = [k for k, v in (t or {}).get("assignments", {}).items() if v["signal"].endswith("MISO")]
            add(",".join(miso) or "MISO", "spi_device_output", f"spi_devices[{i}] {d['reference']}", d["supply"],
                worst_rail(d["supply"]), 0, f"spi_devices[{i}].supply")
    for i, d in enumerate(s.get("i2c_devices", [])):
        if d["supply"] != supply:
            add("i2c lines of " + d["bus"], "i2c_device_open_drain", f"i2c_devices[{i}] {d['reference']}",
                d["supply"], 0, "-", f"i2c_devices[{i}].supply: open drain, drives low only")
    # 3. the power tree: sense dividers, crossings, control wires
    if power:
        for i, x in enumerate(power.get("sense", [])):
            top, bot = x["top"], x["bottom"]
            et, eb = res_err_ppm(top["part"]), res_err_ppm(bot["part"])
            rt, rb = top["ohms"] * (1 - et / 1e6), bot["ohms"] * (1 + eb / 1e6)
            vb = worst_rail("battery")
            w = math.ceil(vb * rb / (rt + rb))
            th = round(top["ohms"] * bot["ohms"] / (top["ohms"] + bot["ohms"]))
            add(x["net"], "battery_divider", f"power.sense[{i}] {top['reference']}/{bot['reference']} "
                f"{top['ohms']}/{bot['ohms']} ohm, each +/-{et/1e4:.2f} %", "battery", w, th,
                f"battery {power['battery']['cells']['max']} x {power['battery']['cell_max_millivolts']['value']} mV; "
                f"{top['part']}/{bot['part']} rows (tolerance + TCR + drift)")
        for i, c in enumerate(power.get("crossings", [])):
            nets = [k for k, v in (t or {}).get("assignments", {}).items()
                    if t["spi_lines"].get(k, {}).get("bus") == c["bus"] or k.startswith(c["bus"] + "_")]
            cs = [d["cs"]["net"] for d in s.get("spi_devices", []) if d["bus"] == c["bus"]]
            for n in sorted(set(nets) | set(cs)) or [c["bus"] + " lines"]:
                add(n, "crossing_a_side", f"power.crossings[{i}] {c['reference']} A side (VCCA; the B side "
                    f"is {c['b_domain']})", c["a_domain"], worst_rail(c["a_domain"]), 0,
                    f"power.crossings[{i}].a_domain")
            for n in cs:
                add(n, "crossing_cs_pull", f"power.crossings[{i}].cs_pull_up {c['cs_pull_up']['reference']} "
                    f"{c['cs_pull_up']['ohms']} ohm", c["a_domain"], worst_rail(c["a_domain"]),
                    c["cs_pull_up"]["ohms"], f"power.crossings[{i}].cs_pull_up, on the A side")
        for owner in [("regulators", r) for r in power.get("regulators", [])] + \
                     [("crossings", c) for c in power.get("crossings", [])]:
            for k in owner[1].get("controls", []):
                if k.get("wire") and k["wire"]["net"] in mcu_nets:
                    add(k["wire"]["net"], "control_wire", f"{owner[1]['reference']}.{k['pin']} wired",
                        owner[1].get("output", owner[1].get("b_domain")), "not stated", "-",
                        f"power.{owner[0]} controls")
    # 4. pin devices
    for i, d in enumerate(s.get("pin_devices", []) or []):
        row = pin_parts.get(d["part"], {})
        for g in d.get("signals", []):
            kind = row.get("pins", {}).get(g["device_pin"], {}).get("kind")
            pull = (g.get("pull") or {}).get("resistor")
            if pull and pull["to"] != supply and pull["to"] != "GND":
                add(g["net"], "pin_device_pull", f"pin_devices[{i}] {pull['reference']} {pull['ohms']} ohm",
                    pull["to"], worst_rail(pull["to"]), pull["ohms"], f"pin_devices[{i}].signals pull")
            if kind not in ("open_drain_output", "logic_input", "enable_input", None):
                add(g["net"], "pin_device_output", f"{d['reference']}.{g['device_pin']} ({kind})",
                    "/".join(sorted(set(d["supplies"].values()))), "not stated", "-", row.get("source", {}).get("document", "?"))
        for a in d.get("analog", []):
            p = row.get("pins", {}).get(a["device_pin"], {})
            if p.get("kind") == "current_output":
                tr = a["transfer"]["current_output"]["resistor"]
                rmax = tr["ohms"] * (1 + res_err_ppm(tr["part"]) / 1e6) if tr["part"] in pp else tr["ohms"]
                w = math.ceil(p["output_max_ua"] * rmax / 1000)
                add(a["net"], "pin_device_current_output",
                    f"{d['reference']}.{a['device_pin']} into {tr['reference']} {tr['ohms']} ohm: "
                    f"{p['output_max_ua']} uA (row output_max_ua) x {rmax:.0f} ohm, if the driver drives",
                    "/".join(sorted(set(d["supplies"].values()))), w, round(rmax),
                    f"{row['source']['document']} {p['page']}")
    # 5. connectors and test points: the far end is off-board
    for c in s.get("connectors", []) or []:
        rails = sorted({p["net"] for p in c["pins"] if p["net"] in doms or p["net"] in ("VBAT",)})
        for p in c["pins"]:
            if p["net"] in mcu_nets:
                add(p["net"], "connector", f"{c['reference']}.{p['pin']} {c['part']}"
                    + (f" ({c['standard']})" if c.get("standard") else "")
                    + (f"; the connector also carries {','.join(rails)}" if rails else "; carries no rail"),
                    "off-board", "not stated", "-", "connectors[]: no off-board voltage is declared",
                    ",".join(r for r in rails if r != supply) or "-")
    for tp in s.get("test_points", []) or []:
        if tp["net"] in mcu_nets:
            add(tp["net"], "test_point", tp["reference"], "off-board", "not stated", "-",
                "test_points[]: a probe, when attached")
    # 6. a foreign clock onto PH0
    hse = mcu.get("hse")
    if isinstance(hse, dict) and "external" in hse:
        pin_of["HSE_IN"] = "PH0"
        add("HSE_IN", "foreign_clock", "mcu.hse.external: " + hse["external"]["source"][:80], "off-board",
            "not stated", "-", "mcu.hse.external")
    # 7. ESCs on motor outputs (fed from the pack)
    for m in s.get("motor_outputs", []) or []:
        esc = m.get("esc")
        if esc:
            add(m["label"], "esc", "motor_outputs esc: " + json.dumps(esc)[:90], "off-board", "not stated", "-",
                "motor_outputs[].esc")
    print("\t".join(HEADER))
    for r in rows:
        print("\t".join(r))


if __name__ == "__main__":
    main(*sys.argv[1:5])
