#!/usr/bin/env python3
"""AET-WO-1217 §1.1: write the probe specifications, each one edit of a committed file.
Usage: python3 -I probes.py <repository root> <out dir>"""
import copy, json, sys

repo, out = sys.argv[1], sys.argv[2]
d1 = json.load(open(f"{repo}/crates/aethelgard-ir/tests/fixtures/d1-drv8876.json"))
minimal = json.load(open(f"{repo}/examples/sentinel-01-minimal.json"))

def dev(s):
    return s["pin_devices"][0]

def probe(name, base, edit):
    s = copy.deepcopy(base)
    edit(s)
    json.dump(s, open(f"{out}/{name}.json", "w"), indent=2)
    print(name)

# F9: an ADC refusal and an A2 refusal on one device.
def vref_gnd(s): dev(s)["straps"]["VREF"] = {"tie": "GND"}
def no_reservoir(s): dev(s)["analog"][0]["transfer"]["current_output"]["reservoir"] = None
def over_vdda(s): dev(s)["analog"][0]["transfer"]["current_output"]["resistor"]["ohms"] = 2000
probe("f9-vref-gnd", d1, vref_gnd)
probe("f9-no-reservoir", d1, no_reservoir)
probe("f9-vref-gnd-no-reservoir", d1, lambda s: (vref_gnd(s), no_reservoir(s)))
probe("f9-over-vdda", d1, over_vdda)
probe("f9-vref-gnd-over-vdda", d1, lambda s: (vref_gnd(s), over_vdda(s)))

# F10: an MCU part with no power-parts row.
def other_mcu(s): s["mcu"]["part"] = "STM32H743ZITx"
probe("f10-d1-other-mcu", d1, other_mcu)
probe("f10-minimal-other-mcu", minimal, other_mcu)

# F2: IPROPI tied to GND.
def ipropi_gnd(s): dev(s)["straps"]["IPROPI"] = {"tie": "GND"}
def no_analog(s): dev(s)["analog"] = []
probe("f2-ipropi-gnd", d1, lambda s: (ipropi_gnd(s), no_analog(s)))
probe("f2-ipropi-gnd-vref-gnd", d1, lambda s: (ipropi_gnd(s), no_analog(s), vref_gnd(s)))
probe("f2-ipropi-gnd-and-analog", d1, ipropi_gnd)
probe("f2-no-analog", d1, no_analog)
