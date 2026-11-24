# AET-WO-4300k D14 (ruled): sentinel-02-power to 4S, a drop limit on the MCU's supply, the same FET and fuse,
# two input ceramics per buck, and the junction's loss with its current (spec 30).
import json, collections, subprocess
OD = collections.OrderedDict
P = "crates/aethelgard-ir/tests/fixtures/sentinel-02-power.json"
s = json.loads(subprocess.run(["git", "show", "4a8c1056:" + P], capture_output=True, text=True, check=True).stdout,
               object_pairs_hook=OD)
s["schema_version"] = 30
p = s["power"]
p["battery"]["chemistry"] = "LiPo, 3S to 4S, high-voltage cells allowed"
p["battery"]["cells"]["max"] = 4
p["input"]["protection"] = OD([
    ("fuse", OD([("reference", "F1"), ("part", "SF-1206F700-2")])),
    ("fet", OD([("reference", "Q1"), ("part", "DMP6023LSS-13")])),
    ("zener", OD([("reference", "D1"), ("part", "MMSZ5242BT1G")])),
    ("gate_resistor", OD([("reference", "R60"), ("part", "CRCW0603_F_K"), ("ohms", 100000)])),
    ("source", "AET-WO-4300k D14 (ruled): Sentinel-03's protection, the same parts (Q1 ruled (a))"),
])
extra = {"U10": "C60", "U11": "C61"}
loss_at = {"U10": 2000, "U11": 679}
for r in p["regulators"]:
    if r["reference"] not in extra:
        continue
    ext = []
    for e in r["external"]:
        ext.append(e)
        if e["role"] == "cin":
            ext.append(OD([("role", "cin"), ("reference", extra[r["reference"]]), ("part", e["part"])]))
    r["external"] = ext
    j = r["junction"]
    r["junction"] = OD([("loss_milliwatts", j["loss_milliwatts"]), ("loss_at_milliamps", loss_at[r["reference"]])])
p["drop_limits"] = [OD([("domain", "VDD_3V3"), ("max_millivolts", 165),
    ("source", "AET-WO-4300k D14: test fixture, 5 % of the 3.3 V nominal (the supply gate is refused before it on this board, TranslatorPinUnbounded)")])]
open(P, "w").write(json.dumps(s, indent=2, ensure_ascii=False) + "\n")
print("written", P)
