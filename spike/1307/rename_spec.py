#!/usr/bin/env python3
"""rename_spec.py <spec> <seed> <out>: seeded prefix on device references and bus/CS net names (and net_constraints)."""
import json, sys, hashlib
spec, seed, out = sys.argv[1], sys.argv[2], sys.argv[3]
d = json.load(open(spec))
def h(kind, s):
    return hashlib.sha256(f"{seed}/{kind}/{s}".encode()).hexdigest()[:4]
ref = lambda r: f"Q{h('r', r)}_{r}"
net = lambda n: f"{h('n', n)}_{n}"
for k in ("i2c_devices", "spi_devices"):
    for dev in d.get(k, []):
        dev["reference"] = ref(dev["reference"])
        if "cs" in dev and dev["cs"].get("net"):
            dev["cs"]["net"] = net(dev["cs"]["net"])
for k in ("i2c_buses", "spi_buses"):
    for b in d.get(k, []):
        for sig in ("sda", "scl", "sck", "mosi", "miso", "cs"):
            if sig in b and b[sig].get("net"):
                b[sig]["net"] = net(b[sig]["net"])
for c in d.get("net_constraints", []):
    c["net_name"] = net(c["net_name"])
json.dump(d, open(out, "w"), indent=1)
