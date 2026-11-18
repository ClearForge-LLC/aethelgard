"""AET-WO-1314 sweep: ring rule (AET_RING_UM) x bounded-net via cost
(AET_BOUNDED_VIA_COST) on Sentinel-03 and the power fixture (as the CLI
tests take it), ungated. Usage: sweep.py <exe> <repo> <out> [gated]"""
import json, os, subprocess, sys, itertools
exe, repo, out = sys.argv[1:4]
gated = len(sys.argv) > 4
os.makedirs(out, exist_ok=True)
s03 = json.load(open(f"{repo}/crates/aethelgard-cli/tests/fixtures/sentinel-03.json"))
p = json.load(open(f"{repo}/crates/aethelgard-ir/tests/fixtures/sentinel-02-power.json"))
p["power"]["crossings"] = []
p["accept_lifecycle"] = [a for a in p["accept_lifecycle"] if a["part"] != "ICM20948"]
for key in ("spi_devices", "spi_buses", "net_constraints"):
    p[key] = [x for x in p[key] if not (x.get("reference") == "U5" or x.get("id") == "IMU" or str(x.get("net_name", "")).startswith("IMU"))]
def gate(d):
    d = json.loads(json.dumps(d))
    st = d["board"]["stackup"]
    for k, v in [("outer_copper_min_um", 30), ("plane_copper_min_um", 15), ("plating_min_um", 18), ("copper_temperature_max_c", 125)]:
        st[k] = {"value": v, "source": "1314 sweep"}
    d["power"]["drop_limits"] = [{"domain": "VDD_3V3", "max_millivolts": 165, "source": "1314 sweep"}]
    return d
specs = {"s03": s03, "p02": p}
for name, d in specs.items():
    json.dump(gate(d) if gated else d, open(f"{out}/{name}.json", "w"))
rings = json.loads(os.environ.get("RINGS", "[null, 0, 28, 50]"))
costs = json.loads(os.environ.get("COSTS", "[null, 48, 96, 160, 320]"))
boards = json.loads(os.environ.get("BOARDS", '["s03", "p02"]'))
for name, ring, cost in itertools.product(boards, rings, costs):
    env = dict(os.environ)
    env.pop("AET_RING_UM", None); env.pop("AET_BOUNDED_VIA_COST", None)
    if ring is not None: env["AET_RING_UM"] = str(ring)
    if cost is not None: env["AET_BOUNDED_VIA_COST"] = str(cost)
    label = f"{name}-r{ring}-c{cost}" + ("-g" if gated else "")
    r = subprocess.run([sys.executable, "-I", os.path.join(os.path.dirname(__file__), "measure.py"), exe, f"{out}/{name}.json", out, label], capture_output=True, text=True, env=env)
    print(r.stdout.strip() or json.dumps({"label": label, "error": r.stderr[-300:]}), flush=True)
