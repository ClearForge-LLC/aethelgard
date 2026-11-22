"""AET-WO-4300h: one line per board from walk 1 (out/facing): rows per class,
distinct MCU pins, and the non-FT pins. usage: python3 -I summary.py <spike/4300h dir> -> out/summary.md"""
import os
import sys

here = sys.argv[1]
lines = ["| board | connector rows | test-point rows | other off-board | MCU pins | non-FT pins (end) |", "|---|---|---|---|---|---|"]
tot = {"connector": 0, "test_point": 0, "other": 0, "pins": 0}
for name in sorted(os.listdir(os.path.join(here, "out/facing"))):
    rows = [l.rstrip("\n").split("\t") for l in list(open(os.path.join(here, "out/facing", name)))[1:]]
    rows = [r for r in rows if r[2] != "connector_other"]
    c = sum(r[2] == "connector" for r in rows)
    t = sum(r[2] == "test_point" for r in rows)
    o = sum(r[2] in ("undeclared_offboard", "foreign_clock") for r in rows)
    pins = {r[4] for r in rows}
    nonft = sorted({f"{r[4]} {r[5]} ({r[1]})" for r in rows if not r[5].startswith("FT")})
    tot["connector"] += c; tot["test_point"] += t; tot["other"] += o; tot["pins"] += len(pins)
    lines.append(f"| {name[:-4]} | {c} | {t} | {o} | {len(pins)} | {'; '.join(nonft) or '-'} |")
lines.append(f"| **total** | {tot['connector']} | {tot['test_point']} | {tot['other']} | {tot['pins']} | |")
open(os.path.join(here, "out/summary.md"), "w").write("\n".join(lines) + "\n")
