#!/usr/bin/env python3
"""AET-WO-1211e §1.2: write the rows the three rules read, once.
- power-parts.json (schema 5 -> 6): the MCU row's `io`, each LQFP144 I/O pin's
  structure from DS12117 Rev 10 Table 8 (out/table8-text.tsv, which the visual
  read out/table8-visual.tsv agrees with row for row) keyed by the pin table's
  canonical name, and each structure family's tolerated input from Table 10.
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

T10 = "p. 103, Table 10 (rev Y) = p. 207, Table 108 (rev V), VIN"
NOTES = "notes p. 104 = p. 208"
io = {
    "pins": dict(sorted(pins.items(), key=lambda kv: (kv[0][:2], int(kv[0][2:])))),
    "pins_at": {"document": "DS12117", "section": "Table 8 (pin/ball definition), the I/O structure column; legend Table 7, p. 63"},
    "families": [
        {"family": "FT", "above_supply_millivolts": 4000,
         "max_millivolts": {"value": 6000, "at": {"document": "DS12117", "section": f"{T10} on FT_xxx pins: Min(Min(VDD, VDDA, VDD33USB, VBAT) + 4.0, 6 V); {NOTES} 3 (its structure's supplies), 4 (above 4 V no internal pull), 5"}}},
        {"family": "TT", "above_supply_millivolts": None,
         "max_millivolts": {"value": 4000, "at": {"document": "DS12117", "section": f"{T10} on TT_xx pins: 4.0 V"}}},
    ],
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
