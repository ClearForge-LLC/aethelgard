"""AET-WO-4300i R2: every solved MCU assignment whose net is a connectors[].pins[].net.

usage: python3 -I facing.py <repo root> <scratch dir (solve/ from run.sh)>
       -> out/facing.tsv, out/facing-summary.md
hand/solved comes from the specification (the pin given where the net is declared);
a solved net with no declaration this script maps is `unknown`.
"""
import collections
import json
import os
import subprocess
import sys

root, W = sys.argv[1], sys.argv[2]
here = os.path.join(root, "spike/4300i")
pp = {p["part"]: p for p in json.load(open(os.path.join(root, "crates/aethelgard-ir/data/power-parts.json")))["parts"]}
uni = [l.split("\t") for l in subprocess.check_output(["bash", os.path.join(here, "universe.sh")], text=True).splitlines()]


def declared(s):
    d = {}

    def put(net, pin, where):
        if net:
            d.setdefault(net, []).append((pin, where))
    for b in s.get("i2c_buses", []) or []:
        for k in ("sda", "scl"):
            put(b[k]["net"], b[k].get("pin"), "i2c_buses")
    for b in s.get("spi_buses", []) or []:
        for k in ("sck", "mosi", "miso"):
            put(b[k]["net"], b[k].get("pin"), "spi_buses")
    for x in s.get("spi_devices", []) or []:
        put(x["cs"]["net"], x["cs"].get("pin"), "spi_devices")
    for p in s.get("serial_ports", []) or []:
        for e in ("tx", "rx"):
            if p.get(e):
                put(p[e]["net"], p[e].get("pin"), "serial_ports")
    for m in s.get("motor_outputs", []) or []:
        put(m["label"], m.get("pin"), "motor_outputs")
    for x in s.get("pin_devices", []) or []:
        for g in x.get("signals", []) or []:
            put(g["net"], g.get("pin"), "pin_devices.signals")
        for a in x.get("analog", []) or []:
            put(a["net"], a.get("pin"), "pin_devices.analog")
    return d


HEAD = ["board", "net", "pin", "hand_solved", "structure", "ft", "connector_pins", "signal", "check"]
rows, skipped, nconn = [], [], 0
for n, f in uni:
    sp = os.path.join(W, "solve", n + ".json")
    if not os.path.exists(sp):
        skipped.append(n)
        continue
    s = json.load(open(os.path.join(root, f)))
    t = json.load(open(sp))
    io = pp.get(s["mcu"]["part"], {}).get("kind", {}).get("mcu", {}).get("io", {}).get("pins", {})
    decl = declared(s)
    solved = {}
    for k, v in t.get("assignments", {}).items():
        solved[k] = (v["pin"], f"{v['signal']} AF{v['alternate_function']}")
    for k, v in t.get("gpio", {}).items():
        solved[k] = (v["pin"], f"GPIO {v['direction']}")
    for k, v in t.get("adc_inputs", {}).items():
        solved[k] = (v["pin"], f"{v['peripheral']}_{v['channel']}")
    conn = collections.defaultdict(list)
    for c in s.get("connectors") or []:
        for p in c["pins"]:
            if p["net"]:
                conn[p["net"]].append(f"{c['reference']}.{p['pin']}")
    for net in sorted(conn):
        if net not in solved:
            continue
        pin, sig = solved[net]
        ds = decl.get(net)
        check = "ok"
        if not ds:
            hs = "unknown"
            check = "no declaration mapped"
        elif len(ds) > 1 and len({d[0] for d in ds}) > 1:
            hs, check = "unknown", "declared in %d places: %s" % (len(ds), ds)
        else:
            hs = "hand" if ds[0][0] else "solved"
            if ds[0][0] and ds[0][0] != pin:
                check = f"MISMATCH hand {ds[0][0]} vs solved {pin}"
        st = io.get(pin, {}).get("structure", "unheld")
        rows.append([n, net, pin, hs, st, "FT" if st.startswith("FT") else "non-FT", ",".join(conn[net]), sig, check])
with open(os.path.join(here, "out/facing.tsv"), "w") as o:
    o.write("\t".join(HEAD) + "\n")
    for r in rows:
        o.write("\t".join(r) + "\n")
moves = [r for r in rows if r[3] == "solved" and r[5] == "non-FT"]
refs = [r for r in rows if r[3] == "hand" and r[5] == "non-FT"]
unk = [r for r in rows if r[3] == "unknown"]
bad = [r for r in rows if r[8] != "ok" and r[3] != "unknown"]
boards = sorted({r[0] for r in rows})
with open(os.path.join(here, "out/facing-summary.md"), "w") as o:
    o.write("# facing summary (R2)\n\n")
    o.write(f"- boards solved: {len(uni) - len(skipped)}; boards skipped (no solve output): {', '.join(skipped)}\n")
    o.write(f"- connector-facing assignments: {len(rows)} over {len(boards)} boards\n")
    o.write(f"- by hand/solved: {dict(collections.Counter(r[3] for r in rows))}\n")
    o.write(f"- by FT: {dict(collections.Counter(r[5] for r in rows))}\n")
    o.write(f"- MOVES (solved + non-FT): {len(moves)}\n- REFUSALS (hand + non-FT): {len(refs)}\n- unknown hand/solved: {len(unk)}\n- hand-vs-solved mismatches: {len(bad)}\n")
    for title, lst in (("MOVES", moves), ("REFUSALS", refs), ("UNKNOWN", unk), ("MISMATCH", bad)):
        o.write(f"\n## {title}\n")
        if not lst:
            o.write("- none\n")
        for r in lst:
            o.write(f"- {r[0]}: {r[1]} on {r[2]} ({r[4]}), {r[6]}, {r[7]}; {r[8]}\n")
