#!/usr/bin/env python3
"""AET-WO-1218 §1.1: write the scratch boards, each one edit of a committed fixture.
D2 is `d2-drv8833.json` drawn, with its input PWM limit declared (as `pin_devices_solve.rs`' `d2_declared`);
D1 is `d1-drv8876.json` drawn (as `emit_tier1.rs`' `d1_drawn`).
Usage: python3 -I probes.py <repository root> <out dir>"""
import copy, json, sys

repo, out = sys.argv[1], sys.argv[2]
fx = f"{repo}/crates/aethelgard-ir/tests/fixtures"
d2 = json.load(open(f"{fx}/d2-drv8833.json"))
d2["pin_devices"][0]["part_limits"]["input_pwm_max_hz"] = {
    "value": 50000, "source": "a declaration for this test, no held document"}
d2["board"].pop("drawn", None)
d1 = json.load(open(f"{fx}/d1-drv8876.json"))
d1["board"].pop("drawn", None)

def probe(name, base, tps):
    s = copy.deepcopy(base)
    if tps is not None:
        s["test_points"] = [{"net": n, "reference": r} for n, r in tps]
    json.dump(s, open(f"{out}/{name}.json", "w"), indent=2)
    print(name)

# The bare boards: the references a test point is added to.
probe("d2-bare", d2, None)
probe("d1-bare", d1, None)
# (1): a test point beside a pin device.
probe("d2-tp-gnd", d2, [("GND", "TP1")])
probe("d1-tp-gnd", d1, [("GND", "TP1")])
probe("d2-tp-nfault", d2, [("DRV_NFAULT", "TP1")])
probe("d2-tp-boot0", d2, [("BOOT0", "TP1")])
probe("d2-tp-two", d2, [("GND", "TP1"), ("DRV_NSLEEP", "TP2")])
# Collisions that must stay refused by step 15a: the pin device's own reference, exact and by key,
# and D1's passives (its first resistor's reference, read from the fixture).
probe("d2-tp-U2", d2, [("GND", "U2")])
probe("d2-tp-u02", d2, [("GND", "u02")])
dev = d1["pin_devices"][0]
passive = next(a["transfer"]["current_output"]["resistor"]["reference"] for a in dev["analog"])
probe(f"d1-tp-{passive}", d1, [("GND", passive)])
probe(f"d1-tp-{passive.lower()}", d1, [("GND", passive.lower())])
# The same collisions outside the emitter's numbering, so step 15a's "every other reference" is what
# refuses them, not `emitter_prefix`: D2's device renamed `DRV1`, D1's sense resistor renamed `RSENSE`.
d2n = copy.deepcopy(d2)
d2n["pin_devices"][0]["reference"] = "DRV1"
probe("d2n-bare", d2n, None)
probe("d2n-tp-DRV1", d2n, [("GND", "DRV1")])
probe("d2n-tp-drv01", d2n, [("GND", "drv01")])
d1n = copy.deepcopy(d1)
for a in d1n["pin_devices"][0]["analog"]:
    a["transfer"]["current_output"]["resistor"]["reference"] = "RSENSE"
probe("d1n-bare", d1n, None)
probe("d1n-tp-RSENSE", d1n, [("GND", "RSENSE")])
probe("d1n-tp-rsense", d1n, [("GND", "rsense")])
# R5: a reservoir RAIN does not need, on D1 (R2 at 1070, 1000 and 900 ohms, each under RAIN at 12 bits)
# and on the power tree's sense (sentinel-02-power, 10 k / 1 k): a part no row holds, a resistor row,
# and (power) the held capacitor, which stays admitted.
pw = json.load(open(f"{fx}/sentinel-02-power.json"))
for part in ["NOPE", "CRCW0603_F_K", "GCM188R71H104KA57"]:
    s = copy.deepcopy(pw)
    sense = s["power"]["sense"][0]
    sense["top"]["ohms"], sense["bottom"]["ohms"] = 10000, 1000
    sense["reservoir"]["part"] = part
    probe(f"r5-power-under-rain-{part}", s, None)
for ohms in [1070, 1000, 900]:
    for part in ["NOPE", "CRCW0603_F_K"]:
        s = copy.deepcopy(d1)
        c = s["pin_devices"][0]["analog"][0]["transfer"]["current_output"]
        c["resistor"]["ohms"], c["reservoir"]["part"] = ohms, part
        probe(f"r5-d1-{ohms}-{part}", s, None)
# Adversarial (§5): readers that number around the declared references. The reset switch (numbered
# `SW<n>` past every declared reference) beside a pin device and a test point on `NRST`; a test point
# on a bridge output and on the motor supply.
sw = copy.deepcopy(d2)
sw["mcu"]["reset_switch"] = True
probe("adv-d2-switch-bare", sw, None)
probe("adv-d2-switch-tp-nrst", sw, [("NRST", "TP1")])
probe("adv-d2-tp-maout1", d2, [("MA_OUT1", "TP1")])
probe("adv-d2-tp-vmotor", d2, [("VMOTOR", "TP1")])
