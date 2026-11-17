#!/usr/bin/env python3
"""rename_perm.py <spec> <seed> <out>: a renaming that keeps every reference's length (AET-WO-1313).

1307's rename_spec.py prefixes references ("U2" -> "Qab12_U2"). That changes each part's reference
text on the silkscreen, and with it the part's keep-out box, so it measures size as well as order.
Here device, connector and test-point references are permuted among references of the same length
and prefix (a seeded permutation), carried to the power input and the MCU's supply source; bus and
chip-select nets get 1307's seeded prefix (a net name is not on the silkscreen), carried to connector
pins, test points and net_constraints."""
import hashlib
import json
import re
import sys

spec, seed, out = sys.argv[1], sys.argv[2], sys.argv[3]
d = json.load(open(spec))


def h(kind, s):
    return hashlib.sha256(f"{seed}/{kind}/{s}".encode()).hexdigest()


def perm(refs):
    groups = {}
    for r in refs:
        groups.setdefault((re.match(r"[A-Z]+", r).group(0), len(r)), []).append(r)
    m = {}
    for g in groups.values():
        for a, b in zip(sorted(g), sorted(g, key=lambda r: h("p", r))):
            m[a] = b
    return m


devs = [x for k in ("i2c_devices", "spi_devices") for x in d.get(k, [])]
rmap = perm([x["reference"] for x in devs])
for x in devs:
    x["reference"] = rmap[x["reference"]]
jmap = perm([c["reference"] for c in d.get("connectors", [])])
for c in d.get("connectors", []):
    c["reference"] = jmap[c["reference"]]
ss = d.get("mcu", {}).get("supply_source")
if ss and ss.get("connector") in jmap:
    ss["connector"] = jmap[ss["connector"]]
tmap = perm([t["reference"] for t in d.get("test_points", [])])
for t in d.get("test_points", []):
    t["reference"] = tmap[t["reference"]]
nmap = {}
net = lambda n: nmap.setdefault(n, f"{h('n', n)[:4]}_{n}")
for k, sigs in (("i2c_buses", ("sda", "scl")), ("spi_buses", ("sck", "mosi", "miso"))):
    for b in d.get(k, []):
        for s in sigs:
            b[s]["net"] = net(b[s]["net"])
for x in d.get("spi_devices", []):
    x["cs"]["net"] = net(x["cs"]["net"])
for c in d.get("net_constraints", []):
    if c["net_name"] in nmap:
        c["net_name"] = nmap[c["net_name"]]
for c in d.get("connectors", []):
    for p in c["pins"]:
        if p["net"] in nmap:
            p["net"] = nmap[p["net"]]
for t in d.get("test_points", []):
    if t["net"] in nmap:
        t["net"] = nmap[t["net"]]
json.dump(d, open(out, "w"), indent=1)
