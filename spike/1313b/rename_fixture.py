#!/usr/bin/env python3
"""rename_fixture.py <spec> <seed> <out>: rename_spec.py's renaming (1307: a seeded prefix on device
references and bus/CS net names, and net_constraints), carried through to every place the fixture
names the same nets (connector pins and test points), so the renamed specification stays valid; and
connector and test-point references renamed too (beyond 1307's renamer)."""
import hashlib
import json
import subprocess
import sys
import pathlib

spec, seed, out = sys.argv[1], sys.argv[2], sys.argv[3]
subprocess.run([sys.executable, "-I", str(pathlib.Path(__file__).with_name("rename_spec.py")), spec, seed, out],
               check=True)
d = json.load(open(out))
orig = json.load(open(spec))
renamed = {}
for k, sigs in (("i2c_buses", ("sda", "scl")), ("spi_buses", ("sck", "mosi", "miso"))):
    for a, b in zip(orig.get(k, []), d.get(k, [])):
        for s in sigs:
            renamed[a[s]["net"]] = b[s]["net"]
for a, b in zip(orig.get("spi_devices", []), d.get("spi_devices", [])):
    renamed[a["cs"]["net"]] = b["cs"]["net"]
for c in d.get("connectors", []):
    for p in c["pins"]:
        if p["net"] in renamed:
            p["net"] = renamed[p["net"]]
for t in d.get("test_points", []):
    if t["net"] in renamed:
        t["net"] = renamed[t["net"]]
# Beyond 1307's renamer (AET-WO-1313): connector and test-point references, with the same seeded
# prefix, carried to the power input and the MCU's supply source.
ref = lambda r: f"Q{hashlib.sha256(f'{seed}/r/{r}'.encode()).hexdigest()[:4]}_{r}"
jmap = {}
for c in d.get("connectors", []):
    jmap[c["reference"]] = c["reference"] = ref(c["reference"])
if d.get("power") and d["power"].get("input") and d["power"]["input"]["reference"] in jmap:
    d["power"]["input"]["reference"] = jmap[d["power"]["input"]["reference"]]
ss = d.get("mcu", {}).get("supply_source")
if ss and ss.get("connector") in jmap:
    ss["connector"] = jmap[ss["connector"]]
for t in d.get("test_points", []):
    t["reference"] = ref(t["reference"])
json.dump(d, open(out, "w"), indent=1)
