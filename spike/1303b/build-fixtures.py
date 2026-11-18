"""AET-WO-1303b §1.1: the boards measured, besides the two committed
specifications with a regulator (sentinel-02-power, sentinel-03):
- s024l: the power fixture at four layers, constructed (no 4-layer power
  fixture exists): sentinel-02-power.json with Sentinel-03's
  board.copper_layers, board.layers and board.stackup;
- the "-gnd" variants: GND added to the SUPPLY_MCU class (0.2 mm, 0.7/0.3 vias),
  the router's via-pad remedy (AET-WO-1308) applied to the ground return.
Usage: build-fixtures.py <repo> <out dir>; then `aethelgard emit` each."""
import json, sys
repo, out = sys.argv[1:3]
p = json.load(open(f"{repo}/crates/aethelgard-ir/tests/fixtures/sentinel-02-power.json"))
s3 = json.load(open(f"{repo}/crates/aethelgard-cli/tests/fixtures/sentinel-03.json"))
def gnd(d):
    for c in d["net_classes"]:
        if c["name"] == "SUPPLY_MCU":
            c["nets"].append("GND")
    return d
four = json.loads(json.dumps(p))
for k in ("copper_layers", "layers", "stackup"):
    four["board"][k] = s3["board"][k]
for name, d in [("s024l", four), ("s024l-gnd", gnd(json.loads(json.dumps(four)))),
                ("sentinel-02-power-gnd", gnd(json.loads(json.dumps(p)))),
                ("sentinel-03-gnd", gnd(json.loads(json.dumps(s3))))]:
    json.dump(d, open(f"{out}/{name}.json", "w"), indent=2)
