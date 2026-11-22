#!/usr/bin/env python3
"""AET-WO-1217 F4 (ruled 12:25Z): write the MCU row's `io.analog_pads` into
power-parts.json (schema 7 -> 8), from DS12117 Rev 10 Table 8's type column as
1211e's table8.py extracted it (spike/1211e/out/table8-text.tsv): every row of
type ANA, by the pin table's canonical name (`PC2_C` is `PC2`).
Usage: python3 -I pads.py <repo>"""
import json, re, sys

repo = sys.argv[1]
path = f"{repo}/crates/aethelgard-ir/data/power-parts.json"
pads = []
for line in open(f"{repo}/spike/1211e/out/table8-text.tsv"):
    name, pos, typ, struct, page, _ = line.rstrip("\n").split("\t")[:6]
    if typ == "ANA":
        pads.append((re.match(r"P[A-K]\d{1,2}", name).group(0), name, int(page)))
assert [p[0] for p in pads] == ["PC2", "PC3"], pads
assert {p[2] for p in pads} == {67}, pads
doc = json.load(open(path))
assert doc["schema_version"] == 7, doc["schema_version"]
doc["schema_version"] = 8
mcu = next(r for r in doc["parts"] if "mcu" in r["kind"])
io = mcu["kind"]["mcu"]["io"]
assert "analog_pads" not in io
io["analog_pads"] = {
    "pins": [p[0] for p in pads],
    "at": {
        "document": "DS12117",
        "section": "p. 67, Table 8 (pin/ball definition): `PC2_C` and `PC3_C` are pin type ANA, and note 4: the Pxy alternate functions reach the Pxy_C pad only through the analog switch (SYSCFG); p. 104, Table 11 (current characteristics): a Px_C pin sinks at most 1 mA",
    },
}
open(path, "w").write(json.dumps(doc, indent=2, ensure_ascii=False) + "\n")
