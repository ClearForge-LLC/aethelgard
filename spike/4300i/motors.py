"""AET-WO-4300i R3: one row per motor output on every board that has motor_outputs.

usage: python3 -I motors.py <repo root> <scratch dir (solve/, emit/ from run.sh)>
       -> out/motors.tsv, out/motors-summary.md
"""
import collections
import glob
import json
import os
import re
import subprocess
import sys

root, W = sys.argv[1], sys.argv[2]
here = os.path.join(root, "spike/4300i")
pp = {p["part"]: p for p in json.load(open(os.path.join(root, "crates/aethelgard-ir/data/power-parts.json")))["parts"]}
uni = [l.split("\t") for l in subprocess.check_output(["bash", os.path.join(here, "universe.sh")], text=True).splitlines()]

HEAD = ["board", "label", "pin", "hand_solved", "solved_pin", "protocol", "idle", "implied_pupd", "structure",
        "connector_ends", "test_point_ends", "bridge_input_ends", "far_class", "drawn"]
rows, boards, emits = [], [], {}
for n, f in uni:
    s = json.load(open(os.path.join(root, f)))
    mo = s.get("motor_outputs") or []
    if not mo:
        continue
    boards.append(n)
    sp = os.path.join(W, "solve", n + ".json")
    t = json.load(open(sp)) if os.path.exists(sp) else None
    pcbs = glob.glob(os.path.join(W, "emit", n, "*.kicad_pcb"))
    pcb = open(pcbs[0]).read() if pcbs else None
    emits[n] = pcb is not None
    io = pp.get(s["mcu"]["part"], {}).get("kind", {}).get("mcu", {}).get("io", {}).get("pins", {})
    for m in mo:
        lab = m["label"]
        idle = (m.get("pwm") or {}).get("idle") if m["protocol"] == "pwm" else None
        idles = "dshot" if m["protocol"].startswith("dshot") else json.dumps(idle, sort_keys=True)
        pupd = "up" if idle == {"level": "high"} else "down"
        solved = t["assignments"][lab]["pin"] if t and lab in t.get("assignments", {}) else "-"
        pin = m.get("pin") or solved
        st = io.get(pin, {}).get("structure") if pin != "-" else None
        cls = "-" if st is None else ("FT" if st.startswith("FT") else "non-FT")
        conn = [f"{c['reference']}.{p['pin']}" for c in s.get("connectors") or [] for p in c["pins"] if p["net"] == lab]
        tps = [tp["reference"] for tp in s.get("test_points") or [] if tp["net"] == lab]
        br = [f"{d.get('reference', d.get('part', '?'))}.{b['id']}.{k}" for d in s.get("pin_devices") or []
              for b in d.get("bridges") or [] for k, v in b["inputs"].items() if v == lab]
        if conn:
            far = "connector"
        elif tps and br:
            far = "test_point+bridge_input"
        elif tps:
            far = "test_point_only"
        elif br:
            far = "bridge_input_only"
        else:
            far = "none"
        drawn = "-" if pcb is None else ("yes" if re.search(r'\(net (?:\d+ )?"/?%s"\)' % re.escape(lab), pcb) else "no")
        rows.append([n, lab, m.get("pin") or "null", "hand" if m.get("pin") else "solved", solved, m["protocol"], idles,
                     pupd, f"{st or '-'} ({cls})", ",".join(conn) or "-", ",".join(tps) or "-", ",".join(br) or "-", far, drawn])
with open(os.path.join(here, "out/motors.tsv"), "w") as o:
    o.write("\t".join(HEAD) + "\n")
    for r in rows:
        o.write("\t".join(r) + "\n")
per = collections.Counter(r[0] for r in rows)
idle = collections.Counter(r[6] for r in rows)
far = collections.Counter(r[12] for r in rows)
nd = [f"{r[0]}:{r[1]}" for r in rows if r[13] == "no"]
noemit = [b for b in boards if not emits[b]]
with open(os.path.join(here, "out/motors-summary.md"), "w") as o:
    o.write(f"# motors summary (R3)\n\n- boards with motor_outputs: {len(boards)}\n- motor nets: {len(rows)}\n")
    o.write("- boards that emit a pcb: %d; boards with no emit: %s\n" % (len(boards) - len(noemit), ", ".join(noemit) or "none"))
    o.write("\n## per board\n" + "".join(f"- {b}: {per[b]}\n" for b in boards))
    o.write("\n## by idle\n" + "".join(f"- {k}: {v}\n" for k, v in sorted(idle.items())))
    o.write("\n## by far-end class\n" + "".join(f"- {k}: {v}\n" for k, v in sorted(far.items())))
    o.write("\n## by hand/solved\n" + "".join(f"- {k}: {v}\n" for k, v in sorted(collections.Counter(r[3] for r in rows).items())))
    o.write("\n## by pin structure\n" + "".join(f"- {k}: {v}\n" for k, v in sorted(collections.Counter(r[8].rsplit(' (',1)[-1].rstrip(')') for r in rows).items())))
    o.write("\n## motor nets not drawn on a board that emits\n" + ("".join(f"- {x}\n" for x in nd) or "- none\n"))
