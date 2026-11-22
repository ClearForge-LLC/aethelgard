"""AET-WO-4300h walk 1: every MCU pin a connector pin or a test point reaches,
read from the specification and its solved topology.

usage: python3 -I facing.py <spec.json> <solve.json or -> <power-parts.json> <undeclared.tsv> <board>
       -> TSV on stdout, one row per (off-board end, net), header first

Every connector pin is listed: one on an MCU net as `connector`, one on a rail,
GND or a net no MCU pin carries as `connector_other` (so no pin is silently
dropped). A test point on an MCU net is `test_point`. Two further classes are
not `connectors[]` or `test_points[]` at all, and no option of the work order
sees them: `undeclared_offboard` (a sidecar row: the bench's header wiring) and
`foreign_clock` (`mcu.hse.external` onto PH0). `power_input` (the battery
connector `power.input` draws, adversarial F2) is listed for each sense net it
reaches through a divider's top resistor. A test point on a rail or GND is not
listed (walk 2 lists it).

`fw_pull` is the internal pull the solved topology implies after the first GPIO
init, by `aethelgard-ir` `silicon::implied_pupd`'s definition (adversarial F1):
a motor output pulls down, or up for a declared idle level high; a GPIO input
takes its declared pull; every other pin none. `reset_pull` is the pull from
reset to that write (RM0433 Rev 9 p. 542).

The structure is DS12117 Rev 10 Table 8's, as `power-parts.json` holds it
(`kind.mcu.io.pins`, `1211e`); NRST is `RST` and BOOT0 is `B` (Table 8 rows
the io table does not hold). Walk 2 (inward.py) reads the structure from the
rendered-page transcription instead, and table8.py reconciles the three reads.
"""
import json
import sys

HEADER = ["board", "end", "class", "net", "pin", "structure", "signal", "hand", "mcu_dir",
          "reset_pull", "fw_pull", "rails_on_end", "far_end", "source"]

# RM0433 Rev 9 p. 542: GPIOA_PUPDR resets to 0x6400 0000 (PA15 up, PA14 down,
# PA13 up), GPIOB_PUPDR to 0x0000 0100 (PB4 up); every other pin resets with
# no pull. Table 13 note 3: above VDD + 0.3 V the pulls must be off.
RESET_PULL = {"PA13": "up", "PA14": "down", "PA15": "up", "PB4": "up"}
FIXED = (("NRST", "NRST", "NRST (RST, bidirectional, embedded weak pull-up)", "bidir_od"),
         ("SWDIO", "PA13", "SWDIO (AF0 from reset)", "bidir"),
         ("SWCLK", "PA14", "SWCLK (AF0 from reset)", "in"),
         ("BOOT0", "BOOT0", "BOOT0 (B)", "in"))


def direction(signal, gpio_dir=None):
    if gpio_dir:
        return {"output": "out", "input": "in"}.get(gpio_dir, gpio_dir)
    s = signal.split("_", 1)[-1]
    if signal.startswith(("USART", "UART", "LPUART")):
        return {"TX": "out", "RX": "in"}.get(s, "?")
    if signal.startswith("I2C"):
        return "bidir_od"
    if signal.startswith("SPI"):
        return {"SCK": "out", "MOSI": "out", "MISO": "in"}.get(s, "?")
    if signal.startswith("TIM"):
        return "out"
    if signal.startswith("ADC"):
        return "in"
    return "?"


