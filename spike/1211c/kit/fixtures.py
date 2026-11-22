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
         "hold_ns": 600000, "rise_ns": 20000, "source_milliohms": 500, "load_milliamps": 100, "bor_level": 3}
for base in ["d2g", "d1"]:
    for name, sv, exp, ov, fx_ in [
        ("baseline", 1, "pass", [], []),
        ("droop-vmotor", 2, "pass", [{"droop": droop}], []),
        ("vmotor-out-of-band", 4, "fail", [{"rail_out_of_band": "VMOTOR"}], ["spi_identity_mismatch"]),
    ]:
        put(f"sc/{base}-{name}.json", {"schema_version": sv, "name": name, "base": f"../{base}.json",
             "expect": exp, "expect_findings": fx_, "overrides": ov})
# Isolation (added after M3's first run, on the advisor's point): with a detected serial_rx_open on
# CMD and a BOR-crossing droop on the MCU's supply beside the baseline, VMOTOR should be the only
# unmet target. The droop is sentinel-03/droop-bor3.json's, its base changed.
bor = json.load(open(os.path.join(REPO, "scenarios/sentinel-03/droop-bor3.json")))
for base in ["d2g", "d1"]:
    put(f"sc/{base}-rx-open.json", {"schema_version": 3, "name": "rx-open", "base": f"../{base}.json",
         "expect": "fail", "expect_findings": ["serial_echo_missing"], "overrides": [{"serial_rx_open": "CMD"}]})
    put(f"sc/{base}-droop-bor3.json", dict(bor, base=f"../{base}.json"))
# The build's runs (AET-WO-1211c §1.2-1.4), on D1: a held nFAULT, a set IPROPI, a hold on an output.
for name, sv, exp, fx_, ov in [
    ("nfault-held-low", 5, "fail", ["pin_device_input_asserted"], [{"pin_held": {"net": "DRV_NFAULT", "level": "low"}}]),
    ("nfault-held-high", 5, "pass", [], [{"pin_held": {"net": "DRV_NFAULT", "level": "high"}}]),
    ("ipropi-full-scale", 5, "pass", [], [{"analog_value": {"net": "DRV_IPROPI", "millivolts": 2850}}]),
    ("nsleep-held", 5, "fail", ["pin_device_input_asserted"], [{"pin_held": {"net": "DRV_NSLEEP", "level": "high"}}]),
]:
    put(f"sc/d1-{name}.json", {"schema_version": sv, "name": name, "base": "../d1.json",
         "expect": exp, "expect_findings": fx_, "overrides": ov})
put("sc/d2g-nfault-held-low.json", {"schema_version": 5, "name": "nfault-held-low", "base": "../d2g.json",
     "expect": "fail", "expect_findings": ["pin_device_input_asserted"],
     "overrides": [{"pin_held": {"net": "DRV_NFAULT", "level": "low"}}]})
print("written", sorted(os.listdir(OUT)), sorted(os.listdir(os.path.join(OUT, "sc"))))
