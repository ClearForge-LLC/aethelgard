#!/usr/bin/env python3
"""AET-WO-1211e §1.2: write the rows the three rules read, once.
- power-parts.json (schema 5 -> 6): the MCU row's `io`, each LQFP144 I/O pin's
  structure from DS12117 Rev 10 Table 8 (out/table8-text.tsv, which the visual
  read out/table8-visual.tsv agrees with row for row) keyed by the pin table's
  canonical name, and each structure family's operating input from Table 13
  (ruled D-A1, 19:58Z; the WO's Table 10 is a stress rating).
- parts.json (schema 11 -> 12): the DRV8876 row's `trip` (VREF sets ITRIP
  against IPROPI, SLVSDS7B p. 13) and `bulk_unstated` (CVM2, p. 10 Table 1 and
  section 9.1 p. 31).
Usage: python3 -I rows.py <repo>"""
import json, re, sys
import xml.etree.ElementTree as ET

repo = sys.argv[1]
data = f"{repo}/crates/aethelgard-ir/data"
xml = f"{repo}/crates/aethelgard-svd-hal/data/st-open-pin-data/STM32H753ZITx.xml"
io_type = {p.get("Name"): p.get("Type") for p in ET.parse(xml).getroot().iter() if p.tag.endswith("Pin")}

pins = {}
for line in open(f"{repo}/spike/1211e/out/table8-text.tsv"):
    name, pos, typ, struct, page, _ = line.rstrip("\n").split("\t")[:6]
    full = next(n for n in io_type if n.split(" ")[0] == name.split(" ")[0])
    if io_type[full] != "I/O":
        continue
    key = re.match(r"P[A-K]\d{1,2}", full).group(0)
    assert key not in pins, key
    pins[key] = {"structure": struct, "page": int(page)}
assert len(pins) == 114, len(pins)

T13 = "p. 105, Table 13 (rev Y) = p. 209, Table 111 (rev V), general operating conditions, VIN"
io = {
    "pins": dict(sorted(pins.items(), key=lambda kv: (kv[0][:2], int(kv[0][2:])))),
    "pins_at": {"document": "DS12117", "section": "Table 8 (pin/ball definition), the I/O structure column; legend Table 7, p. 63"},
    # A pull is permanent: an operating condition, not Table 10's stress rating (ruled D-A1).
    "families": [
        {"family": "FT", "above_supply_millivolts": 3600, "max_millivolts": 5499,
         "at": {"document": "DS12117", "section": f"{T13} of all I/O but BOOT0 and TT_xx: Min(VDD, VDDA, VDD33USB) + 3.6 V, < 5.5 V (at most 5499 mV); note 2, the structure's own supplies"}},
        {"family": "TT", "above_supply_millivolts": 300, "max_millivolts": None,
         "at": {"document": "DS12117", "section": f"{T13} of TT_xx I/O: VDD + 0.3 V"}},
    ],
    "pulls_off_above_supply_millivolts": {"value": 300, "at": {"document": "DS12117", "section": "p. 105, Table 13 note 3 (= p. 209, Table 111 note 3): above Min(VDD, VDDA, VDD33USB) + 0.3 V the internal pull-up and pull-down must be disabled"}},
}

p = f"{data}/power-parts.json"
d = json.load(open(p))
assert d["schema_version"] == 5
d["schema_version"] = 6
mcu = next(r for r in d["parts"] if r["part"] == "STM32H753ZITx")
assert "io" not in mcu["kind"]["mcu"]
mcu["kind"]["mcu"]["io"] = io
open(p, "w").write(json.dumps(d, indent=2, ensure_ascii=False) + "\n")

p = f"{data}/parts.json"
d = json.load(open(p))
assert d["schema_version"] == 11
d["schema_version"] = 12
row = next(r for r in d["pin_parts"] if r["part"] == "DRV8876PWPR")
row["trip"] = {
    "reference": "VREF", "sense": "IPROPI",
    "page": "p. 13, 7.3.3.2: ITRIP x AIPROPI = VVREF / RIPROPI (Eq. 3), chopping in every IMODE state (Table 6); regulation is off only with IPROPI on GND and VREF above GND",
}
row["bulk_unstated"] = {
    "supply": "VM",
    "page": "p. 3 (VM: sufficient bulk capacitance rated for VM); p. 10, Table 1, CVM2 'Bulk Capacitance, VM-rated', no value; 9.1 p. 31: system level testing sizes it",
}
open(p, "w").write(json.dumps(d, indent=2, ensure_ascii=False) + "\n")
print("io pins", len(pins), "families", [f["family"] for f in io["families"]])
