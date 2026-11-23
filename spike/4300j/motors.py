#!/usr/bin/env python3
"""AET-WO-4300j M1: every committed motor output, its pull, and its board's rail.

usage: python3 -I spike/4300j/motors.py   (from the repository root; reads
universe.sh's list on stdin-free terms by running it, and out/baseline.tsv)
writes spike/4300j/out/motors.tsv and prints a summary.
"""
import csv
import json
import subprocess
import sys
from collections import Counter

here = "spike/4300j"
uni = subprocess.run(["bash", f"{here}/universe.sh"], capture_output=True, text=True, check=True).stdout
base = {r["name"]: r for r in csv.DictReader(open(f"{here}/out/baseline.tsv"), delimiter="\t")}
rows = []
for line in uni.splitlines():
    name, path = line.split("\t")
    s = json.load(open(path))
    mo = s.get("motor_outputs") or []
    if not mo:
        continue
    sup = s["mcu"]["supply"]
    dom = next((d for d in s.get("voltage_domains", []) if d["id"] == sup), None)
    for m in mo:
        r = m["pull"]["resistor"]
        rows.append({
            "board": name, "path": path, "label": m["label"], "pin": m["pin"] or "-",
            "protocol": m["protocol"], "pull_ref": r["reference"], "part": r["part"],
            "ohms": r["ohms"], "to": r["to"], "supply": sup,
            "supply_mv": dom["nominal_millivolts"] if dom else "-",
            "supply_tol_permille": dom["tolerance_permille"] if dom else "-",
            "validate_rc": base[name]["validate_rc"], "emit_rc": base[name]["emit_rc"],
        })
out = f"{here}/out/motors.tsv"
with open(out, "w", newline="") as f:
    w = csv.DictWriter(f, fieldnames=list(rows[0]), delimiter="\t", lineterminator="\n")
    w.writeheader()
    w.writerows(rows)
boards = Counter(r["board"] for r in rows)
print(f"motor boards: {len(boards)}; motor outputs: {len(rows)}")
for b, n in sorted(boards.items()):
    r0 = next(r for r in rows if r["board"] == b)
    print(f"  {b}: {n} outputs, validate {r0['validate_rc']}, emit {r0['emit_rc']}, supply {r0['supply']} {r0['supply_mv']} mV +/-{r0['supply_tol_permille']}")
print("pulls:", Counter((r["part"], r["ohms"], r["to"]) for r in rows))
print("rails:", Counter((r["supply"], r["supply_mv"], r["supply_tol_permille"]) for r in rows))
drawn = [r for r in rows if r["emit_rc"] == "0"]
print(f"drawn motor nets (emit 0): {len(drawn)} on {len({r['board'] for r in drawn})} boards")
sys.exit(0)
