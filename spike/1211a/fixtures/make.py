#!/usr/bin/env python3
"""AET-WO-1211a §1.6: D1 and D2 as committed test fixtures, from 1211's spike fixtures
(spike/1211/fixtures/d{1,2}-*.json, merged as the record in #288), with exactly these changes:
- schema_version: the engine's current specification version (renumber.py moves it with the rest);
- each pin device gains `part_limits` (1211 §7 as ruled: a limit the row lacks is declared here with
  its citation) and `temperature_celsius` (`null` when `power` declares the board temperature; D1 and
  D2 declare no power tree, so D1, which has resistors to judge, declares one; D2 has none);
- D1's IPROPI input gains `channel: null` (1211 §2.3's key, the solver to choose; the spike's file
  left it out), and: full_scale 2000 -> 1900 mA, adc_resolution_bits 16 -> 12, and a reservoir
  capacitor C1 (the sentinel-03 battery sense's part). As the spike wrote it, D1 is refused twice by
  the rules this work order builds: at 16 bits R2 = 1500 ohms is above RAIN's 170 ohms (DS12117
  Rev 10 Table 173 p. 273, rev V, the narrower table) and no reservoir is declared
  (AdcSourceImpedance); and 2 A x 1000 uA/A x 1.05 (AERR's worst band, SLVSDS7B p. 6) x 1500 ohms at
  CRCW0603_F_K's worst error is above VREF+'s minimum, VDD_3V3's 3135 mV low edge
  (AdcFullScaleExceeded). The spike judged 3.15 V against the 3.3 V nominal. Both refusals are kept
  as tests (pin_devices.rs) on the spike's values.
- each declares `board.drawn: false` (AET-WO-1210's spec 21): a pin device has no emitter arm until
  1211d, so it is declared only on a specification that draws no board (PartNotDrawable otherwise);
- D1 and D2 keep every other value, page and pin of the spike.
Usage: python3 -I make.py REPO VERSION"""
import collections, json, sys

repo, version = sys.argv[1], int(sys.argv[2])
O = collections.OrderedDict
src = f"{repo}/spike/1211/fixtures"
out = f"{repo}/crates/aethelgard-ir/tests/fixtures"

def load(name):
    return json.load(open(f"{src}/{name}"), object_pairs_hook=O)

def device(d, temperature):
    d["part_limits"] = O(input_pwm_max_hz=None)
    d["temperature_celsius"] = temperature

d1 = load("d1-drv8876.json")
d1["schema_version"] = version
dev = d1["pin_devices"][0]
device(dev, O(min=-20, max=85, source="D1 fixture, declared: Sentinel-02's board range (crates/aethelgard-ir/tests/fixtures/sentinel-02-power.json, power.conditions)"))
a = O((k, v) for k, v in dev["analog"][0].items())
a = O(list(a.items())[:3] + [("channel", None)] + list(a.items())[3:])
dev["analog"][0] = a
a["full_scale"] = O(milliamps=1900)
a["adc_resolution_bits"] = 12
a["transfer"]["current_output"]["reservoir"] = O(reference="C1", part="GCM188R71H104KA57")
d1["board"]["drawn"] = False
json.dump(d1, open(f"{out}/d1-drv8876.json", "w"), indent=2)
open(f"{out}/d1-drv8876.json", "a").write("\n")

d2 = load("d2-drv8833.json")
d2["schema_version"] = version
device(d2["pin_devices"][0], None)
d2["board"]["drawn"] = False
json.dump(d2, open(f"{out}/d2-drv8833.json", "w"), indent=2)
open(f"{out}/d2-drv8833.json", "a").write("\n")
print("d1, d2 at", version)