def main(spec_path, solve_path, pp_path, undeclared_path, board):
    s = json.load(open(spec_path))
    t = json.load(open(solve_path)) if solve_path != "-" else None
    pp = {p["part"]: p for p in json.load(open(pp_path))["parts"]}
    mcu = s["mcu"]
    io = pp.get(mcu["part"], {}).get("kind", {}).get("mcu", {}).get("io", {}).get("pins", {})
    doms = {d["id"] for d in s.get("voltage_domains", [])}

    # the declared (hand) pin of every net, from the specification alone
    hand = {}

    def declared(net, pin):
        if net:
            hand[net] = pin

    for b in s.get("i2c_buses", []):
        declared(b["sda"]["net"], b["sda"].get("pin"))
        declared(b["scl"]["net"], b["scl"].get("pin"))
    for b in s.get("spi_buses", []):
        for k in ("sck", "mosi", "miso"):
            declared(b[k]["net"], b[k].get("pin"))
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
    for x in (s.get("power") or {}).get("sense", []):
        declared(x["net"], None)

    # the solved pin and signal of every MCU net
    pin_of, sig_of, dir_of, pull_of = {}, {}, {}, {}
    if t:
        for k, m in t.get("motor_outputs", {}).items():
            idle = ((m.get("pwm") or {}).get("idle") or {})
            pull_of[k] = "up" if isinstance(idle, dict) and idle.get("level") == "high" else "down"
        for k, g in t.get("gpio", {}).items():
            if g["direction"] == "input" and g.get("pull") in ("up", "down"):
                pull_of[k] = g["pull"]
        for k, v in t.get("assignments", {}).items():
            pin_of[k], sig_of[k] = v["pin"], f"{v['signal']} AF{v['alternate_function']}"
            dir_of[k] = direction(v["signal"])
        for k, v in t.get("gpio", {}).items():
            pin_of[k], sig_of[k] = v["pin"], f"GPIO {v['direction']}"
            dir_of[k] = direction("", v["direction"])
        for k, v in t.get("adc_inputs", {}).items():
            pin_of[k], sig_of[k] = v["pin"], f"{v['peripheral']}_{v['channel']}"
            dir_of[k] = "in"
    else:
        for net, pin in hand.items():
            pin_of[net], sig_of[net], dir_of[net] = pin or "unsolved", "unsolved", "?"
    for net, pin, sig, d in FIXED:
        if net not in pin_of:
            pin_of[net], sig_of[net], dir_of[net] = pin, sig, d

    def structure(pin):
        if pin == "NRST":
            return "RST"
        if pin == "BOOT0":
            return "B"
        return io.get(pin, {}).get("structure", "unheld")

    rows = []

    def add(end, cls, net, rails, far, src):
        pin = pin_of.get(net, "-")
        h = hand.get(net)
        hd = "fixed" if net in dict((f[0], 1) for f in FIXED) and net not in hand else (
            f"hand {h}" if h else ("solver" if net in hand else "-"))
        rows.append([board, end, cls, net, pin, structure(pin) if pin != "-" else "-", sig_of.get(net, "-"), hd,
                     dir_of.get(net, "-"), RESET_PULL.get(pin, "none"), pull_of.get(net, "none"), rails, far, src])

    for c in s.get("connectors", []) or []:
        rails = sorted({p["net"] for p in c["pins"] if p["net"] in doms or p["net"] == "VBAT"})
        std = c.get("standard")
        far = (f"{std}: DS-009 pp. 6-9 prints +3.3V for the signal, a nominal level with no maximum"
               if std and std.startswith("DS-009") else "no standard; nothing declared about the far end")
        for p in c["pins"]:
            net = p["net"]
            end = f"{c['reference']}.{p['pin']}"
            off = p.get("offboard")
            src = f"connectors[] {c['part']}" + (f"; offboard {off['capacitance_femtofarads']} fF ({off['source'][:60]})"
                                                 if off else "; offboard null")
            if net in pin_of:
                add(end, "connector", net, ",".join(rails) or "-", far, src)
            else:
                what = "nc" if net is None else ("GND" if net == "GND" else ("rail" if net in rails else "other"))
                rows.append([board, end, "connector_other", str(net), "-", "-", what, "-", "-", "-", "-",
                             ",".join(rails) or "-", far, src])
    for tp in s.get("test_points", []) or []:
        if tp["net"] in pin_of:
            add(tp["reference"], "test_point", tp["net"], "-", "a probe, when attached", "test_points[]")
    pin_input = (s.get("power") or {}).get("input")
    if pin_input:
        b = s["power"]["battery"]
        for i, x in enumerate(s["power"].get("sense", [])):
            add(pin_input["reference"], "power_input", x["net"], "battery",
                f"the pack, {b['cells']['max']} x {b['cell_max_millivolts']['value']} mV, through "
                f"{x['top']['reference']} {x['top']['ohms']} ohm (1216: 2536 mV on the pin)",
                f"power.input {pin_input['part']}; power.sense[{i}]")
    for line in open(undeclared_path):
        f = line.rstrip("\n").split("\t")
        if f[0] == board:
            add(f[1], "undeclared_offboard", f[2], f[3], f[4], f[5])
    hse = mcu.get("hse")
    if isinstance(hse, dict) and "external" in hse:
        pin_of["HSE_IN"], sig_of["HSE_IN"], dir_of["HSE_IN"] = "PH0", "OSC_IN (HSE bypass)", "in"
        add("mcu.hse.external", "foreign_clock", "HSE_IN", "-", hse["external"]["source"][:100], "mcu.hse.external")
    print("\t".join(HEADER))
    for r in rows:
        print("\t".join(r))


if __name__ == "__main__":
    main(*sys.argv[1:6])
