# Writes the measurement inputs into OUT (argv[1]) from the committed fixtures, changing no
# committed byte:
# - d1.json, d2.json: copies of crates/aethelgard-ir/tests/fixtures/d1-drv8876.json, d2-drv8833.json;
# - d2g.json: D2 with each bridge input a GPIO output that starts low and the board drawn, built
#   exactly as crates/aethelgard-cli/tests/pin_devices_drawn_tier1.rs::d2_drawn builds it;
# - d1-drawn.json: D1 with the board drawn (for verify's coverage only: emit refuses it, CVM2);
# - scenarios for d2g and d1: a baseline, a droop on VMOTOR, a rail_out_of_band on VMOTOR.
import json, os, shutil, sys
REPO = os.path.abspath(os.path.join(os.path.dirname(__file__), "../../.."))
OUT = sys.argv[1]
os.makedirs(os.path.join(OUT, "sc"), exist_ok=True)
fx = lambda n: json.load(open(os.path.join(REPO, "crates/aethelgard-ir/tests/fixtures", n)))
def put(name, v):
    with open(os.path.join(OUT, name), "w") as f:
        json.dump(v, f, indent=2); f.write("\n")
shutil.copy(os.path.join(REPO, "crates/aethelgard-ir/tests/fixtures/d1-drv8876.json"), os.path.join(OUT, "d1.json"))
shutil.copy(os.path.join(REPO, "crates/aethelgard-ir/tests/fixtures/d2-drv8833.json"), os.path.join(OUT, "d2.json"))
d2g = fx("d2-drv8833.json")
d2g["board"].pop("drawn")
inputs = [("A_IN1", "AIN1"), ("A_IN2", "AIN2"), ("B_IN1", "BIN1"), ("B_IN2", "BIN2")]
d2g["motor_outputs"] = [m for m in d2g["motor_outputs"] if m["label"] not in dict(inputs)]
for net, pin in inputs:
    d2g["pin_devices"][0]["signals"].append(
        {"device_pin": pin, "net": net, "pin": None, "direction": "output", "initial": "low"})
put("d2g.json", d2g)
d1d = fx("d1-drv8876.json"); d1d["board"].pop("drawn"); put("d1-drawn.json", d1d)
droop = {"domain": "VMOTOR", "floor_millivolts": 4000, "start_ns": 1000000, "fall_ns": 20000,
         "hold_ns": 600000, "rise_ns": 20000, "source_milliohms": 500, "load_milliamps": 100}
for base in ["d2g", "d1"]:
    for name, sv, exp, ov in [
        ("baseline", 1, "pass", []),
        ("droop-vmotor", 2, "pass", [{"droop": droop}]),
        ("vmotor-out-of-band", 4, "fail", [{"rail_out_of_band": "VMOTOR"}]),
    ]:
        put(f"sc/{base}-{name}.json", {"schema_version": sv, "name": name, "base": f"../{base}.json",
             "expect": exp, "expect_findings": [], "overrides": ov})
print("written", sorted(os.listdir(OUT)), sorted(os.listdir(os.path.join(OUT, "sc"))))
